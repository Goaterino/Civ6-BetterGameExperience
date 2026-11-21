-- Test hors jeu de src/ui_quartiers.lua (BET-22) :
--   lua proxy/tests/test_ui_quartiers.lua
-- Le badge suit le coût que le jeu a enregistré (BuildQueue:GetDistrictCost), pas la règle
-- recalculée : on vérifie le rapprochement avec le coût plein ou réduit (avancement, vitesse, ère
-- de départ) et le badge posé dans la liste de production.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua")
	.. ReadFile(dir .. "/../src/ui_quartiers.lua");

local MODELE = "COST_PROGRESSION_NUM_UNDER_AVG_PLUS_TECH";
local LOCAL = 0;

-- Une table de GameInfo : appelable (itération), indexable par nom ou par index.
local function Table(rows, key)
	local t = {};
	for i, r in ipairs(rows) do
		r.Index = i - 1;
		t[r[key]] = r;
		t[i - 1] = r;
	end
	return setmetatable(t, { __call = function()
		local i = 0;
		return function() i = i + 1; return rows[i]; end;
	end });
end

local function NewMachine()
	local m = { cityDistricts = {}, techs = {}, civics = {}, costs = {}, speed = 100, startEra = "ERA_ANCIENT" };
	local D = Table({
		{ DistrictType = "DISTRICT_CAMPUS", Cost = 54, CostProgressionModel = MODELE, CostProgressionParam1 = 40 },
		{ DistrictType = "DISTRICT_THANH", Cost = 54, CostProgressionModel = MODELE, CostProgressionParam1 = 40 },
		{ DistrictType = "DISTRICT_THEATER", Cost = 54, CostProgressionModel = MODELE, CostProgressionParam1 = 25 },
		{ DistrictType = "DISTRICT_AQUEDUCT", Cost = 36, CostProgressionModel = "COST_PROGRESSION_GAME_PROGRESS", CostProgressionParam1 = 1000 },
	}, "DistrictType");
	m.D = D;
	local techs, civics = {}, {};
	for i = 1, 10 do techs[i] = { TechnologyType = "TECH_" .. i }; civics[i] = { CivicType = "CIVIC_" .. i }; end
	local env = setmetatable({}, { __index = _G });
	env.GameInfo = {
		Districts = D,
		Technologies = Table(techs, "TechnologyType"),
		Civics = Table(civics, "CivicType"),
		GlobalParameters = { GAME_COST_ESCALATION = { Name = "GAME_COST_ESCALATION", Value = "1000" } },
		GameSpeeds = setmetatable({}, { __index = function() return { CostMultiplier = m.speed }; end }),
		Eras = Table({ { EraType = "ERA_ANCIENT", Hash = 11 }, { EraType = "ERA_MEDIEVAL", Hash = 22 } }, "EraType"),
		StartEras = Table({ { EraType = "ERA_ANCIENT" }, { EraType = "ERA_MEDIEVAL", DistrictProductionRate = 40 } }, "EraType"),
	};
	env.GameConfiguration = {
		GetGameSpeedType = function() return 0; end,
		GetStartEra = function() return env.GameInfo.Eras[m.startEra].Hash; end,
	};
	local function Members(list)
		return function()
			local i = 0;
			return function() i = i + 1; if list[i] then return i, list[i]; end end;
		end;
	end
	function m.AddInCity(t)
		m.cityDistricts[#m.cityDistricts + 1] = { GetType = function() return D[t].Index; end };
	end
	env.Players = { [LOCAL] = {
		GetTechs = function() return { HasTech = function(_, i) return m.techs[i] == true; end }; end,
		GetCulture = function() return { HasCivic = function(_, i) return m.civics[i] == true; end }; end,
	} };
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.UI = { GetHeadSelectedCity = function() return {
		GetDistricts = function() return { Members = Members(m.cityDistricts) }; end,
		GetBuildQueue = function() return { GetDistrictCost = function(_, i) return m.costs[D[i].DistrictType]; end }; end,
	}; end };
	env.Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end };
	env.ContextPtr = { IsHidden = function() return m.hidden ~= false; end };
	m.refreshes = 0;
	env.Refresh = function() m.refreshes = m.refreshes + 1; end;
	m.handlers = {};
	env.Events = setmetatable({}, { __index = function(_, k) return { Add = function(h) m.handlers[k] = h; end }; end });
	-- ProductionPanel.lua : ce que le bloc enveloppe.
	env.PopulateGenericBuildData = function(inst, item) inst.text = item.Type; end;
	env.View = function() end;
	assert(load(BLOCK, "ui_quartiers.lua", "t", env))();
	m.env = env;
	-- Une ligne de la liste, après un remplissage du panneau ; coût porté par l'élément sauf si
	-- `viaFile` (repli sur BuildQueue:GetDistrictCost).
	function m.Line(t, viaFile)
		env.View({});
		local inst = { tip = nil };
		inst.LabelText = { SetText = function(_, s) inst.text = s; end, GetText = function() return inst.text; end };
		inst.Button = { SetToolTipString = function(_, s) inst.tip = s; end };
		env.PopulateGenericBuildData(inst, { Type = t, ToolTip = "infobulle", Cost = (not viaFile) and m.costs[t] or nil });
		return inst;
	end
	function m.Badge(t, viaFile) return m.Line(t, viaFile).text ~= t; end
	return m;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Départ : aucune technologie, vitesse normale. Plein 54, réduit (40 %) 32.
