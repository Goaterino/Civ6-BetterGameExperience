-- Test hors jeu des corrections de Real Modifier Analysis (src/mods/rapports/RealModifierAnalysis.lua) :
--   lua proxy/tests/test_mods_rma.lua
-- RMA est chargé comme par BGD_ChargerRMA, dans un environnement qui imite le jeu : tout ce qu'il
-- appelle sans qu'on s'y intéresse rend un objet neutre. On teste ensuite les fonctions corrigées.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local RMA_TEXT = ReadFile(dir .. "/../src/mods/rapports/RealModifierAnalysis.lua");
for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
	RMA_TEXT = RMA_TEXT:gsub("([%w_%)%]])%s*:%s*" .. t .. "%f[^%w_(]", "%1");
end

local Any;
Any = setmetatable({}, {
	__index = function(_, k) if type(k) == "number" then return nil; end return Any; end,
	__call = function() return Any; end,
});
-- GameInfo.X() itère ; GameInfo.X[clé] rend une ligne.
local function Table(rows)
	local byKey = {};
	for _, r in ipairs(rows) do
		for _, v in pairs(r) do byKey[v] = r; end
	end
	return setmetatable(byKey, { __call = function()
		local i = 0;
		return function() i = i + 1; return rows[i]; end;
	end });
end

local LEADER = "LEADER_CVS_ANACAONA";
local RMA = {};
local env = setmetatable({}, { __index = function(_, k) if _G[k] ~= nil then return _G[k]; end return Any; end });
env.BGD_RMA_Table = RMA;
env.BGD_Lookup = function(s) return s; end;
env.print = function() end;
env.Modding = { IsModActive = function() return true; end };
env.Game = { GetLocalPlayer = function() return 0; end };
env.PlayerConfigurations = { [0] = {
	GetLeaderTypeName = function() return LEADER; end,
	GetCivilizationTypeName = function() return "CIVILIZATION_CVS_TAINO"; end,
} };
env.GameInfo = setmetatable({
	Yields = Table({ { YieldType = "YIELD_SCIENCE", Index = 2 }, { YieldType = "YIELD_CULTURE", Index = 3 } }),
	GovernorPromotionSets = Table({
		{ GovernorType = "GOVERNOR_THE_EDUCATOR", GovernorPromotion = "GOVERNOR_PROMOTION_EDUCATOR_LIBRARIAN" },
		{ GovernorType = "GOVERNOR_THE_EDUCATOR", GovernorPromotion = "GOVERNOR_PROMOTION_EDUCATOR_CONNOISSEUR" },
		{ GovernorType = "GOVERNOR_THE_EDUCATOR", GovernorPromotion = "GOVERNOR_PROMOTION_EDUCATOR_RESEARCHER" },
		{ GovernorType = "GOVERNOR_THE_BUILDER", GovernorPromotion = "GOVERNOR_PROMOTION_BUILDER_GUILDMASTER" },
	}),
	GovernorPromotions = Table({
		{ GovernorPromotionType = "GOVERNOR_PROMOTION_EDUCATOR_LIBRARIAN", Hash = 101, Index = 0 },
		{ GovernorPromotionType = "GOVERNOR_PROMOTION_EDUCATOR_CONNOISSEUR", Hash = 102, Index = 1 },
		{ GovernorPromotionType = "GOVERNOR_PROMOTION_EDUCATOR_RESEARCHER", Hash = 103, Index = 2 },
		{ GovernorPromotionType = "GOVERNOR_PROMOTION_BUILDER_GUILDMASTER", Hash = 201, Index = 3 },
	}),
	Governors = setmetatable({}, { __index = function() return { GovernorType = "GOVERNOR_THE_EDUCATOR" }; end }),
	GreatPersonClasses = Table({
		{ GreatPersonClassType = "GREAT_PERSON_CLASS_ENGINEER", IconString = "[ICON_GreatEngineer]" },
		{ GreatPersonClassType = "GREAT_PERSON_CLASS_SCIENTIST", IconString = "[ICON_GreatScientist]" },
	}),
}, { __index = function() return Table({}); end });
env._G = env;

local chunk = assert(load(RMA_TEXT, "@RealModifierAnalysis.lua", "t", env));
local ok, err = pcall(chunk);
assert(ok, err);

