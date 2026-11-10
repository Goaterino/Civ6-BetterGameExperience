-- Test hors jeu de src/ui_banniere_annuler.lua (BET-12) et src/ui_debordement.lua (BET-21) :
--   lua proxy/tests/test_ui_banniere.lua
-- Un CityBanner réduit à UpdateProduction, comme dans CityBannerManager.lua (XP2 ou BSM) : une
-- instance de production avec son bouton et ses tours restants ; le proxy imité par
-- BGD_Debordement.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local TEXTES = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua");
local ANNULER = ReadFile(dir .. "/../src/ui_banniere_annuler.lua");
local DEBORDEMENT = TEXTES .. ReadFile(dir .. "/../src/ui_debordement.lua");

local LOCAL, AUTRE = 0, 1;

local function NewMachine(blocks, reserves)
	local m = { ops = {}, queue = { [LOCAL] = 1, [AUTRE] = 1 }, sounds = 0 };
	local env = setmetatable({}, { __index = _G });
	env.Mouse = { eLClick = "L", eRClick = "R" };
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end };
	env.UI = { PlaySound = function() m.sounds = m.sounds + 1; end };
	env.CityOperationTypes = { PARAM_INSERT_MODE = "mode", VALUE_REMOVE_AT = "remove", PARAM_QUEUE_LOCATION = "loc", BUILD = "build" };
	env.CityManager = { RequestOperation = function(city, op, p) m.ops[#m.ops + 1] = { city = city:GetID(), op = op, mode = p.mode, loc = p.loc }; end };
	local function City(owner, id)
		return {
			GetOwner = function() return owner; end,
			GetID = function() return id; end,
			GetBuildQueue = function() return {
				GetSize = function() return m.queue[owner]; end,
				GetCurrentProductionTypeHash = function() return m.queue[owner] > 0 and 1234 or 0; end,
			}; end,
		};
	end
	m.cities = { [LOCAL] = City(LOCAL, 65536), [AUTRE] = City(AUTRE, 131072) };
	env.Players = {
		[LOCAL] = { GetCities = function() return { FindID = function(_, id) return id == 65536 and m.cities[LOCAL] or nil; end }; end },
		[AUTRE] = { GetCities = function() return { FindID = function() return m.cities[AUTRE]; end }; end },
	};
	if reserves ~= nil then
		env.BGD_Debordement = function(p, c) return reserves[p .. ":" .. c]; end;
	end
	-- CityBannerManager.lua : l'original remplit l'instance (tours, infobulle) et le clic gauche.
	local function Control()
		local c = { cb = {} };
		function c:RegisterCallback(ev, f) self.cb[ev] = f; end
		function c:ClearCallback(ev) self.cb[ev] = nil; end
		function c:SetToolTipString(s) self.tip = s; end
		function c:SetText(s) self.text = s; end
		function c:GetToolTipString() return self.tip or ""; end
		return c;
	end
	local CityBanner = {};
	function CityBanner:UpdateProduction(pCity)
		local inst = self.inst;
		if pCity:GetOwner() == LOCAL then inst.Button:RegisterCallback("L", function() end); end
		if pCity:GetBuildQueue():GetCurrentProductionTypeHash() ~= 0 then
			inst.TurnsLeft:SetText(5);
			inst.Button:SetToolTipString("producing");
		else
			inst.Button:SetToolTipString("LOC_CITY_BANNER_NO_PRODUCTION");
		end
	end
	env.CityBanner = CityBanner;
	for _, b in ipairs(blocks) do
		assert(load(b, "bloc", "t", env))();
	end
	function m.Banner(owner)
		local inst = { Button = Control(), TurnsLeft = Control() };
		local banner = setmetatable({ inst = inst, m_StatProductionIM = { GetAllocatedInstance = function() return inst; end } }, { __index = CityBanner });
		banner:UpdateProduction(m.cities[owner]);
		return banner, inst;
	end
	m.env = env;
	return m;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- BET-12 : clic droit sur la production du bandeau.
do
	local m = NewMachine({ ANNULER });
	local _, inst = m.Banner(LOCAL);
	Check("annuler : clic gauche d'origine gardé", inst.Button.cb.L ~= nil);
	inst.Button.cb.R();
	Check("annuler : retrait de l'élément en tête de la ville du bandeau",
		#m.ops == 1 and m.ops[1].city == 65536 and m.ops[1].mode == "remove" and m.ops[1].loc == 0 and m.ops[1].op == "build");
	m.queue[LOCAL] = 0;
	inst.Button.cb.R();
	Check("annuler : file vide, aucune commande", #m.ops == 1);
	local _, autre = m.Banner(AUTRE);
	Check("annuler : pas de clic droit sur la ville d'un autre joueur", autre.Button.cb.R == nil);
end

-- BET-21 : production en réserve sur le bandeau.
do
	local m = NewMachine({ DEBORDEMENT }, { ["0:65536"] = 12.75, ["1:131072"] = 30 });
	local banner, inst = m.Banner(LOCAL);
	Check("réserve : file pleine, tours d'origine", inst.TurnsLeft.text == 5);
	Check("réserve : file pleine, ligne ajoutée à l'infobulle", inst.Button.tip:find("producing", 1, true) == 1
		and inst.Button.tip:find("+12 [ICON_Production] en réserve", 1, true) ~= nil);
	m.queue[LOCAL] = 0;
	banner:UpdateProduction(m.cities[LOCAL]);
	Check("réserve : file vide, « +12 »", inst.TurnsLeft.text == "+12");
	Check("réserve : infobulle complétée", inst.Button.tip:find("LOC_CITY_BANNER_NO_PRODUCTION", 1, true) == 1
		and inst.Button.tip:find("+12 [ICON_Production] en réserve", 1, true) ~= nil);
	m.env.BGD_Debordement = function() return 0.5; end;
	banner:UpdateProduction(m.cities[LOCAL]);
	Check("réserve : moins d'un point, rien d'affiché", inst.TurnsLeft.text == "");
	m.queue[AUTRE] = 0;
	local _, autre = m.Banner(AUTRE);
	Check("réserve : rien pour la ville d'un autre joueur", autre.TurnsLeft.text == nil);
end

-- Sans la fonction native (relevé non posé) : le bloc ne change rien.
do
	local m = NewMachine({ DEBORDEMENT });
	m.queue[LOCAL] = 0;
	local _, inst = m.Banner(LOCAL);
	Check("sans BGD_Debordement : bandeau inchangé", inst.TurnsLeft.text == nil and inst.Button.tip == "LOC_CITY_BANNER_NO_PRODUCTION");
end

-- Les deux blocs ensemble, dans l'ordre du proxy.
do
	local m = NewMachine({ ANNULER, DEBORDEMENT }, { ["0:65536"] = 7 });
	m.queue[LOCAL] = 0;
	local _, inst = m.Banner(LOCAL);
	Check("ensemble : réserve et clic droit", inst.TurnsLeft.text == "+7" and inst.Button.cb.R ~= nil);
end

-- Panneau de production : « +N » après la progression, infobulle du cadre « aucune production ».
do
	local env = setmetatable({}, { __index = _G });
	local tip;
	local hash = 1234;
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.Locale = { GetCurrentLanguage = function() return { Type = "en_US" }; end, Lookup = function(s) return s; end };
	local progress = { text = "", tip = "" };
	function progress:SetText(s) self.text = s; end
	function progress:GetText() return self.text; end
	function progress:SetToolTipString(s) self.tip = s; end
	env.Controls = { NoProductionContainer = { SetToolTipString = function(_, s) tip = s; end }, CurrentProductionProgressString = progress };
	local city = { GetOwner = function() return LOCAL; end, GetID = function() return 3; end,
		GetBuildQueue = function() return { GetCurrentProductionTypeHash = function() return hash; end }; end };
	env.UI = { GetHeadSelectedCity = function() return city; end };
	env.BGD_Debordement = function() return 9.9; end;
	-- View remplit la progression (onglets Production et File).
	local refill = true;
	env.View = function() if refill then progress.text = "[ICON_Production]45 / 120"; end end;
	assert(load(DEBORDEMENT, "bloc", "t", env))();
	local TXT = "+9 [ICON_Production] in reserve, added to next turn's production (at most one turn of production).";
	env.View({});
	Check("panneau : « +9 » après la progression", progress.text == "[ICON_Production]45 / 120  [COLOR:ResProductionLabelCS]+9[ENDCOLOR]" and progress.tip == TXT);
	Check("panneau : infobulle du cadre en anglais", tip == TXT);
	refill = false;
	env.View({});
	Check("panneau : View sans réécriture, réserve pas répétée", progress.text == "[ICON_Production]45 / 120  [COLOR:ResProductionLabelCS]+9[ENDCOLOR]");
	env.BGD_Debordement = function() return nil; end;
	env.View({});
	Check("panneau : rien de relevé, réserve retirée", tip == "" and progress.text == "[ICON_Production]45 / 120" and progress.tip == "");
	refill = true;
	hash = 0;
	env.BGD_Debordement = function() return 9.9; end;
	env.View({});
	Check("panneau : file vide, pas de « +9 » sur l'ancienne production", progress.text == "[ICON_Production]45 / 120" and tip == TXT);
end

-- Panneau de la ville (CityPanel) : ligne ajoutée aux statistiques de la production.
do
	local env = setmetatable({}, { __index = _G });
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end };
	local stat = { text = nil };
	function stat:SetText(s) self.text = s; end
	local sized = 0;
	local stack = { CalculateSize = function() sized = sized + 1; end };
	env.Controls = { ProductionStatString = stat, ProductionDataStack = stack, ProductionDataScroll = stack };
	local city = { GetOwner = function() return LOCAL; end, GetID = function() return 3; end };
	env.UI = { GetHeadSelectedCity = function() return city; end };
	env.BGD_Debordement = function() return 21.4; end;
	local base = 0;
	env.ViewMain = function(data) base = base + 1; stat.text = data.CurrentProductionStats; end;
	assert(load(DEBORDEMENT, "bloc", "t", env))();
	env.ViewMain({ CurrentProductionStats = "120 [ICON_Production]" });
	Check("panneau de ville : ligne ajoutée", base == 1 and stat.text == "120 [ICON_Production][NEWLINE]+21 [ICON_Production] en réserve, ajoutés à la production du prochain tour (au plus un tour de production)."
		and sized == 2);
	env.BGD_Debordement = function() return 0; end;
	env.ViewMain({ CurrentProductionStats = "" });
	Check("panneau de ville : sans réserve, statistiques seules", stat.text == "");
end

-- Panneau de ville : redessiné quand la file change (ui_panneau_ville.lua).
do
	local env = setmetatable({}, { __index = _G });
	local handler;
	env.Events = { CityProductionQueueChanged = { Add = function(h) handler = h; end } };
	local calls = {};
	env.RefreshIfMatch = function(p, c) calls[#calls + 1] = p .. ":" .. c; end;
	assert(load(ReadFile(dir .. "/../src/ui_panneau_ville.lua"), "bloc", "t", env))();
	assert(load(ReadFile(dir .. "/../src/ui_panneau_ville.lua"), "bloc", "t", env))();
	handler(LOCAL, 65536);
	Check("panneau de ville : CityProductionQueueChanged redessine (une fois)", #calls == 1 and calls[1] == "0:65536");
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
