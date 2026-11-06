-- Test hors jeu de src/mods/drapeaux.lua (charges des bâtisseurs, tours des routes commerciales) :
--   lua proxy/tests/test_mods_drapeaux.lua
-- Le bloc est chargé après un UnitFlagManager.lua minimal : UnitFlag.UpdatePromotions du jeu montre
-- le nombre de promotions, comme l'original.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local f = assert(io.open(dir .. "/../src/mods/drapeaux.lua", "rb"));
local BLOCK = f:read("a");
local function BGD_LireTexte(p) local h = assert(io.open(p, "rb")); local s = h:read("a"); h:close(); return s; end
local BGD_TEXTES_LUA = BGD_LireTexte(dir .. "/../../textes/bgd_textes.lua") .. BGD_LireTexte(dir .. "/../../textes/bgd_texte.lua");
BLOCK = BGD_TEXTES_LUA .. BLOCK;
f:close();

local LOCAL = 0;
local TURN = 40;
local units = {};
local routes = {};		-- routes de la ville unique du joueur local
local finRoute = {};	-- [unitID] = { fin, départ } rendus par le proxy
local chemin = nil;	-- cases du trajet (GetTradeRoutePath), ou nil
local positions = {};	-- [unitID] = case du négociant

local function NewInstance()
	local flag = { hidden = true, text = nil, tip = nil };
	return {
		Promotion_Flag = {
			SetHide = function(_, h) flag.hidden = h; end,
			SetToolTipString = function(_, s) flag.tip = s; end,
		},
		UnitNumPromotions = { SetText = function(_, s) flag.text = s; end },
		state = flag,
	};
end

local function Run(charges, routesOn)
	local env = setmetatable({}, { __index = _G });
	env.UnitFlag = {};
	env.UnitFlag.UpdatePromotions = function(self)
		self.m_Instance.Promotion_Flag:SetHide(true);
		local n = self.unit.promotions or 0;
		if n > 0 then
			self.m_Instance.UnitNumPromotions:SetText(n);
			self.m_Instance.Promotion_Flag:SetHide(false);
		end
	end;
	env.Subscribe = function() end;
	env.Unsubscribe = function() end;
	env.GetUnitFlag = function() return nil; end;
	env.Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end };
	env.Events = setmetatable({}, { __index = function() return { Add = function() end, Remove = function() end }; end });
	env.Game = { GetLocalPlayer = function() return LOCAL; end, GetCurrentGameTurn = function() return TURN; end };
	env.GameInfo = { Units = {
		[1] = { BuildCharges = 3 },					-- bâtisseur
		[2] = { BuildCharges = 1 },					-- légion romaine
		[3] = { BuildCharges = 0, MakeTradeRoute = true },	-- négociant
		[4] = { BuildCharges = 0 },					-- guerrier
	} };
	env.Players = { [LOCAL] = { GetCities = function()
		return { Members = function()
			local done = false;
			return function()
				if done then return nil; end
				done = true;
				return 1, { GetTrade = function() return { GetOutgoingRoutes = function() return routes; end }; end };
			end;
		end };
	end } };
	env.Civ6Proxy_FinRoute = function(op, oid, dp, did, tp, tid)
		local r = finRoute[tid];
		if r == nil then return nil; end
		return r[1], r[2];
	end;
	env.Game.GetTradeManager = function() return { GetTradeRoutePath = function() return chemin; end }; end;
	env.Map = { GetPlotIndex = function(x, y) return x; end };
	local chunk = assert(load(("local BGD_Charges, BGD_Routes = %s, %s;\n"):format(tostring(charges), tostring(routesOn)) .. BLOCK, "@drapeaux.lua", "t", env));
	chunk();
	return env;
end

local function Flag(env, u)
	local inst = NewInstance();
	local self = { m_Instance = inst, unit = u, GetUnit = function() return {
		GetUnitType = function() return u.type; end,
		GetBuildCharges = function() return u.charges or 0; end,
		GetOwner = function() return u.owner or LOCAL; end,
		GetID = function() return u.id; end,
		GetX = function() return positions[u.id] or -1; end,
		GetY = function() return 0; end,
	}; end };
	env.UnitFlag.UpdatePromotions(self);
	return inst.state;
end

local failures = 0;
local function Check(cond, what)
	if not cond then failures = failures + 1; print("ÉCHEC : " .. what); end
end

local env = Run(true, true);
local s = Flag(env, { type = 1, charges = 2, id = 1 });
Check(not s.hidden and s.text == "2", "bâtisseur : 2 charges");
s = Flag(env, { type = 2, charges = 1, promotions = 2, id = 2 });
Check(not s.hidden and s.text == "1", "légion : la charge d'abord");
s = Flag(env, { type = 2, charges = 0, promotions = 2, id = 3 });
Check(not s.hidden and s.text == 2, "légion sans charge : ses promotions");
s = Flag(env, { type = 4, promotions = 0, id = 4 });
Check(s.hidden, "guerrier sans promotion : rien");
s = Flag(env, { type = 1, charges = 3, owner = 5, id = 5 });
Check(not s.hidden and s.text == "3", "bâtisseur étranger : charges montrées, comme le mod");