local failures = 0;
local function Check(cond, what)
	if not cond then
		failures = failures + 1;
		print("ÉCHEC : " .. what);
	end
end

-- Fonctions exposées comme dans le mod.
Check(type(RMA.CalculateModifierEffect) == "function", "RMA.CalculateModifierEffect exposée");
local YieldTableGetYieldOrig = env.YieldTableGetYield;

-- Pourcentages d'une infobulle de rendement, en anglais et en français (espaces insécables).
local function City(tip) return { GetYieldToolTip = function() return tip; end }; end
local tipEN = "+12.5 from Buildings[NEWLINE]+75% (+6.3) from Modifiers[NEWLINE]-10% (-1.2) from Amenities";
local tipFR = "+12,5 des bâtiments[NEWLINE]+75\194\160% (+6,3) des modificateurs[NEWLINE]-10 % (-1,2) des activités";
Check(env.BGD_CityYieldPercent(City(tipEN), "YIELD_SCIENCE") == 65, "somme des % (anglais)");
Check(env.BGD_CityYieldPercent(City(tipFR), "YIELD_SCIENCE") == 65, "somme des % (français)");
Check(env.BGD_CityYieldPercent(City("+3 from Tiles"), "YIELD_SCIENCE") == 0, "aucun %");
Check(env.BGD_CityYieldPercent(nil, "YIELD_SCIENCE") == 0, "sans ville");

-- Monachisme : +75 % de science sur une ville à 33 de science totale, avec +65 % déjà appliqués :
-- base 20, gain 15 (et non 33 × 0,75 = 24,75).
local subject = { City = City(tipEN), Yields = Any };
env.YieldTableGetYield = function() return 33; end;
local impact = env.BGD_YieldModifierImpact(subject, "YIELD_SCIENCE", 75);
Check(math.abs(impact - 15) < 1e-9, "gain du Monachisme : " .. tostring(impact));
env.YieldTableGetYield = YieldTableGetYieldOrig;

-- Prérequis du propriétaire : dirigeant.
local function Set(testAny, reqs) return { TestAny = testAny, TestAll = not testAny, Reqs = reqs }; end
local anacaona = { ReqType = "REQUIREMENT_PLAYER_LEADER_TYPE_MATCHES", Arguments = { LeaderType = "LEADER_CVS_ANACAONA" } };
local autre = { ReqType = "REQUIREMENT_PLAYER_LEADER_TYPE_MATCHES", Arguments = { LeaderType = "LEADER_GANDHI" } };
local inconnu = { ReqType = "REQUIREMENT_PLAYER_HAS_TECHNOLOGY", Arguments = {} };
Check(env.BGD_OwnerRequirementsMet(Set(true, { anacaona })), "Anacaona joue Anacaona");
Check(not env.BGD_OwnerRequirementsMet(Set(true, { autre })), "pas Gandhi");
Check(env.BGD_OwnerRequirementsMet(Set(false, { anacaona, inconnu })), "type inconnu : vrai");
Check(not env.BGD_OwnerRequirementsMet(Set(false, { autre, inconnu })), "TEST_ALL : un faux suffit");
Check(not env.BGD_OwnerRequirementsMet(Set(false, { { ReqType = anacaona.ReqType, Arguments = anacaona.Arguments, Inverse = true } })), "inverse");
Check(env.BGD_OwnerRequirementsMet(nil), "sans prérequis");
LEADER = "LEADER_GANDHI";
Check(not env.BGD_OwnerRequirementsMet(Set(true, { anacaona })), "Gandhi ne joue pas Anacaona");

-- Promotions du gouverneur : par GovernorPromotionSets, HasPromotion par hachage.
env.bIsRiseAndFall = true;
local owned = { [101] = true, [103] = true, [201] = true };
local pGovernor = {
	GetType = function() return 0; end,
	IsEstablished = function() return true; end,
	HasPromotion = function(_, h) return owned[h] == true; end,
};
local pCity = { GetAssignedGovernor = function() return pGovernor; end };
local established, n = env.GetGovernorData(pCity);
Check(established == true and n == 2, "promotions de l'éducateur : " .. tostring(n));

