-- Off-game test of src/ui_logement.lua (early housing warning on the city banner):
--   lua proxy/tests/test_ui_logement.lua
-- A CityBanner reduced to UpdateDetails, as in CityBannerManager.lua (XP2): the effect icons come
-- from an instance manager that reuses its instances in order after ResetInstances, and the
-- game's SetDetailIcon resets neither the colour nor the size of an icon.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_logement.lua");

local LOCAL, AUTRE = 0, 1;
local ORANGE, BLANC = 0xFF3492C8, 0xFFFFFFFF;

local function NewMachine(sansBandeau)
	local m = {
		city = { owner = LOCAL, housing = 6, pop = 3, mod = 1, siege = false, power = false, modifier = true },
		resizes = 0, sized = 0, printed = {},
	};
	local env = setmetatable({}, { __index = _G });
	env.print = function(s) m.printed[#m.printed + 1] = tostring(s); end;
	env.Mouse = { eLClick = "L" };
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.UI = { GetColorValueFromHexLiteral = function(v) return v; end };
	env.Locale = { Lookup = function(k, ...)
		local a = { ... };
		return #a > 0 and (k .. ":" .. table.concat(a, ",")) or k;
	end };

	local function Icon()
		local c = { x = 20, y = 20, color = BLANC, hidden = true };
		function c:SetSizeVal(x, y) self.x, self.y = x, y; end
		function c:GetSizeX() return self.x; end
		function c:GetSizeY() return self.y; end
		function c:SetIcon(name, size) self.icon, self.iconSize = name, size; end
		function c:SetColor(v)
			if self.destroyed then error("destroyed control"); end
			self.color = v;
		end
		function c:SetHide(h) self.hidden = h; end
		function c:SetToolTipString(s) self.tip = s; end
		return c;
	end
	local function Button()
		local b = { cb = {} };
		function b:RegisterCallback(ev, f) self.cb[ev] = f; end
		function b:ClearCallback(ev) self.cb[ev] = nil; end
		return b;
	end
	local function Manager()
		local im = { all = {}, used = 0 };
		function im:ResetInstances() self.used = 0; end
		function im:GetInstance()
			self.used = self.used + 1;
			if self.all[self.used] == nil then
				self.all[self.used] = { Icon = Icon(), Button = Button() };
			end
			return self.all[self.used];
		end
		return im;
	end

	-- CityBannerManager.lua.
	local function SetDetailIcon(instance, icon, tooltip)
		instance.Icon:SetHide(icon == nil);
		if icon then instance.Icon:SetIcon(icon); end
		instance.Icon:SetToolTipString(tooltip and tooltip or "");
		if instance.Button ~= nil then instance.Button:ClearCallback("L"); end
	end
	local CityBanner = {};
	function CityBanner:GetCity() return self.city; end
	function CityBanner:Resize() m.resizes = m.resizes + 1; end
	function CityBanner:UpdateDetails()
		local c, im = m.city, self.m_DetailEffectsIM;
		im:ResetInstances();
		if c.siege then
			SetDetailIcon(im:GetInstance(), "ICON_CITY_EFFECTS_SIEGE", "siege");
		end
		if c.owner == LOCAL and c.housing < c.pop then
			SetDetailIcon(im:GetInstance(), "ICON_CITY_EFFECTS_HOUSING", "LOC_CITY_BANNER_HOUSING_INSUFFICIENT");
		end
		if c.power then
			local inst = im:GetInstance();
			SetDetailIcon(inst, "Power", "power");
			inst.Button:RegisterCallback("L", function() end);
		end
		self.m_Instance.CityDetailsEffects:CalculateSize();
		self:Resize();
	end
	if not sansBandeau then env.CityBanner = CityBanner; end

	local growth = {
		GetHousing = function() return m.city.housing; end,
		GetHousingGrowthModifier = function() return m.city.mod; end,
	};
	local city = {
		GetOwner = function() return m.city.owner; end,
		GetPopulation = function() return m.city.pop; end,
		GetGrowth = function()
			if m.city.modifier then return growth; end
			return { GetHousing = growth.GetHousing };
		end,
	};
	function m.Load()
		assert(load(BLOCK, "bloc", "t", env))();
	end
	function m.Banner()
		local im = Manager();
		local stack = { CalculateSize = function() m.sized = m.sized + 1; end };
		local banner = setmetatable({ city = city, m_DetailEffectsIM = im, m_Instance = { CityDetailsEffects = stack } }, { __index = CityBanner });
		return banner, im;
	end
	-- Icons shown after the last update, in order.
	function m.Shown(im)
		local t = {};
		for i = 1, im.used do t[i] = im.all[i].Icon.icon; end
		return table.concat(t, " ");
	end
	m.env = env;
	m.Load();
	return m;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

local TIP50 = "[NEWLINE]LOC_HUD_CITY_POPULATION_GROWTH_SLOWED:50";
local TIP75 = "[NEWLINE]LOC_HUD_CITY_POPULATION_GROWTH_SLOWED:75";

-- The three levels the game leaves without an icon, and the one it shows itself.
do
	local m = NewMachine();
	local banner, im = m.Banner();
	banner:UpdateDetails();
	Check("normal growth: no icon, banner sized by the game only", im.used == 0 and m.resizes == 1 and m.sized == 1);

	m.city.pop, m.city.mod = 5, 0.5;
	banner:UpdateDetails();
	local icon = im.all[1] and im.all[1].Icon or {};
	Check("one below the cap: orange house", im.used == 1 and icon.icon == "ICON_HOUSING" and icon.color == ORANGE and icon.hidden == false);
	Check("one below the cap: 22 px icon of the Stats atlas in a 22 px slot", icon.iconSize == 22 and icon.x == 22 and icon.y == 22);
	Check("one below the cap: tooltip", icon.tip == "[ICON_Citizen] 5 / 6 [ICON_Housing]" .. TIP50);
	Check("one below the cap: banner sized again with the icon", m.resizes == 3 and m.sized == 3);

	m.city.pop, m.city.mod = 6, 0.25;
	banner:UpdateDetails();
	Check("at the cap: the game's red house, untinted, in the 20 px slot", im.used == 1 and icon.icon == "ICON_CITY_EFFECTS_HOUSING"
		and icon.iconSize == nil and icon.color == BLANC and icon.x == 20 and icon.y == 20 and icon.hidden == false);
	Check("at the cap: tooltip", icon.tip == "[ICON_Citizen] 6 / 6 [ICON_Housing]" .. TIP75);

	m.city.pop, m.city.mod = 5, 0.5;
	banner:UpdateDetails();
	Check("back to one below the cap: orange house again", icon.icon == "ICON_HOUSING" and icon.color == ORANGE and icon.x == 22);
	m.city.mod = 0.25;

	m.city.pop = 7;
	banner:UpdateDetails();
	Check("over the cap: the game's red icon only, as the game left it",
		m.Shown(im) == "ICON_CITY_EFFECTS_HOUSING" and icon.color == BLANC and icon.x == 20 and icon.y == 20);

	m.city.pop, m.city.mod = 3, 1;
	banner:UpdateDetails();
	Check("back to normal growth: nothing shown", im.used == 0 and icon.color == BLANC);
	Check("no failure written", #m.printed == 0);
end

-- Half points of housing (farms): the level follows the game's modifier, the tooltip keeps them.
do
	local m = NewMachine();
	local banner, im = m.Banner();
	m.city.housing, m.city.pop, m.city.mod = 6.5, 5, 0.5;
	banner:UpdateDetails();
	Check("fractional housing: orange, « 5 / 6.5 »", im.all[1].Icon.color == ORANGE
		and im.all[1].Icon.tip == "[ICON_Citizen] 5 / 6.5 [ICON_Housing]" .. TIP50);
	-- A mod with other thresholds: only the modifier counts.
	m.city.housing, m.city.pop, m.city.mod = 9, 5, 0.5;
	banner:UpdateDetails();
	Check("modifier at 50% with more housing left (modded thresholds): orange", im.used == 1 and im.all[1].Icon.color == ORANGE);
	m.city.mod = 0.75;
	banner:UpdateDetails();
	Check("modifier above 50%: nothing", im.used == 0);
end

-- Instances are reused by the game's other icons.
do
	local m = NewMachine();
	local banner, im = m.Banner();
	m.city.pop, m.city.mod = 5, 0.5;
	banner:UpdateDetails();
	m.city.siege, m.city.pop, m.city.mod = true, 3, 1;
	banner:UpdateDetails();
	local first = im.all[1].Icon;
	Check("instance reused by the siege icon: colour and size put back",
		m.Shown(im) == "ICON_CITY_EFFECTS_SIEGE" and first.color == BLANC and first.x == 20 and first.y == 20);

	m.city.pop, m.city.mod = 5, 0.5;
	banner:UpdateDetails();
	local second = im.all[2].Icon;
	Check("siege and warning: house after the game's icons",
		m.Shown(im) == "ICON_CITY_EFFECTS_SIEGE ICON_HOUSING" and first.color == BLANC and second.color == ORANGE and second.x == 22);

	m.city.siege = false;
	banner:UpdateDetails();
	Check("siege lifted: house in the first instance, second one put back",
		m.Shown(im) == "ICON_HOUSING" and first.color == ORANGE and first.x == 22 and second.color == BLANC and second.x == 20);

	-- The power icon leaves a click on its instance.
	m.city.power, m.city.pop, m.city.mod = true, 3, 1;
	banner:UpdateDetails();
	Check("power icon: its click, white", im.all[1].Button.cb.L ~= nil and first.color == BLANC);
	m.city.power, m.city.pop, m.city.mod = false, 5, 0.5;
	banner:UpdateDetails();
	Check("house in the instance of the power icon: no click left", m.Shown(im) == "ICON_HOUSING" and im.all[1].Button.cb.L == nil);
	Check("no failure written", #m.printed == 0);
end

-- Two banners keep their own state.
do
	local m = NewMachine();
	local a, ima = m.Banner();
	local b, imb = m.Banner();
	m.city.pop, m.city.mod = 5, 0.5;
	a:UpdateDetails();
	m.city.pop, m.city.mod = 3, 1;
	b:UpdateDetails();
	Check("two banners: the first keeps its house", ima.all[1].Icon.color == ORANGE and imb.used == 0);
	a:UpdateDetails();
	Check("two banners: the first put back on its own update", ima.used == 0 and ima.all[1].Icon.color == BLANC);
end

-- Cities of other players, missing API, destroyed instances.
do
	local m = NewMachine();
	local banner, im = m.Banner();
	m.city.owner, m.city.pop, m.city.mod = AUTRE, 5, 0.5;
	banner:UpdateDetails();
	Check("another player's city: nothing", im.used == 0);

	m.city.owner, m.city.modifier = LOCAL, false;
	banner:UpdateDetails();
	Check("no GetHousingGrowthModifier: nothing, no error", im.used == 0);

	m.city.modifier = true;
	banner.GetCity = function() return nil; end;
	banner:UpdateDetails();
	Check("banner without a city: nothing", im.used == 0);
	banner.GetCity = nil;

	banner:UpdateDetails();
	Check("warning shown", im.used == 1 and im.all[1].Icon.color == ORANGE);
	-- DestroyInstances: the control kept from the last update is gone, the manager starts again.
	im.all[1].Icon.destroyed = true;
	im.all = {};
	local ok = pcall(banner.UpdateDetails, banner);
	Check("instance destroyed since the last update: no error, new house", ok and im.used == 1 and im.all[1].Icon.color == ORANGE);
	Check("no failure written", #m.printed == 0);
end

-- Loaded twice (two blocks of the same context), and without CityBanner.
do
	local m = NewMachine();
	m.Load();
	local banner, im = m.Banner();
	m.city.pop, m.city.mod = 5, 0.5;
	banner:UpdateDetails();
	Check("loaded twice: wrapped once", im.used == 1 and m.resizes == 2);

	local vide = NewMachine(true);
	Check("without CityBanner: nothing done, no failure", vide.env.BGD_Logement_Etat == nil and #vide.printed == 0);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
