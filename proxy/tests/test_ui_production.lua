-- Test hors jeu de src/ui_production.lua :
--   lua proxy/tests/test_ui_production.lua [ProductionPanel.lua]
-- On charge comme le proxy (`patch_production_panel`) : ProductionPanel.lua du jeu, puis le bloc,
-- dans un environnement qui imite l'interface. Les listes et la file du panneau sont remplacées
-- par des fonctions vides : seule compte la visibilité de la production en cours.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_production.lua");
local PANEL_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Assets\UI\Panels\ProductionPanel.lua]];
local okPanel, PANEL = pcall(ReadFile, PANEL_PATH);
if not okPanel then
	print("ProductionPanel.lua introuvable (" .. PANEL_PATH .. "), test sauté");
	return;
end
-- HavokScript accepte des annotations de type (`local x:number`) que Lua 5.4 refuse : on les retire.
for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
	PANEL = PANEL:gsub("%s*:%s*" .. t .. "%f[^%w_(]", "");
end

local LOCAL, A, B = 0, 1, 2;

-- Tout ce que le panneau appelle sans qu'on s'y intéresse : chaque champ et chaque appel rendent
-- le même objet, sauf les index numériques (ipairs s'arrête).
local Any;
Any = setmetatable({}, {
	__index = function(_, k) if type(k) == "number" then return nil; end return Any; end,
	__call = function() return Any; end,
});

-- Un contrôle : garde sa visibilité, rend 0 pour les tailles.
local function NewControl()
	local c = { hidden = false };
	return setmetatable(c, { __index = function(_, k)
		if k == "SetHide" then return function(self, h) self.hidden = h; end; end
		if k == "IsHidden" then return function(self) return self.hidden; end; end
		if k == "IsStopped" then return function() return true; end; end
		if k == "GetTextControl" then return function() return NewControl(); end; end
		if type(k) == "string" and k:match("^Get") then return function() return 0; end; end
		return function() return Any; end;
	end });
end

-- Une machine : le panneau chargé, deux villes dont on règle la production.
local function NewPanel(withBlock)
	local m = { selected = A, current = { [A] = 0, [B] = 0 }, handlers = {} };
	local function City(id)
		return {
			GetID = function() return id; end,
			GetOwner = function() return LOCAL; end,
			GetCityAI = function() return { GetBuildRecommendations = function() return {}; end }; end,
		};
	end
	local cities = { [A] = City(A), [B] = City(B) };

	local env = setmetatable({}, { __index = function(_, k) return _G[k] ~= nil and _G[k] or Any; end });
	env.include = function() end;
	env.IsTutorialRunning = function() return false; end;
	env.Controls = setmetatable({}, { __index = function(t, k) local c = NewControl(); rawset(t, k, c); return c; end });
	env.ContextPtr = setmetatable({ IsHidden = function() return false; end }, { __index = Any });
	env.Locale = { Lookup = function(s) return s; end, ToUpper = function(s) return s; end };
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.Players = { [LOCAL] = {} };
	env.UI = setmetatable({ GetHeadSelectedCity = function() return cities[m.selected]; end }, { __index = Any });
	local function Event(name)
		return { Add = function(h) m.handlers[name] = h; end, Remove = function() end };
	end
	env.Events = setmetatable({}, { __index = function(_, k) return Event(k); end });
	-- ProductionHelper.lua : vrai s'il y a une production (en cours ou terminée) à montrer.
	env.RefreshCurrentProduction = function(_, _, cityID) return m.current[cityID] ~= 0; end;

	local chunk = PANEL;
	if withBlock then chunk = chunk .. "\r\n" .. BLOCK; end
	local f = assert(load(chunk, "ProductionPanel.lua", "t", env));
	f();
	-- Les listes et la file ne servent pas ici ; on garde PopulateCurrentProduction, qui met à jour
	-- m_hasProductionToShow.
	local PopulateCurrentProduction = env.PopulateCurrentProduction;
	env.PopulateList = function(data, listMode)
		if listMode == 1 or listMode == 4 then PopulateCurrentProduction(data); end
	end;
	env.RefreshQueue = function() end;
	env.GetData = function()
		local c = cities[m.selected];
		return { City = c, Owner = LOCAL };
	end;

	function m.Shown()
		return not env.Controls.CurrentProductionContainer.hidden;
	end
	function m.NoProductionShown()
		return not env.Controls.NoProductionContainer.hidden;
	end
	-- Ce que fait le jeu : la file de la ville sélectionnée change, ou une autre ville est choisie.
	function m.SetProduction(hash)
		m.current[m.selected] = hash;
		m.handlers.CityProductionQueueChanged(LOCAL, m.selected, 0, 0);
	end
	function m.Select(id)
		m.selected = id;
		m.handlers.CitySelectionChanged(LOCAL, id, 0, 0, 0, true, true);
	end
	m.env = env;
	return m;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Le déroulé signalé, file ouverte : production choisie puis retirée dans A, puis ville B.
local function QueueScenario(withBlock)
	local m = NewPanel(withBlock);
	m.current[B] = 42;
	m.env.Refresh();
	m.env.OnTabChangeQueue();
	m.SetProduction(7);
	local afterAdd = m.Shown();
	m.SetProduction(0);
	local afterRemove = not m.Shown() and m.NoProductionShown();
	m.Select(B);
	return afterAdd, afterRemove, m.Shown() and not m.NoProductionShown(), m;
end

do
	local added, removed, shownB = QueueScenario(false);
	Check("sans le bloc : le défaut se reproduit (B reste cachée)", added and removed and not shownB);
end
do
	local added, removed, shownB, m = QueueScenario(true);
	Check("file : production de A montrée puis cachée", added and removed);
	Check("file : production de B montrée en passant sur B", shownB);
	m.Select(A);
	Check("file : retour sur A, rien à montrer", not m.Shown() and m.NoProductionShown());
end

-- Onglet production : même chose, sans le message « aucune production ».
do
	local m = NewPanel(true);
	m.current[B] = 42;
	m.env.Refresh();
	m.env.OnTabChangeProduction();
	Check("production : A sans production, cachée", not m.Shown());
	m.Select(B);
	Check("production : B montrée", m.Shown() and not m.NoProductionShown());
	m.Select(A);
	Check("production : retour sur A, cachée", not m.Shown());
end

-- Onglets d'achat et gestionnaire : la production en cours reste cachée.
for _, tab in ipairs({ "OnTabChangePurchase", "OnTabChangePurchaseFaith", "OnTabChangeManager" }) do
	local m = NewPanel(true);
	m.current[B] = 42;
	m.env.Refresh();
	m.env[tab]();
	m.Select(B);
	Check(tab .. " : rien de montré", not m.Shown() and not m.NoProductionShown());
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
