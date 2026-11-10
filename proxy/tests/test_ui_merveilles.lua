-- Test hors jeu de src/ui_merveilles.lua, avec une file de production qui imite GameCore :
--   lua proxy/tests/test_ui_merveilles.lua [ProductionHelper.lua]
-- On charge comme le proxy (`patch_merveilles`) : ProductionHelper.lua du jeu, puis le bloc. Les
-- commandes partent dans un flux, que chaque machine exécute dans l'ordre ; on vérifie qu'une
-- machine sans le bloc, qui rejoue le même flux, finit avec la même file.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_merveilles.lua");
local HELPER_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Assets\UI\ProductionHelper.lua]];
local okHelper, HELPER = pcall(ReadFile, HELPER_PATH);
if not okHelper then
	print("ProductionHelper.lua introuvable (" .. HELPER_PATH .. "), test sauté");
	return;
end
-- HavokScript accepte des annotations de type (`local x:number`) que Lua 5.4 refuse : on les retire.
for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
	HELPER = HELPER:gsub("%s*:%s*" .. t .. "%f[^%w_(]", "");
end

local OP = { PARAM_INSERT_MODE = "mode", PARAM_QUEUE_SOURCE_LOCATION = "src", PARAM_QUEUE_DESTINATION_LOCATION = "dst",
	PARAM_QUEUE_LOCATION = "at", PARAM_BUILDING_TYPE = "building", VALUE_SWAP = "swap", VALUE_APPEND = "append",
	VALUE_REMOVE_AT = "remove", VALUE_MOVE_TO = "move", BUILD = "build" };
local DIRECTIVES = { CONSTRUCT = 1, TRAIN = 2 };
-- Bâtiments : index -> définition. 10 et 11 sont des merveilles.
local BUILDINGS = {};
for i, t in pairs({ [3] = "BUILDING_MONUMENT", [10] = "BUILDING_PYRAMIDS", [11] = "BUILDING_COLOSSEUM" }) do
	BUILDINGS[i] = { BuildingType = t, Hash = 1000 + i, IsWonder = i >= 10 };
end
local function ByHash(h)
	for i, d in pairs(BUILDINGS) do if d.Hash == h then return i; end end
end

