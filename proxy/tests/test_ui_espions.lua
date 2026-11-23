-- Test hors jeu de src/ui_espions.lua (chance de succès des espions avec l'évasion) :
--   lua proxy/tests/test_ui_espions.lua [EspionageSupport.lua]
-- Les valeurs de référence viennent de analyses/espions/proba.py (« affichage corrigé ») : même
-- table 3d6, même troncature sur 8 bits que GetResultProbability. Si le EspionageSupport.lua du jeu
-- est présent, on le charge suivi du bloc, comme le proxy (`patch_espions`), et on lit le chiffre
-- que RefreshMissionStats écrit ; sinon cette partie est sautée.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_espions.lua");
local SUPPORT_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Assets\UI\EspionageSupport.lua]];

local failures = 0;
local function Check(cond, msg)
	if not cond then
		failures = failures + 1;
		print("ÉCHEC : " .. msg);
	end
end

-- Issues de GameCore (FUN_180374180), comme proba.py.
local DES = { [0] = 0, 0, 0, 1, 3, 6, 10, 15, 21, 25, 27, 27, 25, 21, 15, 10, 6, 3, 1 };
local function F(lo, hi)
	local n = 0;
	for i = math.max(lo, 0), math.min(hi, 18) do
		n = n + DES[i];
	end
	return n;
end
local function Cl(v, a, b) return math.max(a, math.min(b, v)); end
local function Q8(w) return math.floor(w * 256 / 216) / 256; end
local function Issues(N)
	local s5, s4, s3, s2, s1 = Cl(N, 8, 18), Cl(N - 2, 7, 17), Cl(N - 3, 6, 16), Cl(N - 5, 5, 15), Cl(N - 7, 4, 14);
	return {
		ESPIONAGE_KILLED = Q8(F(3, s1 - 1)),
		ESPIONAGE_CAPTURED = Q8(F(s1, s2 - 1)),
		ESPIONAGE_FAIL_MUST_ESCAPE = Q8(F(s2, s3 - 1)),
		ESPIONAGE_FAIL_UNDETECTED = Q8(F(s3, s4 - 1)),
		ESPIONAGE_SUCCESS_MUST_ESCAPE = Q8(F(s4, s5 - 1)),
		ESPIONAGE_SUCCESS_UNDETECTED = Q8(F(s5, 18)),
	};
end

-- Une table de GameInfo : indexable par clé ou par Index, et appelable pour itérer.
local function Table(rows, key)
	local t = {};
	for i, r in ipairs(rows) do
		r.Index = i - 1;
		t[r[key]] = r;
		t[i - 1] = r;
	end
	return setmetatable(t, { __call = function()
		local i = 0;
		return function()
			i = i + 1;
			return rows[i];
		end;
	end });
end

