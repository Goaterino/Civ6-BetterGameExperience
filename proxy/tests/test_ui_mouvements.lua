-- Test hors jeu de src/ui_mouvements.lua (zone de déplacement sur un quartier ennemi, et sa mesure) :
--   lua proxy/tests/test_ui_mouvements.lua [SelectedUnit.lua]
-- On charge comme le proxy (`patch_selected_unit`) : SelectedUnit.lua du jeu, puis le bloc, dans un
-- environnement qui imite l'interface. Carte hexagonale en coordonnées axiales ; GetMoveToPathEx est
-- un vrai plus court chemin (tours, PM restants, zones de contrôle, embarquement) sur cette carte.
-- La zone du bloc est comparée à celle de l'ancien algorithme (un chemin par case du disque, anneau
-- par anneau), recopié plus bas, avec un A* qui dit vrai. Une case « mensongère » imite une case non
-- révélée : l'A* du jeu la croit franchissable, le bloc lit sa vraie nature.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_mouvements.lua");
local FILE_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Assets\UI\WorldView\SelectedUnit.lua]];
local okFile, FILE = pcall(ReadFile, FILE_PATH);
if not okFile then
	print("SelectedUnit.lua introuvable (" .. FILE_PATH .. "), test sauté");
	return;
end
-- HavokScript accepte des annotations de type (`local x:number`) que Lua 5.4 refuse : on les retire.
local function Strip(s)
	for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
		s = s:gsub("%s*:%s*" .. t .. "%f[^%w_(]", "");
	end
	return s;
end

local LOCAL, ENEMY = 0, 1;
local W, H = 64, 48;
local UX, UY = 20, 20;
-- Directions dans l'ordre du jeu imité : est, nord-est, nord-ouest, ouest, sud-ouest, sud-est.
local DIRS = { { 1, 0 }, { 1, -1 }, { 0, -1 }, { -1, 0 }, { -1, 1 }, { 0, 1 } };
local D = { DIRECTION_EAST = 0, DIRECTION_NORTHEAST = 1, DIRECTION_NORTHWEST = 2, DIRECTION_WEST = 3,
	DIRECTION_SOUTHWEST = 4, DIRECTION_SOUTHEAST = 5, NUM_DIRECTION_TYPES = 6 };
local TECH_SHIPBUILDING, TECH_CARTOGRAPHY = 1, 2;
local OCEAN = 5;

local Any;
Any = setmetatable({}, {
	__index = function(_, k) if type(k) == "number" then return nil; end return Any; end,
	__call = function() return Any; end,
});

local function Dist(x1, y1, x2, y2)
	local dq, dr = x1 - x2, y1 - y2;
	return (math.abs(dq) + math.abs(dr) + math.abs(dq + dr)) / 2;
end

-- Une table de GameInfo : indexée par clé, appelable pour parcourir les lignes.
local function Table(rows, key)
	local t = {};
	for i, r in ipairs(rows) do
		if key then t[r[key]] = r; end
		t[r.Index or (i - 1)] = t[r.Index or (i - 1)] or r;
	end
	return setmetatable(t, { __call = function()
		local i = 0;
		return function() i = i + 1; return rows[i]; end;
	end });
end