-- GameCore, réduit à ce qu'on a observé (wonder_queue/README.md) : un élément qui arrive en tête est
-- lancé ; une merveille qui porte sa case et dont la case est déjà posée est retirée.
local function NewCore()
	local core = { queue = {}, placed = {}, ops = {} };
	local function Start()
		while #core.queue > 0 do
			local e = core.queue[1];
			if e.b ~= nil and BUILDINGS[e.b].IsWonder and e.coords then
				if core.placed[e.b] then
					table.remove(core.queue, 1); -- 1800ffbf0
				else
					core.placed[e.b] = true;
					return;
				end
			else
				return;
			end
		end
	end
	function core.Add(e)
		core.queue[#core.queue + 1] = e;
		if #core.queue == 1 then Start(); end
	end
	function core.Apply(p)
		core.ops[#core.ops + 1] = p;
		if p.mode == "swap" then
			local a, b = p.src, p.dst;
			if a == b or a >= #core.queue or b >= #core.queue then return; end
			core.queue[a + 1], core.queue[b + 1] = core.queue[b + 1], core.queue[a + 1];
			if a == 0 or b == 0 then Start(); end
		elseif p.mode == "append" then
			core.Add({ b = ByHash(p.building), coords = false });
		elseif p.mode == "remove" then
			table.remove(core.queue, p.at + 1);
			if p.at == 0 then Start(); end
		end
	end
	return core;
end

local function Keys(core)
	local t = {};
	for _, e in ipairs(core.queue) do
		t[#t + 1] = (e.b and BUILDINGS[e.b].BuildingType or ("UNIT_" .. e.u)) .. (e.coords and "@" or "");
	end
	return table.concat(t, ",");
end

local failures = 0;
local function Check(cond, msg)
	if not cond then
		failures = failures + 1;
		print("ECHEC : " .. msg);
	end
end

-- Une machine avec interface : flux réseau, événements, contextes Lua chargés comme par le jeu.
local function NewMachine(nContexts)
	local m = { core = NewCore(), net = {}, printed = {}, handlers = {} };
	local LOCAL, CITY = 0, 7;
	local queueObj = {
		GetSize = function() return #m.core.queue; end,
		GetAt = function(_, i)
			local e = m.core.queue[i + 1];
			if e == nil then return nil; end
			if e.b ~= nil then return { Directive = DIRECTIVES.CONSTRUCT, BuildingType = e.b }; end
			return { Directive = DIRECTIVES.TRAIN, UnitType = e.u };
		end,
	};
	local city = { GetBuildQueue = function() return queueObj; end, GetOwner = function() return LOCAL; end,
		GetID = function() return CITY; end };
	m.city = city;
	function m.Pump()
		while #m.net > 0 do
			local p = table.remove(m.net, 1);
			m.core.Apply(p);
			for _, h in ipairs(m.handlers) do h(LOCAL, CITY, 0, 0); end
		end
	end
	m.contexts = {};
	for c = 1, nContexts do
		local env = setmetatable({}, { __index = _G });
		env.include = function() end;
		env.print = function(s) m.printed[#m.printed + 1] = s; end
		env.UI = { GetHeadSelectedCity = function() return city; end };
		env.CityOperationTypes = OP;
		env.CityProductionDirectives = DIRECTIVES;
		env.GameInfo = { Buildings = BUILDINGS };
		env.Game = { GetLocalPlayer = function() return LOCAL; end };
		env.CityManager = {
			GetCity = function(p, id) if p == LOCAL and id == CITY then return city; end end,
			RequestOperation = function(_, op, t)
				assert(op == "build");
				local p = {};
				for k, v in pairs(t) do p[k] = v; end
				m.net[#m.net + 1] = p;
			end,
		};
		env.Events = { CityProductionQueueChanged = { Add = function(h) m.handlers[#m.handlers + 1] = h; end } };
		local chunk = HELPER .. "\r\n" .. BLOCK;
		local f = assert(load(chunk, "ProductionHelper.lua", "t", env));
		f();
		m.contexts[c] = env;
	end
	return m;
end

-- Rejoue le flux d'une machine sur une machine sans le bloc, à partir de la même file de départ.
local function Replay(m, start)
	local core = NewCore();
	for _, e in ipairs(start) do core.Add({ b = e.b, u = e.u, coords = e.coords }); end
	for _, p in ipairs(m.core.ops) do core.Apply(p); end
	return Keys(core);
end

local function Setup(m, start)
	for _, e in ipairs(start) do m.core.Add({ b = e.b, u = e.u, coords = e.coords }); end
end

-- 1. File A (merveille posée), unité : échange puis échange inverse.
do
	local start = { { b = 10, coords = true }, { u = 1 } };
	local m = NewMachine(2);
	Setup(m, start);
	m.contexts[1].SwapQueueItem(0, 1); m.Pump();
	Check(Keys(m.core) == "UNIT_1,BUILDING_PYRAMIDS@", "1 : après le premier échange, " .. Keys(m.core));
	m.contexts[1].SwapQueueItem(1, 0); m.Pump();
	Check(Keys(m.core) == "BUILDING_PYRAMIDS,UNIT_1", "1 : merveille remise en tête, " .. Keys(m.core));
	Check(#m.core.ops == 4, "1 : 2 échanges, 1 rajout, 1 échange, et non " .. #m.core.ops);
	Check(Replay(m, start) == Keys(m.core), "1 : machine sans proxy, " .. Replay(m, start));
	Check(m.printed[1] == "Merveilles : BUILDING_PYRAMIDS retiree par l'echange, remise en tete", "1 : journal");
	-- Un nouvel aller-retour : la merveille rajoutée n'a plus de case, rien n'est retiré ni renvoyé.
	m.contexts[1].SwapQueueItem(0, 1); m.Pump();
	m.contexts[1].SwapQueueItem(1, 0); m.Pump();
	Check(Keys(m.core) == "BUILDING_PYRAMIDS,UNIT_1" and #m.core.ops == 6, "1 : second aller-retour, " .. Keys(m.core));
end

-- 2. File plus longue, merveille ramenée de l'index 2 : l'ordre des autres est gardé.
do
	local start = { { b = 10, coords = true }, { b = 3 }, { u = 1 }, { u = 2 } };
	local m = NewMachine(1);
	Setup(m, start);
	m.contexts[1].SwapQueueItem(0, 2); m.Pump();
	Check(Keys(m.core) == "UNIT_1,BUILDING_MONUMENT,BUILDING_PYRAMIDS@,UNIT_2", "2 : premier échange, " .. Keys(m.core));
	m.contexts[1].SwapQueueItem(2, 0); m.Pump();
	Check(Keys(m.core) == "BUILDING_PYRAMIDS,BUILDING_MONUMENT,UNIT_1,UNIT_2", "2 : ordre rétabli, " .. Keys(m.core));
	Check(Replay(m, start) == Keys(m.core), "2 : machine sans proxy, " .. Replay(m, start));
end

-- 3. Échange sans merveille vers la tête, ou merveille jamais posée : aucune commande en plus.
do
	local start = { { u = 1 }, { b = 3 }, { b = 11, coords = true } };
	local m = NewMachine(1);
	Setup(m, start);
	m.contexts[1].SwapQueueItem(0, 1); m.Pump();
	m.contexts[1].SwapQueueItem(2, 0); m.Pump(); -- merveille 11 posée à son arrivée en tête
	Check(Keys(m.core) == "BUILDING_COLOSSEUM@,UNIT_1,BUILDING_MONUMENT", "3 : " .. Keys(m.core));
	Check(#m.core.ops == 2, "3 : aucune commande en plus, " .. #m.core.ops);
end

-- 4. Événement en avance (file pas encore échangée), puis événement d'un autre joueur : rien.
do
	local start = { { b = 10, coords = true }, { u = 1 } };
	local m = NewMachine(1);
	Setup(m, start);
	m.contexts[1].SwapQueueItem(0, 1); m.Pump();
	m.contexts[1].SwapQueueItem(1, 0);
	for _, h in ipairs(m.handlers) do h(0, 7, 0, 0); h(1, 7, 0, 0); end
	Check(#m.net == 1, "4 : rien envoyé avant l'échange, " .. #m.net);
	m.Pump();
	Check(Keys(m.core) == "BUILDING_PYRAMIDS,UNIT_1", "4 : " .. Keys(m.core));
end

-- 5. Retrait volontaire de la merveille : pas de rajout.
do
	local start = { { u = 1 }, { b = 10, coords = true } };
	local m = NewMachine(1);
	Setup(m, start);
	m.core.placed[10] = true;
	m.contexts[1].RemoveQueueItem(1); m.Pump();
	Check(Keys(m.core) == "UNIT_1" and #m.core.ops == 1, "5 : " .. Keys(m.core));
end

-- 6. Le bloc exécuté deux fois dans le même contexte n'enveloppe qu'une fois.
do
	local m = NewMachine(1);
	local env = m.contexts[1];
	local wrapped = env.SwapQueueItem;
	assert(load(BLOCK, "bloc", "t", env))();
	Check(env.SwapQueueItem == wrapped and #m.handlers == 1, "6 : bloc rechargé");
end

if failures == 0 then
	print("ui_merveilles : tous les tests passent");
else
	print("ui_merveilles : " .. failures .. " échec(s)");
	os.exit(1);
end