-- Entretien : trois unités militaires (entretien 1, 2 et 4) et un bâtisseur ; le Trône du seigneur
-- de guerre réduit déjà de 1 (GetMaintDiscountPerUnit).
env.Players = { [0] = {
	GetTreasury = function() return { GetMaintDiscountPerUnit = function() return 1; end }; end,
	GetCulture = function() return { GetNumPolicySlots = function() return 0; end }; end,
} };
local player = { SubjectType = "Player", Units = {
	{ Maintenance = 1 }, { Maintenance = 2 }, { Maintenance = 4 }, { Maintenance = 0, IsCivilian = true },
} };
local function Entretien(amount)
	local t = env.ApplyEffectAndCalculateImpact({ EffectType = "EFFECT_ADJUST_UNIT_MAINTENANCE_DISCOUNT", Arguments = { Amount = tostring(amount) } }, player, "Player");
	return t and t.GOLD;
end
-- Conscription (1) : 0 + 1 + 1 ; Levée en masse (2) : 0 + 1 + 2 ; Forces d'élite (-2) : -2 × 3.
Check(Entretien(1) == 2, "Conscription après le Trône : " .. tostring(Entretien(1)));
Check(Entretien(2) == 3, "Levée en masse après le Trône : " .. tostring(Entretien(2)));
Check(Entretien(-2) == -6, "Forces d'élite : " .. tostring(Entretien(-2)));

-- Prétoire : villes annexées (REQUIREMENT_CITY_IS_ORIGINAL_OWNER inversé).
local function Ville(owner, original, x)
	return { SubjectType = "City", City = {
		GetOwner = function() return owner; end,
		GetOriginalOwner = function() return original; end,
		GetX = function() return x; end,
		GetY = function() return 0; end,
	} };
end
local fondee = { ReqType = "REQUIREMENT_CITY_IS_ORIGINAL_OWNER", Arguments = {}, Inverse = true };
Check(env.CheckOneRequirement(fondee, Ville(0, 3, 0), "City") == true, "Prétoire : ville annexée");
Check(env.CheckOneRequirement(fondee, Ville(0, 0, 0), "City") == false, "Prétoire : ville fondée");

-- Taxe coloniale : 8 cases ou plus de la capitale (en x seul ici).
env.Players[0].GetCities = function() return { GetCapitalCity = function() return { GetX = function() return 10; end, GetY = function() return 0; end }; end }; end;
env.Map = { GetPlotDistance = function(x1, y1, x2, y2) return math.abs(x1 - x2) + math.abs(y1 - y2); end };
local loin = { ReqType = "REQUIREMENT_PLOT_NEAR_CAPITAL", Arguments = { MinDistance = "8" } };
Check(env.CheckOneRequirement(loin, Ville(0, 0, 18), "City") == true, "Taxe coloniale : à 8 cases");
Check(env.CheckOneRequirement(loin, Ville(0, 0, 17), "City") == false, "Taxe coloniale : à 7 cases");

-- Points de personnages illustres : la clé de la classe, et « +x[icône] ».
local pg = env.ApplyEffectAndCalculateImpact({ EffectType = "EFFECT_ADJUST_GREAT_PERSON_POINTS",
	Arguments = { Amount = "2", GreatPersonClassType = "GREAT_PERSON_CLASS_ENGINEER" } }, Ville(0, 0, 0), "City");
Check(pg ~= nil and pg.PG_ENGINEER == 2, "points d'ingénieur par atelier");
local total = env.YieldTableNew();
Check(total.PG_ENGINEER == 0 and total.PG_SCIENTIST == 0, "clés des points dans une table neuve");
env.YieldTableAdd(total, pg);
env.YieldTableAdd(total, pg);
total.PG_ENGINEER = total.PG_ENGINEER + 4;
env.Locale = { ToNumber = function(v) return string.format("%+g", v); end };
Check(env.YieldTableGetInfo(total):find("[ICON_GreatEngineer]+8", 1, true) ~= nil, "texte des points : " .. env.YieldTableGetInfo(total));