-- Cartes : coût d'entrée, eau (océan), impassable, tunnel, route, unité amie (traversable, on ne peut
-- pas s'y arrêter), unité ennemie (zone de contrôle), quartier ennemi, rivières (bords portés par
-- la case : est = wRiver, sud-est = nwRiver, sud-ouest = neRiver), mensonge (case non révélée que
-- l'A* du jeu croit franchissable).
local function NewMap(kind)
	local c = {};
	for y = 0, H - 1 do
		for x = 0, W - 1 do
			c[y * W + x] = { cost = 1, revealed = true };
		end
	end
	local function At(x, y) return c[y * W + x]; end
	if kind == "route" or kind == "rail" then
		for x = 0, W - 1 do At(x, UY).cost = kind == "route" and 0.5 or 0.25; At(x, UY).route = 1; end
	elseif kind == "cote" or kind == "ocean" or kind == "brouillard_eau" then
		for y = 0, H - 1 do for x = 0, W - 1 do
			local d = x + y / 2 - (UX + UY / 2);
			if d > 2 then At(x, y).water = true; end
			if kind == "ocean" and d > 3 then At(x, y).ocean = true; end
		end end
		if kind == "brouillard_eau" then
			-- Côte non révélée : l'A* du jeu la croit franchissable.
			for y = 0, H - 1 do for x = 0, W - 1 do if At(x, y).water then At(x, y).revealed = false; At(x, y).lie = true; end end end
		end
	elseif kind == "montagnes" or kind == "brouillard" or kind == "couloir" then
		for y = 0, H - 1 do At(UX + 2, y).impassable = true; end
		At(UX + 2, UY - 1).improvement = 7;		-- tunnel
		for x = UX - 3, UX + 1 do At(x, UY - 2).cost = 2; end
		if kind == "brouillard" then
			for y = 0, H - 1 do for x = 0, W - 1 do if x > UX then At(x, y).revealed = false; end end end
		end
		if kind == "couloir" then
			for x = UX - 6, UX + 6 do
				if x ~= UX + 2 then At(x, UY - 1).impassable = true; At(x, UY + 1).impassable = true; end
			end
			for y = 0, H - 1 do At(UX + 2, y).impassable = (y ~= UY); end
			At(UX + 2, UY).impassable = false;
			At(UX - 1, UY).friend = true;
			At(UX + 1, UY).friend = true;
			for x = UX - 6, UX + 6 do At(x, UY).cost = 0.5; end
		end
	elseif kind == "zoc" or kind == "zoc_riviere" then
		At(UX + 3, UY).enemy = true;			-- guerrier ennemi à 3 cases à l'est
		At(UX - 3, UY + 1).enemy = true;		-- un autre au sud-ouest
		if kind == "zoc_riviere" then
			-- Rivière à l'ouest du guerrier de l'est : bords est des cases (UX+2, *) .
			for y = 0, H - 1 do At(UX + 2, y).wRiver = true; end
		end
	end
	return c;
end

local function Neighbor(i, d)
	local x, y = i % W, math.floor(i / W);
	local nx, ny = x + DIRS[d + 1][1], y + DIRS[d + 1][2];
	if nx < 0 or nx >= W or ny < 0 or ny >= H then return nil; end
	return ny * W + nx;
end

-- Rivière entre la case i et sa voisine j (direction d depuis i), mêmes conventions que le jeu.
local function RiverBetween(cells, i, d, j)
	if d == D.DIRECTION_EAST then return cells[i].wRiver == true; end
	if d == D.DIRECTION_WEST then return cells[j].wRiver == true; end
	if d == D.DIRECTION_SOUTHEAST then return cells[i].nwRiver == true; end
	if d == D.DIRECTION_NORTHWEST then return cells[j].nwRiver == true; end
	if d == D.DIRECTION_SOUTHWEST then return cells[i].neRiver == true; end
	if d == D.DIRECTION_NORTHEAST then return cells[j].neRiver == true; end
	return false;
end

-- Zone de contrôle sur la case i en y entrant depuis origin (GameCore 180378a70).
local function ZocAt(cells, o, i, origin)
	for d = 0, 5 do
		local j = Neighbor(i, d);
		if j ~= nil and j ~= origin then
			local cj = cells[j];
			if cj.district and cj.district.complete and cj.district.owner == ENEMY then return true; end
			if cj.enemy and not RiverBetween(cells, i, d, j) then return true; end
		end
	end
	return false;
end

-- Plus court chemin (tour, -PM restants) ; turns[i] = tour d'arrivée sur plots[i]. Entrer sous
-- zone de contrôle met les PM à 0, sauf o.ignoreZoc. m.truth : les cases mensongères disent vrai.
local function NewPathfinder(cells, o, stats, m)
	local function Enterable(cell)
		if cell == nil or cell.enemy then return false; end
		if cell.impassable and not cell.improvement and not cell.route then return false; end
		if cell.water then
			if cell.lie and not m.truth then return true; end
			return o.embark == true and (not cell.ocean or o.ocean == true);
		end
		return true;
	end
	return function(iTarget)
		stats.paths = stats.paths + 1;
		local target = cells[iTarget];
		local M = o.moves;
		local start = UY * W + UX;
		local best = { [start] = { t = 1, r = o.moves } };
		local parent = {};
		local open = { start };
		local closed = {};
		local function Better(a, b) return a.t < b.t or (a.t == b.t and a.r > b.r); end
		while #open > 0 do
			local bi = 1;
			for i = 2, #open do if Better(best[open[i]], best[open[bi]]) then bi = i; end end
			local i = table.remove(open, bi);
			if not closed[i] then
				closed[i] = true;
				stats.nodes = stats.nodes + 1;
				if i == iTarget then break; end
				local s = best[i];
				for d = 0, 5 do
					local j = Neighbor(i, d);
					if j ~= nil then
						local cell = cells[j];
						if Enterable(cell) and not closed[j] then
							local cost = (cell.water and not (cell.lie and not m.truth)) and 99 or cell.cost;	-- embarquer : tous les PM
							local t, r = s.t, s.r;
							if r <= 0 then t, r = t + 1, M; end
							if r >= cost then r = r - cost;
							elseif r == M then r = 0;
							else t, r = t + 1, M; r = (r >= cost) and r - cost or 0; end
							if not o.ignoreZoc and ZocAt(cells, o, j, i) then r = 0; end
							local n = { t = t, r = r };
							if best[j] == nil or Better(n, best[j]) then
								best[j] = n; parent[j] = i; table.insert(open, j);
							end
						end
					end
				end
			end
		end
		if not closed[iTarget] or target.friend then return { plots = {}, turns = {} }; end
		local plots, turns = {}, {};
		local i = iTarget;
		while i ~= nil do
			table.insert(plots, 1, i);
			table.insert(turns, 1, best[i].t);
			i = parent[i];
		end
		return { plots = plots, turns = turns };
	end;
end

-- Une machine : SelectedUnit.lua + bloc, une unité sur un quartier. o : moves, owner (du quartier),
-- complete, war, map, embark, ocean (techs du joueur), ignoreZoc, slowMs (durée de chaque chemin).
local function NewMachine(o)
	local m = { handlers = {}, prints = {}, layers = {}, layerExtras = {}, stats = { paths = 0, nodes = 0 }, now = 0, turn = 5 };
	local cells = NewMap(o.map or "plaine");
	m.cells = cells;
	cells[UY * W + UX].district = { complete = o.complete or false, owner = o.owner or ENEMY, start = true };
	local plots = {};
	local function PlotByIndex(i)
		if i == nil or cells[i] == nil then return nil; end
		if plots[i] == nil then
			local x, y = i % W, math.floor(i / W);
			local cell = cells[i];
			plots[i] = {
				GetX = function() return x; end, GetY = function() return y; end, GetIndex = function() return i; end,
				IsImpassable = function() return cell.impassable == true; end,
				GetImprovementType = function() return cell.improvement or -1; end,
				GetRouteType = function() return cell.route or -1; end,
				IsWater = function() return cell.water == true; end,
				GetTerrainType = function() return cell.ocean and OCEAN or (cell.water and 4 or 0); end,
				IsWOfRiver = function() return cell.wRiver == true; end,
				IsNWOfRiver = function() return cell.nwRiver == true; end,
				IsNEOfRiver = function() return cell.neRiver == true; end,
			};
		end
		return plots[i];
	end
	local function Plot(x, y)
		if x < 0 or x >= W or y < 0 or y >= H then return nil; end
		return PlotByIndex(y * W + x);
	end
	local unit = {
		GetID = function() return 65536; end,
		GetOwner = function() return LOCAL; end,
		GetX = function() return UX; end,
		GetY = function() return UY; end,
		GetPlotId = function() return UY * W + UX; end,
		GetUnitType = function() return 1; end,
		GetMovesRemaining = function() return o.moves; end,
		HasMovedIntoZOC = function() return false; end,
		GetAttacksRemaining = function() return 1; end,
		GetReligiousStrength = function() return 0; end,
		GetArchaeology = function() return nil; end,
		GetGreatPerson = function() return nil; end,
		IsEmbarked = function() return false; end,
	};
	m.unit = unit;
	local function District(cell)
		return {
			GetType = function() return 7; end,
			GetOwner = function() return cell.district.owner; end,
			IsComplete = function() return cell.district.complete; end,
			IsPillaged = function() return false; end,
		};
	end
	local enemyUnit = {
		GetOwner = function() return ENEMY; end, GetUnitType = function() return 2; end,
		IsEmbarked = function() return false; end, GetDamage = function() return 0; end, GetMaxDamage = function() return 100; end,
	};
	local friendUnit = {
		GetOwner = function() return LOCAL; end, GetUnitType = function() return 1; end,
		IsEmbarked = function() return false; end, GetDamage = function() return 0; end, GetMaxDamage = function() return 100; end,
	};
	local techs = { [TECH_SHIPBUILDING] = o.embark == true, [TECH_CARTOGRAPHY] = o.ocean == true };
	local function Player(id)
		return {
			GetDiplomacy = function() return { IsAtWarWith = function(_, other) return o.war ~= false and other ~= id; end }; end,
			GetUnits = function() return { FindID = function(_, uid) return uid == 65536 and unit or nil; end }; end,
			GetCities = function() return { GetCount = function() return 1; end }; end,
			GetTechs = function() return { HasTech = function(_, i) return techs[i] == true; end }; end,
			GetCulture = function() return { HasCivic = function() return false; end }; end,
		};
	end
	local Path = NewPathfinder(cells, o, m.stats, m);

	local env = setmetatable({}, { __index = function(_, k)
		if _G[k] ~= nil then return _G[k]; end
		if k == "Civ6RolloverProxy_Clock" then return nil; end		-- horloge retirée : os.clock
		return Any;
	end });
	local t = {};
	for k, v in pairs(table) do t[k] = v; end
	t.count = function(x) local n = 0; for _ in pairs(x) do n = n + 1; end return n; end;
	env.table = t;
	env.print = function(s) table.insert(m.prints, s); end;
	-- Horloge du proxy simulée : avance de slowMs à chaque chemin.
	env.Civ6RolloverProxy_Clock = function() return m.now; end;
	env.include = function() end;
	env.Game = {
		GetLocalPlayer = function() return LOCAL; end,
		GetCurrentGameTurn = function() return m.turn; end,
		GetGreatPeople = function() return { GetPastTimeline = function() return {}; end }; end,
	};
	env.Players = { [LOCAL] = Player(LOCAL), [ENEMY] = Player(ENEMY) };
	env.PlayerConfigurations = { [LOCAL] = { GetCivilizationTypeName = function() return "CIVILIZATION_X"; end, GetLeaderTypeName = function() return "LEADER_X"; end } };
	env.PlayersVisibility = { [LOCAL] = {
		IsUnitVisible = function() return true; end,
		IsVisible = function(_, x, y) return cells[y * W + x].revealed ~= false; end,
	} };
	env.GameInfo = {
		Units = { [1] = { UnitType = "UNIT_SPEARMAN", Domain = "DOMAIN_LAND", ZoneOfControl = true, IgnoreMoves = false, ParkCharges = 0, FoundCity = false },
			[2] = { UnitType = "UNIT_WARRIOR", Domain = "DOMAIN_LAND", ZoneOfControl = true } },
		Districts = { [7] = { DistrictType = "DISTRICT_ENCAMPMENT", HitPoints = 100 } },
		Technologies = Table({
			{ Index = 0, TechnologyType = "TECH_SAILING", EmbarkUnitType = "UNIT_BUILDER", EmbarkAll = false },
			{ Index = TECH_SHIPBUILDING, TechnologyType = "TECH_SHIPBUILDING", EmbarkAll = true },
			{ Index = TECH_CARTOGRAPHY, TechnologyType = "TECH_CARTOGRAPHY", EmbarkAll = false },
		}, "TechnologyType"),
		Civics = Table({}, "CivicType"),
		TechnologyModifiers = Table({ { TechnologyType = "TECH_CARTOGRAPHY", ModifierId = "CARTOGRAPHY_GRANT_OCEAN_NAVIGATION" } }),
		CivicModifiers = Table({}),
		Modifiers = Table({ { ModifierId = "CARTOGRAPHY_GRANT_OCEAN_NAVIGATION", ModifierType = "MODIFIER_PLAYER_UNITS_ADJUST_VALID_TERRAIN" },
			{ ModifierId = "TRAIT_X_EMBARK", ModifierType = "MODIFIER_PLAYER_UNITS_GRANT_ABILITY" } }, "ModifierId"),
		GlobalParameters = Table({ { Name = "UNLOCK_EMBARKATION", Value = "false" } }, "Name"),
		CivilizationTraits = Table({ { CivilizationType = "CIVILIZATION_X", TraitType = "TRAIT_X" } }),
		LeaderTraits = Table({}),
		TraitModifiers = Table(o.traitEmbark and { { TraitType = "TRAIT_X", ModifierId = "TRAIT_X_EMBARK" } } or {}),
		Terrains = Table({ { Index = OCEAN, TerrainType = "TERRAIN_OCEAN" } }, "TerrainType"),
		GreatPersonIndividuals = Table({}),
		GreatPersonIndividualActionModifiers = Table({}),
		GreatPersonIndividualBirthModifiers = Table({}),
	};
	env.UnitOperationResults = { PLOTS = 1, MODIFIERS = 2, MODIFIER_IS_TARGET = 3 };
	env.UnitOperationTypes = { RANGE_ATTACK = 1 };
	env.DirectionTypes = D;
	env.MapLayers = { ANY = 0 };
	env.Units = { GetUnitsInPlotLayerID = function(x, y)
		local c = cells[y * W + x];
		if c == nil then return {}; end
		if c.enemy then return { enemyUnit }; end
		if c.friend then return { friendUnit }; end
		return {};
	end };
	env.UILens = setmetatable({
		CreateLensLayerHash = function(name) return name; end,
		ClearLayerHexes = function(layer) m.layers[layer] = nil; m.layerExtras[layer] = nil; end,
		SetLayerHexesArea = function(layer, _, plotsList, extras) m.layers[layer] = plotsList; m.layerExtras[layer] = extras; end,
		IsLayerOn = function() return false; end,
		IsLensActive = function() return false; end,
	}, { __index = function() return function() end; end });
	env.UI = setmetatable({
		IsGameCoreBusy = function() return false; end,
		GetInterfaceMode = function() return 0; end,
		GetHeadSelectedUnit = function() return unit; end,
	}, { __index = function() return function() end; end });
	env.InterfaceModeTypes = { CINEMATIC = 99, VIEW_MODAL_LENS = 98, CITY_RANGE_ATTACK = 97, SELECTION = 0 };
	env.ContextPtr = setmetatable({
		SetShutdown = function(_, f) m.shutdown = f; end,
	}, { __index = function() return function() end; end });
	local function Event(name)
		return {
			Add = function(h) m.handlers[name] = m.handlers[name] or {}; table.insert(m.handlers[name], h); end,
			Remove = function(h)
				for i, x in ipairs(m.handlers[name] or {}) do if x == h then table.remove(m.handlers[name], i); return; end end
			end,
		};
	end
	local events = {};
	env.Events = setmetatable({}, { __index = function(_, k) events[k] = events[k] or Event(k); return events[k]; end });
	env.LuaEvents = env.Events;
	env.Map = {
		GetPlot = Plot,
		GetPlotByIndex = PlotByIndex,
		GetAdjacentPlot = function(x, y, d) local v = DIRS[d + 1]; return v and Plot(x + v[1], y + v[2]); end,
		GetPlotXYWithRangeCheck = function(x, y, dx, dy, r)
			if Dist(x, y, x + dx, y + dy) <= r then return Plot(x + dx, y + dy); end
			return nil;
		end,
		GetPlotDistance = Dist,
	};
	env.CityManager = {
		GetDistrictAt = function(p)
			if o.district == false then return nil; end
			local c = cells[p:GetIndex()];
			return c and c.district and District(c) or nil;
		end,
	};
	env.UnitManager = {
		GetReachableTargets = function() return {}; end,
		GetReachableMovement = function() return nil; end,		-- GameCore : rien sur un quartier ennemi
		GetReachableZonesOfControl = function() return nil; end,	-- même parcours : rien non plus
		GetOperationTargets = function() return {}; end,
		GetMoveToPathEx = function(_, iPlot)
			m.now = m.now + (o.slowMs or 0) / 1000;
			return Path(iPlot);
		end,
	};

	local mesure = (o.mesure == nil) and "" or ("local BGD_MOUV_MESURE = " .. tostring(o.mesure) .. ";\r\n");
	local f = assert(load(Strip(FILE) .. "\r\n" .. mesure .. Strip(BLOCK), "SelectedUnit.lua", "t", env));
	f();
	function m.Fire(name, ...)
		for _, h in ipairs(m.handlers[name] or {}) do h(...); end
	end
	function m.Select()
		m.Fire("UnitSelectionChanged", LOCAL, 65536, 0, 0, 0, true, true);
	end
	function m.Lines(pattern)
		local r = {};
		for _, s in ipairs(m.prints) do if s:find(pattern) then table.insert(r, s); end end
		return r;
	end
	function m.Zone(layer)
		local s = {};
		for _, i in ipairs(m.layers[layer or "Movement_Range"] or {}) do s[i] = true; end
		return s;
	end
	m.env = env;
	return m;
end

-- L'ancien algorithme (avant l'exploration par front), sur la même machine, A* qui dit vrai.
local function OldZone(m)
	local env, kUnit = m.env, m.unit;
	m.truth = true;
	local count = env.table.count;
	local function Atteignable(iPlot)
		local kPath = env.UnitManager.GetMoveToPathEx(kUnit, iPlot);
		if kPath == nil or kPath.plots == nil or kPath.turns == nil then return false; end
		local n = count(kPath.plots);
		local t = count(kPath.turns);
		return n > 1 and t > 0 and kPath.plots[n] == iPlot and kPath.turns[t] <= 1;
	end
	local x, y = kUnit:GetX(), kUnit:GetY();
	local iPortee = math.min(10, math.max(1, math.ceil(kUnit:GetMovesRemaining())) * 4);
	local kAnneaux = {};
	for dx = -iPortee, iPortee do
		for dy = -iPortee, iPortee do
			local pPlot = env.Map.GetPlotXYWithRangeCheck(x, y, dx, dy, iPortee);
			if pPlot ~= nil then
				local d = env.Map.GetPlotDistance(x, y, pPlot:GetX(), pPlot:GetY());
				if d > 0 then kAnneaux[d] = kAnneaux[d] or {}; table.insert(kAnneaux[d], pPlot:GetIndex()); end
			end
		end
	end
	local kZone = { [env.Map.GetPlot(x, y):GetIndex()] = true };
	local iVides = 0;
	for d = 1, iPortee do
		local bTrouve = false;
		for _, iPlot in ipairs(kAnneaux[d] or {}) do
			if Atteignable(iPlot) then kZone[iPlot] = true; bTrouve = true; end
		end
		if bTrouve then iVides = 0; else iVides = iVides + 1; if iVides >= 2 then break; end end
	end
	m.truth = false;
	return kZone;
