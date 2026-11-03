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
Check(env.YieldTableGetInfo(total):find("+8[ICON_GreatEngineer]", 1, true) ~= nil, "texte des points : " .. env.YieldTableGetInfo(total));

if failures == 0 then print("ok : corrections de RMA"); end
os.exit(failures == 0 and 0 or 1);