-- Cartes en % des bâtiments (Rationalisme…) : base + bonus fixes lus au jeu, sans les % déjà
-- appliqués. Bibliothèque : 2 de base + 2 d'une cité-État ; Rationalisme posé, ses deux % remplis
-- (+50 % et +50 %) : le jeu donne 8. Seondeok ajoute +30 % au Campus par titre de gouverneur.
local function Req(id, t, args) return { ReqId = id, ReqType = t, Arguments = args }; end
local function ReqSet(reqs) return { TestAll = true, TestAny = false, Reqs = reqs }; end
local pop13 = ReqSet({ Req("POP", "REQUIREMENT_CITY_HAS_X_POPULATION", { Amount = "13" }) });
local titre1 = ReqSet({ Req("GOV", "REQUIREMENT_CITY_HAS_GOVERNOR_WITH_X_TITLES", { Amount = "1" }) });
local function PctMod(id, district, amount, reqset)
	return { ModifierId = id, EffectType = "EFFECT_ADJUST_BUILDING_YIELD_MODIFIERS_FOR_DISTRICT",
		Arguments = { DistrictType = district, YieldType = "YIELD_SCIENCE", Amount = tostring(amount) }, SubjectReqSet = reqset };
end
local tMods = {
	RATIONALISM_BUILDING_YIELDS_HIGH_POP = PctMod("RATIONALISM_BUILDING_YIELDS_HIGH_POP", "DISTRICT_CAMPUS", 50, pop13),
	RATIONALISM_BUILDING_YIELDS_HIGH_ADJACENCY = PctMod("RATIONALISM_BUILDING_YIELDS_HIGH_ADJACENCY", "DISTRICT_CAMPUS", 50, nil),
	GRANDOPERA_BUILDING_YIELDS_HIGH_POP = PctMod("GRANDOPERA_BUILDING_YIELDS_HIGH_POP", "DISTRICT_THEATER", 100, pop13),
	SEONDEOK_1 = PctMod("SEONDEOK_1", "DISTRICT_CAMPUS", 30, titre1),
	ATTACHE = { ModifierId = "ATTACHE", EffectType = "EFFECT_ATTACH_MODIFIER", Arguments = { ModifierId = "SEONDEOK_1" } },
	AUTRE = { ModifierId = "AUTRE", EffectType = "EFFECT_ADJUST_CITY_YIELD_CHANGE", Arguments = {} },
};
env.FetchAndCacheData = function(id) return tMods[id]; end;
env.GameInfo.Policies = Table({ { PolicyType = "POLICY_RATIONALISM", Index = 7 }, { PolicyType = "POLICY_GRAND_OPERA", Index = 8 } });
env.GameInfo.PolicyModifiers = Table({
	{ PolicyType = "POLICY_RATIONALISM", ModifierId = "RATIONALISM_BUILDING_YIELDS_HIGH_POP" },
	{ PolicyType = "POLICY_RATIONALISM", ModifierId = "RATIONALISM_BUILDING_YIELDS_HIGH_ADJACENCY" },
	{ PolicyType = "POLICY_GRAND_OPERA", ModifierId = "GRANDOPERA_BUILDING_YIELDS_HIGH_POP" },
});
env.GameInfo.LeaderTraits = Table({ { LeaderType = "LEADER_SEONDEOK", TraitType = "TRAIT_LEADER_HWARANG" } });
env.GameInfo.TraitModifiers = Table({ { TraitType = "TRAIT_LEADER_HWARANG", ModifierId = "ATTACHE" }, { TraitType = "TRAIT_LEADER_HWARANG", ModifierId = "AUTRE" } });
env.GameInfo.BuildingReplaces = Table({ { CivUniqueBuildingType = "BUILDING_MADRASA", ReplacesBuildingType = "BUILDING_UNIVERSITY" } });
env.GameInfo.DistrictReplaces = Table({ { CivUniqueDistrictType = "DISTRICT_SEOWON", ReplacesDistrictType = "DISTRICT_CAMPUS" } });
local tSlots = { 7, -1 };
env.Players[0].GetCulture = function() return {
	GetNumPolicySlots = function() return #tSlots; end,
	GetSlotPolicy = function(_, i) return tSlots[i + 1]; end,
	GetCurrentGovernment = function() return -1; end,
}; end;
LEADER = "LEADER_GANDHI";