end

local function Same(a, b)
	for k in pairs(a) do if not b[k] then return false; end end
	for k in pairs(b) do if not a[k] then return false; end end
	return true;
end
local function Size(s) local n = 0; for _ in pairs(s) do n = n + 1; end return n; end
local function Field(line, key) return tonumber((line or ""):match(" " .. key .. "=([%d%.]+)")); end
local function Diff(a, b)
	local r = {};
	for k in pairs(a) do if not b[k] then table.insert(r, "+" .. (k % W) .. "," .. math.floor(k / W)); end end
	for k in pairs(b) do if not a[k] then table.insert(r, "-" .. (k % W) .. "," .. math.floor(k / W)); end end
	return table.concat(r, " ");
end

local failures = 0;
local function Check(name, ok, detail)
	print((ok and "ok      " or "ECHEC   ") .. name .. ((not ok and detail) and ("\n        " .. tostring(detail)) or ""));
	if not ok then failures = failures + 1; end
end

-- Zone identique à l'ancien algorithme (A* qui dit vrai), et gains, sur plusieurs cartes.
print("carte          PM  ancien: chemins noeuds  nouveau: chemins noeuds filtrees faux  zoc  zone");
for _, c in ipairs({
	{ "plaine", 1 }, { "plaine", 2 }, { "plaine", 3 }, { "plaine", 5 },
	{ "route", 2 }, { "route", 4 }, { "rail", 2 }, { "rail", 3 },
	{ "cote", 1 }, { "cote", 3 }, { "cote", 2, { embark = true } }, { "cote", 2, { traitEmbark = true, embark = true } },
	{ "ocean", 3, { embark = true } }, { "ocean", 3, { embark = true, ocean = true } },
	{ "brouillard_eau", 2 }, { "brouillard_eau", 3 },
	{ "montagnes", 2 }, { "montagnes", 3 }, { "brouillard", 3 }, { "couloir", 2 }, { "couloir", 3 },
	{ "zoc", 2 }, { "zoc", 3 }, { "zoc", 3, { ignoreZoc = true } }, { "zoc_riviere", 3 }, { "zoc", 2, { complete = true } },
}) do
	local kind, moves, extra = c[1], c[2], c[3] or {};
	local o = { map = kind, moves = moves };
	for k, v in pairs(extra) do o[k] = v; end
	local m = NewMachine(o);
	m.Select();
	local new, newPaths, newNodes = m.Zone(), m.stats.paths, m.stats.nodes;
	local line = m.Lines("^BGD mouvements: n=1 ")[1];
	m.stats.paths, m.stats.nodes = 0, 0;
	local old = OldZone(m);
	local label = kind .. " " .. moves .. " PM" .. (extra.embark and " embarque" or "") .. (extra.ocean and " ocean" or "")
		.. (extra.traitEmbark and " trait" or "") .. (extra.ignoreZoc and " ignore_zoc" or "") .. (extra.complete and " termine" or "");
	print(string.format("%-14s %d   %6d %7d          %6d %7d %6s %4s %4s  %4d  %s", kind, moves, m.stats.paths, m.stats.nodes, newPaths, newNodes,
		tostring(Field(line, "filtrees")), tostring(Field(line, "faux")), tostring(Field(line, "zoc")), Size(new), label));
	Check(label .. " : même zone que l'ancien algorithme avec un A* exact", Same(new, old), Size(new) .. " contre " .. Size(old) .. " : " .. Diff(new, old));
	Check(label .. " : pas de budget atteint", Field(line, "budget") == 0, line);
	if kind ~= "brouillard_eau" then
		Check(label .. " : pas plus de chemins", newPaths <= m.stats.paths, newPaths .. " contre " .. m.stats.paths);
	end
	-- Couche des zones de contrôle : cases de la zone (hors départ) voisines d'une source, comme le jeu.
	local zoc = m.Zone("Movement_Zone_Of_Control");
	local want = {};
	for i in pairs(new) do
		if i ~= UY * W + UX and ZocAt(m.cells, o, i, UY * W + UX) and m.cells[i].revealed ~= false then want[i] = true; end
	end
	if Size(want) > 0 then
		Check(label .. " : couche des zones de contrôle", Same(zoc, want), Size(zoc) .. " contre " .. Size(want) .. " : " .. Diff(zoc, want));
	end