do
	local m = NewMachine();
	m.costs.DISTRICT_CAMPUS = 54;
	m.costs.DISTRICT_THANH = 32;
	Check("coût plein : pas de badge", not m.Badge("DISTRICT_CAMPUS"));
	local line = m.Line("DISTRICT_THANH");
	Check("coût réduit : « -40% » ajouté au nom", line.text == "DISTRICT_THANH [COLOR_GREEN]-40%[ENDCOLOR]");
	Check("badge : infobulle traduite, puis celle du jeu", line.tip ~= nil and line.tip:find("Réduction de quartier : -40 %", 1, true) == 1
		and line.tip:find("infobulle", 1, true) ~= nil);
	Check("coût lu par la file si l'élément n'en porte pas", m.Badge("DISTRICT_THANH", true) and not m.Badge("DISTRICT_CAMPUS", true));
	m.costs.DISTRICT_AQUEDUCT = 20;
	Check("aqueduc : autre modèle, jamais", not m.Badge("DISTRICT_AQUEDUCT"));
	-- Le jeu n'a pas encore recalculé : coût plein même si la règle donnerait la réduction.
	m.costs.DISTRICT_THANH = 54;
	Check("coût enregistré encore plein : pas de badge", not m.Badge("DISTRICT_THANH"));
	m.costs.DISTRICT_THANH = 32;
	m.AddInCity("DISTRICT_THANH");
	Check("posé dans la ville : pas de badge (coût fixé)", not m.Badge("DISTRICT_THANH"));
end

-- Avancement : 5 technologies sur 10, 2 dogmes sur 10 → N = 50.
-- Plein : 54 + (540 - 54) × 50 / 100 = 297 ; réduit : 32 + (320 - 32) × 50 / 100 = 176.
do
	local m = NewMachine();
	for i = 0, 4 do m.techs[i] = true; end
	m.civics[0], m.civics[1] = true, true;
	m.costs.DISTRICT_CAMPUS, m.costs.DISTRICT_THANH = 297, 176;
	Check("N = 50 : 297 plein", not m.Badge("DISTRICT_CAMPUS"));
	Check("N = 50 : 176 réduit", m.Badge("DISTRICT_THANH"));
	-- Théâtre à 25 % : réduit 40 → 40 + (400 - 40) / 2 = 220.
	m.costs.DISTRICT_THEATER = 220;
	Check("théâtre : « -25% »", m.Line("DISTRICT_THEATER").text == "DISTRICT_THEATER [COLOR_GREEN]-25%[ENDCOLOR]");
end

-- Vitesse en ligne (50 %) : plein 27, réduit 16.
do
	local m = NewMachine();
	m.speed = 50;
	m.costs.DISTRICT_CAMPUS, m.costs.DISTRICT_THANH = 27, 16;
	Check("vitesse 50 : 27 plein", not m.Badge("DISTRICT_CAMPUS"));
	Check("vitesse 50 : 16 réduit", m.Badge("DISTRICT_THANH"));
end

-- BGE-36E0D7 : campus posé dans une autre ville, la table de GameCore est passée au plein (27) mais
-- le cache de cette ville donne encore le réduit (16). Le badge suit le coût lu en direct.
do
	local m = NewMachine();
	m.speed = 50;
	m.costs.DISTRICT_CAMPUS = 16;	-- cache de la ville, périmé
	local direct = 27;
	m.env.BGD_CoutQuartier = function(joueur, index)
		return (joueur == LOCAL and index == m.D.DISTRICT_CAMPUS.Index) and direct or nil;
	end;
	Check("coût direct plein, cache réduit périmé : pas de badge", not m.Badge("DISTRICT_CAMPUS"));
	direct = 16;
	Check("coût direct réduit : badge", m.Badge("DISTRICT_CAMPUS"));
	direct = nil;
	Check("coût direct absent : repli sur le coût du panneau", m.Badge("DISTRICT_CAMPUS"));
end

