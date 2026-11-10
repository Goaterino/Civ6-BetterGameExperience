-- Test hors jeu de src/ui_quartiers.lua (BET-22) :
--   lua proxy/tests/test_ui_quartiers.lua
-- Une base de quartiers réduite (dont le Thanh du Viêt Nam : RequiresPopulation à false mais modèle
-- NUM_UNDER_AVG_PLUS_TECH), un joueur et ses quartiers ; on vérifie la règle lue dans GameCore et le
-- badge posé dans la liste de production.

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
	local m = { districts = {}, cityDistricts = {}, techs = { TECH_BRONZE = true }, civics = {} };
	local D = Table({
		{ DistrictType = "DISTRICT_CAMPUS", RequiresPopulation = true, CostProgressionModel = MODELE, CostProgressionParam1 = 40 },
		{ DistrictType = "DISTRICT_HOLY_SITE", RequiresPopulation = true, CostProgressionModel = MODELE, CostProgressionParam1 = 40 },
		{ DistrictType = "DISTRICT_ENCAMPMENT", RequiresPopulation = true, CostProgressionModel = MODELE, CostProgressionParam1 = 40, PrereqTech = "TECH_BRONZE" },
		{ DistrictType = "DISTRICT_THANH", RequiresPopulation = false, CostProgressionModel = MODELE, CostProgressionParam1 = 40, PrereqTech = "TECH_BRONZE", TraitType = "TRAIT_CIVILIZATION_DISTRICT_THANH" },
		{ DistrictType = "DISTRICT_THEATER", RequiresPopulation = true, CostProgressionModel = MODELE, CostProgressionParam1 = 35, PrereqCivic = "CIVIC_DRAMA" },
		{ DistrictType = "DISTRICT_AQUEDUCT", RequiresPopulation = false, CostProgressionModel = "COST_PROGRESSION_GAME_PROGRESS", CostProgressionParam1 = 0 },
	}, "DistrictType");
	m.D = D;
	local env = setmetatable({}, { __index = _G });
	env.GameInfo = {
		Districts = D,
		DistrictReplaces = { DISTRICT_THANH = { ReplacesDistrictType = "DISTRICT_ENCAMPMENT" } },
		CivilizationTraits = Table({ { CivilizationType = "CIVILIZATION_VIETNAM", TraitType = "TRAIT_CIVILIZATION_DISTRICT_THANH" } }, "TraitType"),
		LeaderTraits = Table({}, "TraitType"),
		Technologies = Table({ { TechnologyType = "TECH_BRONZE" } }, "TechnologyType"),
		Civics = Table({ { CivicType = "CIVIC_DRAMA" } }, "CivicType"),
	};
	env.PlayerConfigurations = { [LOCAL] = {
		GetCivilizationTypeName = function() return m.civ or "CIVILIZATION_VIETNAM"; end,
		GetLeaderTypeName = function() return "LEADER_LADY_TRIEU"; end,
	} };
	local function Members(list)
		return function()
			local i = 0;
			return function() i = i + 1; if list[i] then return i, list[i]; end end;
		end;
	end
	local function District(t, complete)
		return { GetType = function() return D[t].Index; end, IsComplete = function() return complete; end };
	end
	function m.Add(t, complete) m.districts[#m.districts + 1] = District(t, complete); end
	function m.AddInCity(t) m.cityDistricts[#m.cityDistricts + 1] = District(t, false); m.Add(t, false); end
	env.Players = { [LOCAL] = {
		GetTechs = function() return { HasTech = function(_, i) return m.techs[env.GameInfo.Technologies[i].TechnologyType] == true; end }; end,
		GetCulture = function() return { HasCivic = function(_, i) return m.civics[env.GameInfo.Civics[i].CivicType] == true; end }; end,
		GetDistricts = function() return { Members = Members(m.districts) }; end,
	} };
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.UI = { GetHeadSelectedCity = function() return { GetDistricts = function() return { Members = Members(m.cityDistricts) }; end }; end };
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
	function m.Reductions() return env.BGD_ReductionsQuartiers(LOCAL); end
	-- Une ligne de la liste, après un remplissage du panneau.
	function m.Line(t)
		env.View({});
		local inst = { tip = nil };
		inst.LabelText = { SetText = function(_, s) inst.text = s; end, GetText = function() return inst.text; end };
		inst.Button = { SetToolTipString = function(_, s) inst.tip = s; end };
		env.PopulateGenericBuildData(inst, { Type = t, ToolTip = "infobulle" });
		return inst;
	end
	return m;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Viêt Nam : types comptés = campus, lieu saint (Thanh sans RequiresPopulation, caserne remplacée,
-- théâtre sans son dogme). Deux campus terminés : moyenne 2/2 = 1.
do
	local m = NewMachine();
	m.Add("DISTRICT_CAMPUS", true);
	m.Add("DISTRICT_CAMPUS", true);
	local r = m.Reductions();
	Check("moyenne 1 : lieu saint en réduction (0 < 1)", r.DISTRICT_HOLY_SITE == 40);
	Check("moyenne 1 : Thanh en réduction, hors moyenne", r.DISTRICT_THANH == 40);
	Check("moyenne 1 : campus sans réduction (2)", r.DISTRICT_CAMPUS == nil);
	Check("aqueduc : autre modèle, jamais", r.DISTRICT_AQUEDUCT == nil);
	local line = m.Line("DISTRICT_THANH");
	Check("badge : « -40% » ajouté au nom", line.text == "DISTRICT_THANH [COLOR_GREEN]-40%[ENDCOLOR]");
	Check("badge : infobulle traduite, puis celle du jeu", line.tip ~= nil and line.tip:find("Réduction de quartier : -40 %", 1, true) == 1
		and line.tip:find("infobulle", 1, true) ~= nil);
	Check("pas de badge sur le campus", m.Line("DISTRICT_CAMPUS").text == "DISTRICT_CAMPUS");

	-- Un lieu saint posé (pas terminé) compte tout de suite : 1 n'est pas < 1.
	m.AddInCity("DISTRICT_HOLY_SITE");
	Check("lieu saint posé : réduction perdue", m.Reductions().DISTRICT_HOLY_SITE == nil);
	Check("lieu saint posé dans la ville : pas de badge (coût fixé)", m.Line("DISTRICT_HOLY_SITE").text == "DISTRICT_HOLY_SITE");
	-- Un Thanh compte pour la caserne et le Thanh.
	m.Add("DISTRICT_THANH", false);
	Check("Thanh posé : réduction perdue", m.Reductions().DISTRICT_THANH == nil);
end

-- Moyenne sous 1 : aucune réduction.
do
	local m = NewMachine();
	m.Add("DISTRICT_CAMPUS", true);
	local r = m.Reductions();
	Check("moyenne 0,5 : aucune réduction", next(r) == nil);
end

-- Sans le Viêt Nam : la caserne est comptée, le Thanh n'est pas proposé.
do
	local m = NewMachine();
	m.civ = "CIVILIZATION_ROME";
	m.Add("DISTRICT_CAMPUS", true);
	m.Add("DISTRICT_CAMPUS", true);
	m.Add("DISTRICT_CAMPUS", true);
	local r = m.Reductions();
	-- 3 terminés / 3 types (campus, lieu saint, caserne) = 1.
	Check("Rome : caserne en réduction", r.DISTRICT_ENCAMPMENT == 40);
	Check("Rome : pas de Thanh", r.DISTRICT_THANH == nil);
	-- Le dogme du théâtre ajoute un type compté : 3/4 < 1.
	m.civics.CIVIC_DRAMA = true;
	Check("Rome, théâtre débloqué : moyenne 0,75, plus de réduction", next(m.Reductions()) == nil);
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
	Check("quartier posé, dogme : panneau redessiné", m.refreshes == 2);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