routes = { { TraderUnitID = 10, TraderUnitPlayer = LOCAL, OriginCityPlayer = LOCAL, OriginCityID = 1, DestinationCityPlayer = 3, DestinationCityID = 7 } };
-- Sans trajet lisible : la durée minimale seule.
finRoute[10] = { 52, 32 };
s = Flag(env, { type = 3, id = 10 });
Check(not s.hidden and s.text == "12" and s.tip:find("12 tour"), "négociant : 12 tours restants");
finRoute[10] = { 30, 10 };
s = Flag(env, { type = 3, id = 10 });
Check(not s.hidden and s.text == "0", "route échue : 0");
-- Trajet de 5 pas (cases 100 à 105), aller-retour de 10 tours. Départ au tour 15 : au tour 40, phase 5
-- (à destination) ; le négociant, à la case 105, rentre en 5 tours, au tour 45 >= fin 35.
chemin = { 100, 101, 102, 103, 104, 105 };
positions[10] = 105;
finRoute[10] = { 35, 15 };
s = Flag(env, { type = 3, id = 10 });
Check(s.text == "5", "retour après la durée minimale : " .. tostring(s.text));
-- Phase 7 (départ 13) : au retour, à la case 102, rentre en 2 tours.
positions[10] = 102;
finRoute[10] = { 33, 13 };
s = Flag(env, { type = 3, id = 10 });
Check(s.text == "2", "au retour : " .. tostring(s.text));
-- Phase 2 (départ 18) : à l'aller, à la case 102, rentre en 8 tours, au tour 48 ; la fin minimale
-- est au tour 55 : un aller-retour de plus, 18 tours.
finRoute[10] = { 55, 18 };
s = Flag(env, { type = 3, id = 10 });
Check(s.text == "18", "aller, un tour de plus : " .. tostring(s.text));
-- Négociant hors du trajet : la phase seule (départ 33, phase 7 : 3 tours, fin 45 : 3 + 10 = 13).
positions[10] = 999;
finRoute[10] = { 45, 33 };
s = Flag(env, { type = 3, id = 10 });
Check(s.text == "13", "hors du trajet : " .. tostring(s.text));
-- L'exemple du jeu : deux villes à 4 cases (aller-retour de 8 tours), durée minimale de 15 tours
-- (la vitesse en ligne de nos parties : départ 89, fin 104 au journal). Le premier retour, au tour
-- 8, vient avant la fin minimale : la route expire au second, 16 tours après le départ.
chemin = { 200, 201, 202, 203, 204 };
local function Exemple(tourCourant, case)
	TURN = tourCourant;
	positions[10] = case;
	finRoute[10] = { 15, 0 };
	return Flag(env, { type = 3, id = 10 }).text;
end
Check(Exemple(0, 200) == "16", "4 cases, au départ : " .. tostring(Exemple(0, 200)));
Check(Exemple(3, 203) == "13", "4 cases, tour 3 (aller) : " .. tostring(Exemple(3, 203)));
Check(Exemple(6, 202) == "10", "4 cases, tour 6 (premier retour) : " .. tostring(Exemple(6, 202)));
Check(Exemple(8, 200) == "8", "4 cases, tour 8 (chez lui, avant la fin) : " .. tostring(Exemple(8, 200)));
Check(Exemple(13, 203) == "3", "4 cases, tour 13 (second retour) : " .. tostring(Exemple(13, 203)));
Check(Exemple(16, 200) == "0", "4 cases, tour 16 : " .. tostring(Exemple(16, 200)));
TURN = 40;
chemin = nil;
positions[10] = nil;
s = Flag(env, { type = 3, id = 11 });
Check(s.hidden, "négociant sans route : rien");
finRoute[10] = nil;
s = Flag(env, { type = 3, id = 10 });
Check(s.hidden, "fin inconnue du proxy : rien");
finRoute[10] = { 52, 32 };
s = Flag(env, { type = 3, id = 10, owner = 5 });
Check(s.hidden, "négociant étranger : rien");

env = Run(false, true);
s = Flag(env, { type = 1, charges = 2, id = 1 });
Check(s.hidden, "charges coupées : rien");
s = Flag(env, { type = 3, id = 10 });
Check(not s.hidden and s.text == "12", "routes seules");

env = Run(true, false);
s = Flag(env, { type = 3, id = 10 });
Check(s.hidden, "routes coupées : rien");

if failures == 0 then print("ok : drapeaux (charges et routes)"); end
os.exit(failures == 0 and 0 or 1);