end

-- Eau sans Construction navale : aucune case d'eau, même si l'A* du jeu en propose (non révélée).
do
	local m = NewMachine({ map = "brouillard_eau", moves = 3 });
	m.Select();
	local eau = 0;
	for i in pairs(m.Zone()) do if m.cells[i].water then eau = eau + 1; end end
	m.stats.paths = 0;
	m.truth = false;
	-- Ce que proposait l'ancien algorithme avec l'A* du jeu (qui croit la côte non révélée franchissable).
	local env, count = m.env, m.env.table.count;
	local fausses = 0;
	for i, cell in pairs(m.cells) do
		if cell.water and Dist(UX, UY, i % W, math.floor(i / W)) <= 3 then
			local p = env.UnitManager.GetMoveToPathEx(m.unit, i);
			local n = count(p.plots);
			if n > 1 and p.plots[n] == i and p.turns[n] <= 1 then fausses = fausses + 1; end
		end
	end
	Check("eau non révélée sans embarquement : l'A* du jeu la propose (" .. fausses .. " cases)", fausses > 0);
	Check("eau non révélée sans embarquement : aucune case d'eau dans la zone", eau == 0, eau);
	local s = m.Lines("^BGD mouvements: n=1 ")[1] or "";
	Check("ligne : embarque=0 ocean=0, eau filtrée", s:find("embarque=0 ocean=0", 1, true) and (Field(s, "filtrees") or 0) > 0, s);
