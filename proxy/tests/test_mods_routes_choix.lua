-- Test hors jeu de src/mods/routes_choix.lua (or reçu par la destination d'une route intérieure) :
--   lua proxy/tests/test_mods_routes_choix.lua
-- Le bloc est chargé après un TradeRouteChooser.lua minimal : GetYieldsForRoute et GetYieldForCity
-- rendent les rendements de la ville d'origine que donne le jeu (2 or, 1 nourriture).

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua")
	.. ReadFile(dir .. "/../src/mods/routes_choix.lua");

local CREE, AUTRE = 0, 1;
local CAMP, PATURAGE, FERME = 10, 11, 12;
local FOOD, PROD, GOLD = 0, 1, 2;

-- Une table de la base : itérable, et indexable par clé primaire.
local function Table(rows, key)
	local t = {};
	for _, r in ipairs(rows) do if key then t[r[key]] = r; end end
	return setmetatable(t, { __call = function()
		local i = 0;
		return function() i = i + 1; return rows[i]; end;
	end });
end

local function Args(id, list)
	local out = {};
	for k, v in pairs(list) do table.insert(out, { ModifierId = id, Name = k, Value = v }); end
	return out;
end
local args = {};
for _, a in ipairs({
	Args("TRAIT_TRADE_FOOD_FROM_CAMPS", { Amount = "1", ImprovementType = "IMPROVEMENT_CAMP", YieldType = "YIELD_FOOD", Origin = "true" }),
	Args("TRAIT_TRADE_GOLD_FROM_CAMPS", { Amount = "1", ImprovementType = "IMPROVEMENT_CAMP", YieldType = "YIELD_GOLD", Destination = "true" }),
	Args("TRAIT_TRADE_GOLD_FROM_PASTURES", { Amount = "1", ImprovementType = "IMPROVEMENT_PASTURE", YieldType = "YIELD_GOLD", Destination = "true" }),
}) do for _, r in ipairs(a) do table.insert(args, r); end end

local TYPE = "MODIFIER_PLAYER_ADJUST_TRADE_ROUTE_YIELD_PER_IMPROVEMENT_IN_TARGET";
GameInfo = {
	LeaderTraits = Table({ { LeaderType = "LEADER_POUNDMAKER", TraitType = "TRAIT_LEADER_ALLIANCE_AND_TRADE" } }),
	CivilizationTraits = Table({}),
	TraitModifiers = Table({
		{ TraitType = "TRAIT_LEADER_ALLIANCE_AND_TRADE", ModifierId = "TRAIT_TRADE_FOOD_FROM_CAMPS" },
		{ TraitType = "TRAIT_LEADER_ALLIANCE_AND_TRADE", ModifierId = "TRAIT_TRADE_GOLD_FROM_CAMPS" },
		{ TraitType = "TRAIT_LEADER_ALLIANCE_AND_TRADE", ModifierId = "TRAIT_TRADE_GOLD_FROM_PASTURES" },
	}),
	Modifiers = Table({
		{ ModifierId = "TRAIT_TRADE_FOOD_FROM_CAMPS", ModifierType = TYPE },
		{ ModifierId = "TRAIT_TRADE_GOLD_FROM_CAMPS", ModifierType = TYPE },
		{ ModifierId = "TRAIT_TRADE_GOLD_FROM_PASTURES", ModifierType = TYPE },
	}, "ModifierId"),
	ModifierArguments = Table(args),
	Improvements = Table({ { ImprovementType = "IMPROVEMENT_CAMP", Index = CAMP }, { ImprovementType = "IMPROVEMENT_PASTURE", Index = PATURAGE } }, "ImprovementType"),
	Traits = Table({ { TraitType = "TRAIT_LEADER_ALLIANCE_AND_TRADE", Name = "LOC_LEADER_POUNDMAKER_ABILITY_NAME" } }, "TraitType"),
};
local yields = { { YieldType = "YIELD_FOOD", Index = FOOD, IconString = "[ICON_Food]", Name = "LOC_YIELD_FOOD_NAME" },
	{ YieldType = "YIELD_PRODUCTION", Index = PROD, IconString = "[ICON_Production]", Name = "LOC_YIELD_PRODUCTION_NAME" },
	{ YieldType = "YIELD_GOLD", Index = GOLD, IconString = "[ICON_Gold]", Name = "LOC_YIELD_GOLD_NAME" } };
GameInfo.Yields = Table(yields, "YieldType");
for _, y in ipairs(yields) do GameInfo.Yields[y.Index] = y; end

local leaders = { [CREE] = "LEADER_POUNDMAKER", [AUTRE] = "LEADER_TRAJAN" };
PlayerConfigurations = setmetatable({}, { __index = function(_, p)
	return { GetLeaderTypeName = function() return leaders[p]; end, GetCivilizationTypeName = function() return "CIV"; end };
end });
Locale = {
	Lookup = function(s) return ({ LOC_LEADER_POUNDMAKER_ABILITY_NAME = "Favorable Terms", LOC_YIELD_GOLD_NAME = "Gold" })[s] or s; end,
	GetCurrentLanguage = function() return { GetType = function() return "en_US"; end }; end,
};

-- Cases : [ville] = { { aménagement, pillé } }
local plots = {};
local function City(owner, id, name, cases)
	plots[id] = cases;
	return { GetOwner = function() return owner; end, GetID = function() return id; end, GetName = function() return name; end };
end
local allPlots = {};
Map = {
	GetCityPlots = function() return { GetPurchasedPlots = function(_, city)
		local out = {};
		for _, c in ipairs(plots[city:GetID()]) do table.insert(allPlots, c); table.insert(out, #allPlots); end
		return out;
	end }; end,
	GetPlotByIndex = function(i)
		local c = allPlots[i];
		return { GetImprovementType = function() return c[1]; end, IsImprovementPillaged = function() return c[2] == true; end };
	end,
};

local origine = City(CREE, 1, "Origine", {});
local interieure = City(CREE, 2, "Interieure", { { CAMP }, { CAMP }, { PATURAGE }, { PATURAGE, true }, { FERME }, { -1 } });
local etrangere = City(AUTRE, 3, "Etrangere", { { CAMP } });
local sansRien = City(CREE, 4, "SansRien", { { FERME } });

-- Le TradeRouteChooser.lua minimal.
function GetOriginCity() return origine; end
function GetYieldsForRoute(o, d, dest)
	if dest then return { kYieldValues = { 0, 0, 5 }, TooltipText = "dest" }; end
	return { kYieldValues = { 1, 0, 2 }, TooltipText = "jeu" };
end
function GetYieldForCity(yieldIndex, city, originCity)
	return ({ [FOOD] = 1, [PROD] = 0, [GOLD] = 2 })[yieldIndex], true;
end

assert(load(BLOCK, "routes_choix.lua", "t", _G))();

local function check(cond, msg) if not cond then error("ÉCHEC : " .. msg, 2); end print("ok : " .. msg); end

local r = GetYieldsForRoute(origine, interieure);
check(r.kYieldValues[3] == 2 + 3, "route intérieure : 2 camps et 1 pâturage non pillé, +3 or");
check(r.kYieldValues[1] == 1 and r.kYieldValues[2] == 0, "les autres rendements ne changent pas (la nourriture d'origine est déjà comptée par le jeu)");
check(r.TooltipText == "jeu[NEWLINE]+3 [ICON_Gold] Gold from Favorable Terms, received by Interieure", "ligne d'infobulle : " .. r.TooltipText);

r = GetYieldsForRoute(origine, etrangere);
check(r.kYieldValues[3] == 2 and r.TooltipText == "jeu", "route vers un autre joueur : rien d'ajouté");
r = GetYieldsForRoute(origine, interieure, true);
check(r.kYieldValues[3] == 5 and r.TooltipText == "dest", "rendements de la destination : rien d'ajouté");
r = GetYieldsForRoute(origine, sansRien);
check(r.kYieldValues[3] == 2 and r.TooltipText == "jeu", "destination sans camp ni pâturage : rien d'ajouté");

local v, bonus = GetYieldForCity(GOLD, interieure, true);
check(v == 5 and bonus == true, "filtre par or : +3 pour la route intérieure");
check(GetYieldForCity(FOOD, interieure, true) == 1, "filtre par nourriture : rien d'ajouté");
check(GetYieldForCity(GOLD, interieure, false) == 2, "filtre côté destination : rien d'ajouté");

origine = City(AUTRE, 5, "AutreOrigine", {});
local autreInterieure = City(AUTRE, 6, "AutreInterieure", { { CAMP } });
r = GetYieldsForRoute(origine, autreInterieure);
check(r.kYieldValues[3] == 2, "joueur sans la capacité : rien d'ajouté");
print("tout est bon");