local function NewEnv()
	local env = setmetatable({}, { __index = _G });
	env.printed = {};
	env.print = function(s) env.printed[#env.printed + 1] = tostring(s); end;
	env.GameInfo = {
		GlobalParameters = Table({
			{ Name = "ESPIONAGE_ESCAPE_BASE_CHANCE", Value = "10" },
			{ Name = "ESPIONAGE_ESCAPE_LEVEL_BOOST", Value = "1" },
			{ Name = "ESPIONAGE_ESCAPE_POLICE_CORRECT_MODIFIER", Value = "-4" },
		}, "Name"),
		Districts = Table({
			{ DistrictType = "DISTRICT_CITY_CENTER", TravelTime = 4 },
			{ DistrictType = "DISTRICT_HARBOR", TravelTime = 2 },
			{ DistrictType = "DISTRICT_AERODROME", TravelTime = 1 },
			{ DistrictType = "DISTRICT_COMMERCIAL_HUB", TravelTime = 3 },
			{ DistrictType = "DISTRICT_SUGUBA", TravelTime = 3 },
			{ DistrictType = "DISTRICT_CAMPUS" },
		}, "DistrictType"),
		DistrictReplaces = { DISTRICT_SUGUBA = { ReplacesDistrictType = "DISTRICT_COMMERCIAL_HUB" } },
		DynamicModifiers = Table({
			{ ModifierType = "MODIFIER_PLAYER_UNIT_ESCAPE_BOOST", EffectType = "EFFECT_ADJUST_UNIT_ESCAPE_BOOST" },
			{ ModifierType = "MODIFIER_OTHER", EffectType = "EFFECT_OTHER" },
		}, "ModifierType"),
		Modifiers = Table({
			{ ModifierId = "SPY_ACE_DRIVER", ModifierType = "MODIFIER_PLAYER_UNIT_ESCAPE_BOOST" },
			{ ModifierId = "GOUVERNEUR_FUITE", ModifierType = "MODIFIER_PLAYER_UNIT_ESCAPE_BOOST" },
			{ ModifierId = "AUTRE", ModifierType = "MODIFIER_OTHER" },
		}, "ModifierId"),
		ModifierArguments = Table({
			{ ModifierId = "SPY_ACE_DRIVER", Name = "Amount", Value = "4" },
			{ ModifierId = "GOUVERNEUR_FUITE", Name = "Amount", Value = "2" },
			{ ModifierId = "AUTRE", Name = "Amount", Value = "9" },
		}, "ModifierId"),
		UnitPromotions = Table({
			{ UnitPromotionType = "PROMOTION_SPY_LINGUIST" },
			{ UnitPromotionType = "PROMOTION_SPY_ACE_DRIVER" },
		}, "UnitPromotionType"),
		UnitPromotionModifiers = Table({
			{ UnitPromotionType = "PROMOTION_SPY_ACE_DRIVER", ModifierId = "SPY_ACE_DRIVER" },
		}, "ModifierId"),
	};
	env.calls = 0;
	env.UnitManager = {
		GetResultProbability = function(op, spy, plot)
			env.calls = env.calls + 1;
			return Issues(op - (spy.level - 1));
		end,
		GetTimeToComplete = function() return 4; end,
	};
	env.Cities = { GetPlotPurchaseCity = function(plot) return plot.city; end };
	return env;
end

local function Spy(level, promotions)
	local s = { level = level, promotions = promotions or {} };
	function s:GetExperience()
		local spy = self;
		return {
			GetLevel = function() return spy.level; end,
			HasPromotion = function(_, index) return spy.promotions[index] == true; end,
		};
	end
	return s;
end

local function City(districts)
	local list = {};
	for _, d in ipairs(districts) do
		local name, complete = d:match("^(.-)(%??)$");
		list[#list + 1] = {
			GetType = function() return name; end,
			IsComplete = function() return complete ~= "?"; end,
		};
	end
	return { GetDistricts = function() return { Members = function()
		local i = 0;
		return function()
			i = i + 1;
			if list[i] then return i, list[i]; end
		end;
	end }; end };
end

local function Load(env, src)
	local chunk = assert(load(src, "=EspionageSupport.lua", "t", env));
	chunk();
end

local CENTRE = City({ "DISTRICT_CITY_CENTER" });                                                -- W = 1
local TROIS = City({ "DISTRICT_CITY_CENTER", "DISTRICT_HARBOR", "DISTRICT_AERODROME?" });         -- W = 1 + 3
local SUGUBA = City({ "DISTRICT_CITY_CENTER", "DISTRICT_SUGUBA" });                               -- W = 1 + 2 + 2
local QUATRE = City({ "DISTRICT_CITY_CENTER", "DISTRICT_HARBOR", "DISTRICT_AERODROME", "DISTRICT_COMMERCIAL_HUB", "DISTRICT_CAMPUS" }); -- W = 1+3+4+2

local function Affiche(env, base, spy, city)
	local r = env.UnitManager.GetResultProbability(base, spy, { city = city });
	return r.ESPIONAGE_SUCCESS_UNDETECTED + r.ESPIONAGE_SUCCESS_MUST_ESCAPE, r;
end
local function Pct(x) return string.format("%.1f", 100 * x); end

-- 1. Sorties de la police.
do
	local env = NewEnv();
	Load(env, BLOCK);
	local function W(city) local m, t = env.BGD_Espions_Sorties(city); return m .. "/" .. t; end
	Check(W(CENTRE) == "1/1", "1 : centre seul, " .. W(CENTRE));
	Check(W(TROIS) == "1/4", "1 : aérodrome inachevé non compté, " .. W(TROIS));
	Check(W(SUGUBA) == "1/5", "1 : quartier unique compté pour lui et pour celui qu'il remplace, " .. W(SUGUBA));
	Check(W(QUATRE) == "1/10", "1 : quatre sorties, " .. W(QUATRE));
end

-- 2. Chiffres de proba.py, mission de base 14 : L | W=10 W=3 W=1 | As du volant W=1.
do
	local REF = {
		{ "28.2", "25.9", "19.4", "29.2" },
		{ "42.3", "39.6", "32.0", "43.4" },
		{ "57.3", "54.6", "46.9", "58.4" },
		{ "70.7", "68.4", "61.9", "71.6" },
	};
	local W3 = City({ "DISTRICT_CITY_CENTER", "DISTRICT_COMMERCIAL_HUB" });
	local env = NewEnv();
	Load(env, BLOCK);
	for L = 1, 4 do
		local got = {
			Pct(Affiche(env, 14, Spy(L), QUATRE)),
			Pct(Affiche(env, 14, Spy(L), W3)),
			Pct(Affiche(env, 14, Spy(L), CENTRE)),
			Pct(Affiche(env, 14, Spy(L, { [1] = true }), CENTRE)),
		};
		Check(table.concat(got, " ") == table.concat(REF[L], " "),
			"2 : niveau " .. L .. ", " .. table.concat(got, " ") .. " au lieu de " .. table.concat(REF[L], " "));
	end
	Check(#env.printed == 1 and env.printed[1]:find("GOUVERNEUR_FUITE", 1, true) ~= nil,
		"2 : bonus d'évasion hors promotion signalé une fois, " .. table.concat(env.printed, " | "));
end

-- 3. Le reste de la table est intact, la table du jeu n'est pas modifiée, UnitManager reste entier.
do
	local env = NewEnv();
	local original = env.UnitManager;
	Load(env, BLOCK);
	local s, r = Affiche(env, 14, Spy(1), CENTRE);
	local brut = Issues(14);
	Check(r.ESPIONAGE_SUCCESS_UNDETECTED == brut.ESPIONAGE_SUCCESS_UNDETECTED
		and r.ESPIONAGE_FAIL_UNDETECTED == brut.ESPIONAGE_FAIL_UNDETECTED
		and r.ESPIONAGE_CAPTURED == brut.ESPIONAGE_CAPTURED, "3 : autres issues inchangées");
	Check(r.ESPIONAGE_SUCCESS_MUST_ESCAPE < brut.ESPIONAGE_SUCCESS_MUST_ESCAPE, "3 : succès avec fuite réduit");
	Check(env.UnitManager ~= original and original.GetResultProbability ~= env.UnitManager.GetResultProbability,
		"3 : la table du jeu n'est pas touchée");
	Check(env.UnitManager.GetTimeToComplete() == 4, "3 : le reste de UnitManager passe à l'original");
	-- Rechargé dans le même contexte (second include) : pas d'enveloppe en double.
	local avant = env.UnitManager;
	Load(env, BLOCK);
	Check(env.UnitManager == avant, "3 : bloc rechargé sans double enveloppe");
	-- Sans ville (case hors de toute ville) : chiffre du jeu.
	local r2 = env.UnitManager.GetResultProbability(14, Spy(1), { city = nil });
	Check(r2.ESPIONAGE_SUCCESS_MUST_ESCAPE == brut.ESPIONAGE_SUCCESS_MUST_ESCAPE, "3 : sans ville, chiffre du jeu");
end

-- 4. Erreur dans le calcul : chiffre du jeu, une ligne dans Lua.log.
do
	local env = NewEnv();
	Load(env, BLOCK);
	local casse = { GetDistricts = function() error("boum"); end };
	local s1 = Affiche(env, 14, Spy(1), casse);
	local s2 = Affiche(env, 14, Spy(1), casse);
	local brut = Issues(14);
	Check(s1 == brut.ESPIONAGE_SUCCESS_UNDETECTED + brut.ESPIONAGE_SUCCESS_MUST_ESCAPE, "4 : chiffre du jeu");
	local n = 0;
	for _, p in ipairs(env.printed) do
		if p:find("Espions echec", 1, true) then n = n + 1; end
	end
	Check(n == 1, "4 : une seule ligne d'échec, " .. n);
end

-- 5. Avec le vrai EspionageSupport.lua : le libellé du choix de mission.
local okSupport, SUPPORT = pcall(ReadFile, SUPPORT_PATH);
if not okSupport then
	print("EspionageSupport.lua introuvable (" .. SUPPORT_PATH .. "), partie 5 sautée");
else
	-- HavokScript accepte des annotations de type que Lua 5.4 refuse : on les retire.
	for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
		SUPPORT = SUPPORT:gsub("%s*:%s*" .. t .. "%f[^%w_(]", "");
	end
	local env = NewEnv();
	env.UnitOperationTypes = { SPY_COUNTERSPY = "COUNTER" };
	Load(env, SUPPORT .. "\r\n" .. BLOCK);
	local function Label()
		local l = {};
		function l:SetText(t) self.text = t; end
		function l:SetColorByName(c) self.color = c; end
		function l:SetHide(h) self.hidden = h; end
		return l;
	end
	local parent = { TurnsToCompleteLabel = Label(), ProbabilityLabel = Label(), ProbabilityGrid = Label(),
		MissionDistrictName = Label(), MissionDistrictIcon = { SetIcon = function() end } };
	env.Locale = { Lookup = function(k) return k; end };
	local ok, err = pcall(env.RefreshMissionStats, parent, { Index = 14, Hash = "STEAL" }, nil, Spy(1), nil, { city = CENTRE });
	Check(ok, "5 : RefreshMissionStats, " .. tostring(err));
	Check(parent.ProbabilityLabel.text == "19%", "5 : libellé " .. tostring(parent.ProbabilityLabel.text));
	Check(parent.ProbabilityLabel.color == "OperationChance_Red", "5 : couleur " .. tostring(parent.ProbabilityLabel.color));
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("ui_espions : tout est bon");