-- Rendement réel lu au jeu (par index de rendement, ou par nom si l'index échoue).
local tBase = env.YieldTableNew(); tBase.SCIENCE = 2;
local pCityJeu = { GetBuildingYield = function(_, b, y) if y == 2 then return 8; elseif y == 3 then return 1; end return 0; end };
local tTotal, bJeu = env.BGD_BuildingYieldsTotal(pCityJeu, 12, tBase);
Check(bJeu and tTotal.SCIENCE == 8 and tTotal.CULTURE == 1, "rendement réel du bâtiment");
local pCityNom = { GetBuildingYield = function(_, b, y) if type(y) ~= "string" then error("index"); end return y == "YIELD_SCIENCE" and 5 or 0; end };
tTotal, bJeu = env.BGD_BuildingYieldsTotal(pCityNom, 12, tBase);
Check(bJeu and tTotal.SCIENCE == 5, "rendement réel par nom de rendement");
tTotal, bJeu = env.BGD_BuildingYieldsTotal({}, 12, tBase);
Check(not bJeu and tTotal.SCIENCE == 2, "sans GetBuildingYield : base");

local function Bibli(total, pille, type)
	local t = env.YieldTableNew(); t.SCIENCE = total;
	local b = env.YieldTableNew(); b.SCIENCE = 2;
	return { BuildingType = type or "BUILDING_LIBRARY", Yields = b, YieldsTotal = t, IsYieldsTotalFromGame = true, isPillaged = pille };
end
local function VilleCampus(pop, batiments, district)
	return { SubjectType = "City", Population = pop, IsGovernorEstablished = false, NumGovernorPromotions = 0,
		Districts = { { DistrictType = district or "DISTRICT_CAMPUS", isBuilt = true, Buildings = batiments } } };
end
local function Proche(a, b) return math.abs(a - b) < 1e-9; end
local v = VilleCampus(15, { Bibli(8) });
local f = env.BGD_BuildingYieldBeforePercent(v, "DISTRICT_CAMPUS", v.Districts[1].Buildings[1], "YIELD_SCIENCE");
Check(Proche(f, 4), "bibliothèque avant les % (Rationalisme posé, 2 %) : " .. tostring(f));
local rat = tMods.RATIONALISM_BUILDING_YIELDS_HIGH_ADJACENCY;
local t = env.ApplyEffectAndCalculateImpact(rat, v, "City");
Check(t ~= nil and Proche(t.SCIENCE, 2), "Rationalisme (+50 %) sur 4 : " .. tostring(t and t.SCIENCE));
-- Ville de 10 : seul le % de proximité est actif, le jeu donne 4 × 1,5 = 6.
v = VilleCampus(10, { Bibli(6) });
t = env.ApplyEffectAndCalculateImpact(rat, v, "City");
Check(t ~= nil and Proche(t.SCIENCE, 2), "ville de 10 : " .. tostring(t and t.SCIENCE));
-- Carte non posée : le rendement lu au jeu est celui avant la carte.
tSlots = { -1 };
env.CalculateModifierEffect("Policy", "POLICY_X", 0);
v = VilleCampus(15, { Bibli(4) });
t = env.ApplyEffectAndCalculateImpact(rat, v, "City");
Check(t ~= nil and Proche(t.SCIENCE, 2), "carte non posée : " .. tostring(t and t.SCIENCE));
-- Seondeok (trait, modificateur attaché) avec un titre de gouverneur : +30 % déjà dans les 5,2.
LEADER = "LEADER_SEONDEOK";
env.CalculateModifierEffect("Policy", "POLICY_X", 0);
v = VilleCampus(15, { Bibli(5.2, false, "BUILDING_MADRASA") }, "DISTRICT_SEOWON");
v.IsGovernorEstablished, v.NumGovernorPromotions = true, 1;
t = env.ApplyEffectAndCalculateImpact(rat, v, "City");
Check(t ~= nil and Proche(t.SCIENCE, 2), "Seondeok, Seowon et bâtiment unique : " .. tostring(t and t.SCIENCE));
LEADER = "LEADER_GANDHI";
env.CalculateModifierEffect("Policy", "POLICY_X", 0);
-- Bâtiment pillé : rien ; rendement réel indisponible : base de données, comme avant.
v = VilleCampus(15, { Bibli(4, true) });
t = env.ApplyEffectAndCalculateImpact(rat, v, "City");
Check(t ~= nil and t.SCIENCE == 0, "bâtiment pillé");
local sans = Bibli(4); sans.IsYieldsTotalFromGame = false;
v = VilleCampus(15, { sans });
t = env.ApplyEffectAndCalculateImpact(rat, v, "City");
Check(t ~= nil and Proche(t.SCIENCE, 1), "sans rendement réel : base 2 × 50 % = " .. tostring(t and t.SCIENCE));

-- Bonus des cités-États (CityStatesOnly) : hors des % dans le jeu. Bibliothèque : 2 de base + 2 de
-- Genève (3 émissaires, en paix) ; le jeu donne 4, Rationalisme (+50 %) rapporte 1 et non 2.
tMods.MINOR_SCI_MEDIUM = { ModifierId = "MINOR_SCI_MEDIUM", EffectType = "EFFECT_ATTACH_MODIFIER",
	Arguments = { ModifierId = "MINOR_SCI_LIBRARY" },
	SubjectReqSet = ReqSet({ Req("R1", "REQUIREMENT_PLAYER_HAS_GIVEN_INFLUENCE_TOKENS", { MinimumTokens = "3" }),
		Req("R2", "REQUIREMENT_PLAYER_AT_PEACE", {}) }) };
tMods.MINOR_SCI_LIBRARY = { ModifierId = "MINOR_SCI_LIBRARY", EffectType = "EFFECT_ADJUST_BUILDING_YIELD_CHANGE",
	Arguments = { BuildingType = "BUILDING_LIBRARY", YieldType = "YIELD_SCIENCE", Amount = "2", CityStatesOnly = "true" } };
env.DB = { Query = function(q) if q:find("CityStatesOnly", 1, true) then return { { ModifierId = "MINOR_SCI_LIBRARY" } }; end return {}; end };
env.GameInfo.Leaders = Table({ { LeaderType = "LEADER_MINOR_CIV_GENEVA", InheritFrom = "LEADER_MINOR_CIV_SCIENTIFIC" }, { LeaderType = "LEADER_MINOR_CIV_SCIENTIFIC" } });
env.GameInfo.LeaderTraits = Table({ { LeaderType = "LEADER_SEONDEOK", TraitType = "TRAIT_LEADER_HWARANG" },
	{ LeaderType = "LEADER_MINOR_CIV_SCIENTIFIC", TraitType = "MINOR_CIV_SCIENTIFIC_TRAIT" } });
env.GameInfo.TraitModifiers = Table({ { TraitType = "TRAIT_LEADER_HWARANG", ModifierId = "ATTACHE" }, { TraitType = "TRAIT_LEADER_HWARANG", ModifierId = "AUTRE" },
	{ TraitType = "MINOR_CIV_SCIENTIFIC_TRAIT", ModifierId = "MINOR_SCI_MEDIUM" } });
env.PlayerConfigurations[5] = { GetLeaderTypeName = function() return "LEADER_MINOR_CIV_GENEVA"; end };
local iTokens, bGuerre, bCiteEtat = 3, false, true;
local pGeneve = { GetID = function() return 5; end, IsMinor = function() return bCiteEtat; end,
	GetInfluence = function() return { GetTokensReceived = function(_, p) return p == 0 and iTokens or 0; end }; end };
env.PlayerManager = { GetAliveMinors = function() return { pGeneve }; end };
env.Players[0].GetDiplomacy = function() return { IsAtWarWith = function() return bGuerre; end }; end;
local function RationalismeAvecCE()
	env.CalculateModifierEffect("Policy", "POLICY_X", 0); -- vide les caches
	local vCE = VilleCampus(15, { Bibli(4) });
	local r = env.ApplyEffectAndCalculateImpact(rat, vCE, "City");
	return r and r.SCIENCE;
end
Check(Proche(RationalismeAvecCE(), 1), "cité-État : bonus hors du % : " .. tostring(RationalismeAvecCE()));
Check(Proche(env.BGD_CityStateBuildingBonus("BUILDING_LIBRARY", "YIELD_SCIENCE"), 2), "bonus de Genève sur la bibliothèque");
iTokens = 2;
Check(Proche(RationalismeAvecCE(), 2), "2 émissaires : pas de bonus à retirer");
iTokens, bGuerre = 3, true;
Check(Proche(RationalismeAvecCE(), 2), "en guerre : pas de bonus à retirer");
bGuerre, bCiteEtat = false, false;
Check(Proche(RationalismeAvecCE(), 2), "cité libre : pas de bonus");
bCiteEtat = true;

if failures == 0 then print("ok : corrections de RMA"); end
os.exit(failures == 0 and 0 or 1);