end

-- Campement ennemi posé, pas terminé, en guerre, 2 PM : la ligne dit tout.
do
	local m = NewMachine({ moves = 2, slowMs = 1 });
	m.Select();
	local l = m.Lines("^BGD mouvements: n=");
	Check("une ligne de mesure", #l == 1, #l);
	local s = l[1] or "";
	print("        " .. s);
	for _, want in ipairs({
		"n=1 ", "cause=selection ", "prof=1 ", "repli=1 ", "raison=repli ", "portee=8 ", "arret=front_epuise ",
		"zone=19 ", "cache=0 ", "filtrees=0 ", "budget=0 ", "faux=0 ", "zoc=0 ", "embarque=0 ", "ocean=0 ",
		"unite=65536 ", "case=20,20 ", "pm=2 ",
		"quartier=DISTRICT_ENCAMPMENT ", "pv_base=100 ", "proprio=1 ", "unite_proprio=0 ", "guerre=1 ", "fini=0 ", "horloge=proxy",
	}) do
		Check("ligne : " .. want, s:find(want, 1, true) ~= nil, s);
	end
	Check("ligne : ms = chemins × 1 ms", Field(s, "ms") == Field(s, "chemins") and Field(s, "chemins_ms") == Field(s, "chemins"), s);
	m.Fire("LensLayerOn", "Movement_Range");
	local l2 = m.Lines("cause=LensLayerOn")[1];
	Check("LensLayerOn : cause nommée, zone servie par le cache", l2 and l2:find("cache=1 ", 1, true) and l2:find("chemins=0 ", 1, true)
		and l2:find("zone=19 ", 1, true) and l2:find("arret=cache ", 1, true), l2);
	Check("cache : la zone dessinée reste complète", Size(m.Zone()) == 19, Size(m.Zone()));
	m.Fire("UnitMovementPointsChanged", LOCAL, 65536);
	Check("MovementPointsChanged : cause nommée, cache", (m.Lines("cause=MovementPointsChanged")[1] or ""):find("cache=1 ", 1, true));
	m.Fire("UnitMoveComplete", ENEMY, 3, 0, 0);
	m.Select();
	Check("unité déplacée : cache invalidé", (m.prints[#m.prints] or ""):find("cache=0 ", 1, true), m.prints[#m.prints]);
	m.Select();
	Check("rien de changé : cache", (m.prints[#m.prints] or ""):find("cache=1 ", 1, true));
	m.turn = m.turn + 1;
	m.Select();
	Check("tour suivant : cache invalidé", (m.prints[#m.prints] or ""):find("cache=0 ", 1, true));
	m.Fire("ResearchCompleted", LOCAL, 1);
	m.Select();
	Check("technologie : cache invalidé", (m.prints[#m.prints] or ""):find("cache=0 ", 1, true));
	m.shutdown();
	local b = m.Lines("^BGD mouvements: bilan%(fermeture%)");
	Check("bilan à la fermeture", #b == 1 and b[1]:find("appels=7 replis=7", 1, true) ~= nil, b[1]);
	Check("fermeture : enveloppes et écouteurs retirés", #(m.handlers.UnitSelectionChanged or {}) == 0 and #(m.handlers.UnitMoveComplete or {}) == 0
		and #(m.handlers.ResearchCompleted or {}) == 0);
end

-- Zone de contrôle : on y entre, le front ne va pas au-delà ; couche rouge dessinée avec les
-- indicateurs d'attaque.
do
	local m = NewMachine({ map = "zoc", moves = 3 });
	m.Select();
	local zoc = m.Zone("Movement_Zone_Of_Control");
	Check("zoc : couche rouge non vide", Size(zoc) > 0, Size(zoc));
	Check("zoc : case voisine du guerrier dans la zone et en rouge", m.Zone()[UY * W + UX + 2] and zoc[UY * W + UX + 2]);
	Check("zoc : pas de case derrière le guerrier", not m.Zone()[UY * W + UX + 4]);
	Check("zoc : indicateurs d'attaque passés à la couche", m.layerExtras["Movement_Zone_Of_Control"] ~= nil);
end

-- PM changés : recalcul.
do
	local o = { moves = 2 };
	local m = NewMachine(o);
	m.Select();
	o.moves = 1;
	m.Select();
	local s = m.prints[#m.prints] or "";
	Check("PM changés : cache invalidé, zone de 1 PM", s:find("cache=0 ", 1, true) and s:find("zone=7 ", 1, true), s);
end

-- Budget de temps : 1 ms par chemin simulé sur rail à 3 PM, coupé à 50 ms, zone partielle dessinée.
do
	local m = NewMachine({ map = "rail", moves = 3, slowMs = 1 });
	m.Select();
	local s = m.Lines("^BGD mouvements: n=1 ")[1] or "";
	local zone = m.Zone();
	local old = OldZone(m);
	local inclus = true;
	for k in pairs(zone) do if not old[k] then inclus = false; end end
	Check("budget de temps : coupé, signalé", s:find("budget=1 ", 1, true) and s:find("arret=budget ", 1, true), s);
	Check("budget de temps : zone partielle incluse dans la vraie", inclus and Size(zone) > 1 and Size(zone) < Size(old), Size(zone) .. " / " .. Size(old));
end

-- Budget de chemins : sans horloge du proxy (os.clock), plafond de 200 chemins.
do
	local m = NewMachine({ map = "plaine", moves = 5 });
	rawset(m.env, "Civ6RolloverProxy_Clock", nil);
	for y = 0, H - 1 do for x = 0, W - 1 do if (x + y) % 2 == 0 and not (x == UX and y == UY) then m.cells[y * W + x].friend = true; end end end
	m.Select();
	local s = m.Lines("^BGD mouvements: n=1 ")[1] or "";
	Check("budget de chemins : 200 au plus", Field(s, "chemins") == 200 and s:find("budget=1 ", 1, true) and s:find("horloge=os.clock", 1, true), s);
end

-- Campement terminé : même chemin de code, fini=1.
do
	local m = NewMachine({ moves = 2, complete = true });
	m.Select();
	local s = m.Lines("^BGD mouvements: n=")[1] or "";
	Check("terminé : repli et fini=1", s:find("repli=1", 1, true) and s:find("fini=1", 1, true), s);
end

-- Quartier à soi, ou paix : pas de repli, pas de ligne.
do
	local m = NewMachine({ moves = 2, owner = LOCAL });
	m.Select();
	Check("quartier à soi : aucun chemin ni ligne", m.stats.paths == 0 and #m.Lines("^BGD mouvements: n=") == 0, m.prints[1]);
	m = NewMachine({ moves = 2, war = false });
	m.Select();
	Check("paix : aucun chemin ni ligne", m.stats.paths == 0 and #m.Lines("^BGD mouvements: n=") == 0);
end

-- Réglage ui_mesure_mouvements : off (défaut du proxy), même zone, aucune ligne ni bilan.
do
	local on = NewMachine({ moves = 2, mesure = true });
	on.Select();
	local off = NewMachine({ moves = 2, mesure = false });
	off.Select();
	local chemins = off.stats.paths;
	for _ = 1, 59 do off.Select(); end
	if off.shutdown then off.shutdown(); end
	Check("mesure on : ligne écrite", #on.Lines("^BGD mouvements: n=") == 1);
	Check("mesure off : aucune ligne ni bilan", #off.Lines("^BGD mouvements:") == 0, off.prints[1]);
	Check("mesure off : même zone calculée", chemins > 0 and chemins == on.stats.paths, chemins);
end

-- Plafond de lignes et bilans périodiques.
do
	local m = NewMachine({ moves = 2 });
	for _ = 1, 200 do m.Select(); end
	Check("plafond : 150 lignes", #m.Lines("^BGD mouvements: n=") == 150, #m.Lines("^BGD mouvements: n="));
	Check("plafond annoncé une fois", #m.Lines("plafond de 150 lignes") == 1);
	Check("bilans tous les 50 appels", #m.Lines("^BGD mouvements: bilan%(%d+%)") == 4);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