-- Départ médiéval (DistrictProductionRate 40) : plein 32, réduit 19.
do
	local m = NewMachine();
	m.startEra = "ERA_MEDIEVAL";
	m.costs.DISTRICT_CAMPUS, m.costs.DISTRICT_THANH = 32, 19;
	Check("ère de départ : 32 plein (pas confondu avec le réduit sans ère)", not m.Badge("DISTRICT_CAMPUS"));
	Check("ère de départ : 19 réduit", m.Badge("DISTRICT_THANH"));
end

-- Événements : panneau ouvert seulement, joueur local seulement.
do
	local m = NewMachine();
	m.handlers.DistrictAddedToMap(LOCAL);
	Check("panneau fermé : pas de redessin", m.refreshes == 0);
	m.hidden = false;
	m.handlers.DistrictAddedToMap(1);
	Check("autre joueur : pas de redessin", m.refreshes == 0);
	m.handlers.DistrictAddedToMap(LOCAL);
	m.handlers.CivicCompleted(LOCAL);
	m.handlers.CityAddedToMap(LOCAL);
	Check("quartier posé, dogme, ville fondée : panneau redessiné", m.refreshes == 3);
end

-- Diagnostic (diag_production) : règle refaite et ligne de Lua.log, rien sans le réglage.
do
	local m = NewMachine();
	m.D.DISTRICT_CAMPUS.RequiresPopulation = true;
	m.D.DISTRICT_THEATER.RequiresPopulation = true;
	m.D.DISTRICT_THANH.RequiresPopulation = true;
	m.D.DISTRICT_THANH.TraitType = "TRAIT_VIETNAM";
	m.speed = 50;
	local lignes = {};
	m.env.print = function(s) lignes[#lignes + 1] = s; end;
	local quartiers = {};
	local function Ajoute(t, acheve)
		quartiers[#quartiers + 1] = { GetType = function() return m.D[t].Index; end, IsComplete = function() return acheve; end };
	end
	m.env.Players[LOCAL].GetDistricts = function()
		return { Members = function()
			local i = 0;
			return function() i = i + 1; if quartiers[i] then return i, quartiers[i]; end end;
		end };
	end;
	m.env.Game.GetCurrentGameTurn = function() return 41; end;
	local ville = m.env.UI.GetHeadSelectedCity;
	m.env.UI.GetHeadSelectedCity = function()
		local c = ville();
		c.GetName = function() return "LOC_CITY_NAME_AKHETATEN"; end;
		c.GetID = function() return 327683; end;
		return c;
	end;
	Ajoute("DISTRICT_CAMPUS", true);
	Ajoute("DISTRICT_THEATER", true);
	Ajoute("DISTRICT_THEATER", true);
	m.costs.DISTRICT_CAMPUS = 16;
	m.Line("DISTRICT_CAMPUS");
	Check("diag coupé : aucune ligne", #lignes == 0);
	m.env.BGD_DIAG_PRODUCTION = true;
	m.Line("DISTRICT_CAMPUS");
	local l = lignes[#lignes] or "";
	Check("diag : une ligne par ville et par type", #lignes == 1 and l:find("BGD diag quartiers : tour 41, ville LOC_CITY_NAME_AKHETATEN (327683), DISTRICT_CAMPUS", 1, true) == 1);
	Check("diag : coûts, nombre, moyenne sur 2 types (Thanh d'une autre civilisation exclu), règle réduite",
		l:find("kItem.Cost 16, GetDistrictCost 16, GameCore nil, plein 27, reduit 16, badge oui", 1, true) ~= nil
		and l:find("nombre 1 (acheves 1, en construction 0), moyenne 1.500 (3 acheves / 2 types debloques", 1, true) ~= nil
		and l:find("regle reduit", 1, true) ~= nil);
	m.Line("DISTRICT_CAMPUS");
	Check("diag : rien de neuf, pas de nouvelle ligne", #lignes == 1);
	-- Deuxième campus posé ailleurs, en construction : la règle passe au plein ; le coût lu reste
	-- celui que le jeu donne (ici encore réduit), et le badge le suit.
	Ajoute("DISTRICT_CAMPUS", false);
	m.Line("DISTRICT_CAMPUS");
	l = lignes[#lignes] or "";
	Check("diag : campus en construction compté, règle pleine, badge encore affiché",
		#lignes == 2 and l:find("nombre 2 (acheves 1, en construction 1)", 1, true) ~= nil
		and l:find("regle plein", 1, true) ~= nil and l:find("badge oui", 1, true) ~= nil);
	m.hidden = false;
	m.handlers.DistrictAddedToMap(LOCAL, 7, 3, 0, 0, m.D.DISTRICT_CAMPUS.Index);
	Check("diag : quartier posé journalisé", (lignes[#lignes] or ""):find("evenement DistrictAddedToMap DISTRICT_CAMPUS ville 3", 1, true) ~= nil);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
