-- Off-game test of src/ui_formations.lua:
--   lua proxy/tests/test_ui_formations.lua [ProductionPanel.lua]
-- Loaded the way the proxy does it: the game's ProductionPanel.lua, then ui_production.lua, then
-- the block, in an environment that imitates the interface. The game's own PopulateUnits fills
-- the rows of a unit list; the test then reads what each button shows and what its click sends.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local PRODUCTION = ReadFile(dir .. "/../src/ui_production.lua");
local BLOCK = ReadFile(dir .. "/../src/ui_formations.lua");
-- The other blocks the proxy adds to the same file (ui_reduction_quartiers, ui_debordement).
local TEXTES = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua");
local EXTRAS = TEXTES .. ReadFile(dir .. "/../src/ui_quartiers.lua") .. "\r\n" .. TEXTES .. ReadFile(dir .. "/../src/ui_debordement.lua");
local PANEL_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Assets\UI\Panels\ProductionPanel.lua]];
local okPanel, PANEL = pcall(ReadFile, PANEL_PATH);
if not okPanel then
	print("ProductionPanel.lua introuvable (" .. PANEL_PATH .. "), test sauté");
	return;
end
-- HavokScript accepts type annotations (`local x:number`) that Lua 5.4 refuses: removed.
for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
	PANEL = PANEL:gsub("%s*:%s*" .. t .. "%f[^%w_(]", "");
end

local LOCAL = 0;
local PRODUCTION_LIST, GOLD_LIST, QUEUE_LIST = 1, 2, 4; -- LISTMODE of ProductionPanel.lua

-- Whatever the panel calls that the test does not look at: every field and every call give the
-- same object back, except numeric indexes (ipairs stops).
local Any;
Any = setmetatable({}, {
	__index = function(_, k) if type(k) == "number" then return nil; end return Any; end,
	__call = function() return Any; end,
});

-- A control: keeps what Set<X> gave it (text, tooltip, visibility, percent...) and its callbacks.
local function NewControl()
	local c = { Hide = false };
	return setmetatable(c, { __index = function(_, k)
		if k == "RegisterCallback" then return function(self, ev, f) rawset(self, "cb_" .. tostring(ev), f); end; end
		if k == "SetToolTipString" then return function(self, s) rawset(self, "tip", s); end; end
		if k == "IsHidden" then return function(self) return self.Hide; end; end
		if k == "IsSelected" then return function(self) return rawget(self, "Selected") == true; end; end
		if k == "IsStopped" then return function() return true; end; end
		if k == "GetTextControl" then return function() return NewControl(); end; end
		if type(k) ~= "string" then return nil; end
		local set = k:match("^Set(%u.*)$");
		if set then return function(self, v) rawset(self, set, v); end; end
		local get = k:match("^Get(%u.*)$");
		if get then return function(self) local v = rawget(self, get); if v == nil then return 0; end return v; end; end
		if k:match("^%l") then return nil; end -- a field never set (cb_L, tip...)
		return function() return Any; end;
	end });
end

-- An instance: its controls are made on demand. `sans` lists the names it does not have (the
-- civilian row has no drawer; the unit list keeps its instance managers in fields the panel
-- tests against nil).
local function NewInstance(sans)
	return setmetatable({}, { __index = function(t, k)
		if type(k) ~= "string" or (sans ~= nil and sans(k)) then return nil; end
		local c = NewControl();
		rawset(t, k, c);
		return c;
	end });
end
local function SansTiroir(k) return k:find("Corps", 1, true) ~= nil or k:find("Army", 1, true) ~= nil; end
local function SansGestionnaires(k) return k:match("IM$") ~= nil; end

-- InstanceManager.lua, reduced to what PopulateUnits uses: instances are reused in order after
-- ResetInstances, as the game's manager does.
local InstanceManager = {};
InstanceManager.__index = InstanceManager;
function InstanceManager:new(name)
	return setmetatable({ name = name, all = {}, m_iAllocatedInstances = 0 }, InstanceManager);
end
function InstanceManager:GetInstance()
	self.m_iAllocatedInstances = self.m_iAllocatedInstances + 1;
	local i = self.all[self.m_iAllocatedInstances];
	if i == nil then
		i = NewInstance(self.name == "CivilianListInstance" and SansTiroir or nil);
		self.all[self.m_iAllocatedInstances] = i;
	end
	return i;
end
function InstanceManager:ResetInstances()
	self.m_iAllocatedInstances = 0;
end

-- A machine: the panel loaded, one city, the commands its buttons send.
local function NewPanel(withBlock, extras)
	local m = { sent = {}, closed = 0 };
	local city = {
		GetID = function() return 1; end,
		GetOwner = function() return LOCAL; end,
		GetCityAI = function() return { GetBuildRecommendations = function() return {}; end }; end,
		-- The cost of the single unit, which the game's item no longer has once a corps is offered.
		-- Index 99 fails: a read only the block makes, to see one row fail alone.
		GetBuildQueue = function() return { GetUnitCost = function(_, index)
			if index == 99 then error("cost unknown"); end
			return index == 5 and 100 or 0;
		end }; end,
	};
	m.city = city;

	-- Unknown globals are the catch-all object, except the proxy's own (BGD_...): a block tests
	-- its state against nil to install itself once.
	local env = setmetatable({}, { __index = function(_, k)
		if _G[k] ~= nil then return _G[k]; end
		if type(k) == "string" and k:match("^BGD_") then return nil; end
		return Any;
	end });
	env.include = function() end;
	env.InstanceManager = InstanceManager;
	env.Mouse = { eLClick = "L", eRClick = "R", eMouseEnter = "E" };
	env.CreateTabs = function()
		local tabs = {};
		tabs.AddTab = function(c, f)
			rawset(c, "CallbackFunc", function() tabs.selectedControl = c; f(c); end);
			c:RegisterCallback(env.Mouse.eLClick, c.CallbackFunc);
		end;
		tabs.SelectTab = function(c) c.CallbackFunc(); end;
		return setmetatable(tabs, { __index = Any });
	end;
	env.IsTutorialRunning = function() return false; end;
	env.Controls = setmetatable({}, { __index = function(t, k) local c = NewControl(); rawset(t, k, c); return c; end });
	env.ContextPtr = setmetatable({ IsHidden = function() return false; end }, { __index = Any });
	env.Locale = { Lookup = function(s) return s; end, ToUpper = function(s) return s; end };
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.Players = { [LOCAL] = {} };
	env.UI = setmetatable({ GetHeadSelectedCity = function() return city; end }, { __index = Any });
	env.GameInfo = setmetatable({ Units = {
		UNIT_INFANTRY = { Index = 5, Combat = 70, RangedCombat = 0, Domain = "DOMAIN_LAND", FormationClass = "FORMATION_CLASS_LAND_COMBAT" },
		UNIT_BROKEN = { Index = 99, Combat = 70, RangedCombat = 0, Domain = "DOMAIN_LAND", FormationClass = "FORMATION_CLASS_LAND_COMBAT" },
		UNIT_SETTLER = { Index = 1, Combat = 0, RangedCombat = 0, Domain = "DOMAIN_LAND", FormationClass = "FORMATION_CLASS_CIVILIAN" },
	} }, { __index = Any });
	env.Events = setmetatable({}, { __index = function() return { Add = function() end, Remove = function() end }; end });
	env.RefreshCurrentProduction = function() return false; end;

	local chunk = PANEL .. "\r\n" .. PRODUCTION;
	if withBlock then chunk = chunk .. "\r\n" .. BLOCK; end
	if extras then chunk = chunk .. "\r\n" .. EXTRAS; end
	local f = assert(load(chunk, "ProductionPanel.lua", "t", env));
	f();

	-- What a click sends: the game's functions are replaced after loading, the block reads them
	-- by name when it dresses a row.
	for _, name in ipairs({ "BuildUnit", "BuildUnitCorps", "BuildUnitArmy", "PurchaseUnit", "PurchaseUnitCorps", "PurchaseUnitArmy" }) do
		env[name] = function(c, item) m.sent[#m.sent + 1] = { name, c, item }; end;
	end
	env.CloseAfterNewProduction = function() m.closed = m.closed + 1; end;
	env.Close = function() m.closed = m.closed + 1; end;

	-- The panel is shown once, as when a city is selected: View reads the advisor's recommendations,
	-- which every row then looks up. The lists themselves are filled by the test, one at a time.
	env.PopulateList = function() end;
	env.RefreshQueue = function() end;
	env.View({ City = city, Owner = LOCAL });

	-- One unit list per list mode, as the panel keeps one instance manager per tab.
	local lists = {};
	function m.Populate(items, listMode)
		listMode = listMode or PRODUCTION_LIST;
		local list = lists[listMode];
		if list == nil then
			list = { unitList = NewInstance(SansGestionnaires) };
			list.IM = { GetInstance = function() return list.unitList; end };
			lists[listMode] = list;
		end
		local data = { City = city, Owner = LOCAL, UnitItems = items, UnitPurchases = items };
		env.PopulateUnits(data, listMode, list.IM);
		local rows = {};
		local im = list.unitList.unitListIM;
		for i = 1, im.m_iAllocatedInstances do rows[i] = im.all[i]; end
		return rows, list.unitList.civilianListIM;
	end
	-- Clicks a button and gives back the name of the function it called.
	function m.Click(button)
		local before, closed = #m.sent, m.closed;
		button.cb_L();
		local s = m.sent[#m.sent];
		if #m.sent ~= before + 1 or s[2] ~= city or m.closed ~= closed + 1 then return nil; end
		return s[1], s[3];
	end
	m.env = env;
	return m;
end

-- The item the game builds for a unit of the production list (GetData). With a corps offered, the
-- game leaves the cost of the corps in Cost.
local function Infantry(o)
	local item = {
		Type = "UNIT_INFANTRY", Name = "Infantry", ToolTip = "tip single", Hash = 77, Kind = "KIND_UNIT",
		TurnsLeft = 3, Disabled = false, Civilian = false, Cost = 100, Progress = 0, ReligiousStrength = 0,
		IsCurrentProduction = false,
		Corps = false, CorpsCost = 0, CorpsTurnsLeft = 1, CorpsTooltip = "", CorpsName = "",
		Army = false, ArmyCost = 0, ArmyTurnsLeft = 1, ArmyTooltip = "", ArmyName = "",
	};
	if o.corps then
		item.Corps = true; item.CorpsDisabled = false; item.CorpsCost = 200; item.CorpsTurnsLeft = 6;
		item.CorpsTooltip = "tip corps"; item.CorpsName = "(Corps[ICON_Corps])"; item.Cost = item.CorpsCost;
	end
	if o.army then
		item.Army = true; item.ArmyDisabled = false; item.ArmyCost = 300; item.ArmyTurnsLeft = 9;
		item.ArmyTooltip = "tip army"; item.ArmyName = "(Army[ICON_Army])";
	end
	for k, v in pairs(o.set or {}) do item[k] = v; end
	return item;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end
local function Has(s, part) return type(s) == "string" and s:find(part, 1, true) ~= nil; end

-- Without the block: what the game does, and what the block reverses.
do
	local m = NewPanel(false);
	local item = Infantry({ corps = true, army = true });
	local rows = m.Populate({ item });
	local r = rows[1];
	Check("game: one row", #rows == 1);
	Check("game: main button is the single unit", r.LabelText.Text == "Infantry" and r.CostText.Text == "3[ICON_Turn]"
		and r.Button.tip == "tip single" and m.Click(r.Button) == "BuildUnit");
	Check("game: drawer holds the corps, then the army", m.Click(r.TrainCorpsButton) == "BuildUnitCorps"
		and m.Click(r.TrainArmyButton) == "BuildUnitArmy" and r.CorpsLabelIcon.Text == "(Corps[ICON_Corps])"
		and r.ArmyLabelIcon.Text == "(Army[ICON_Army])");
end

-- Corps and army offered: the army on the main button, the corps then the single unit below.
do
	local m = NewPanel(true);
	local item = Infantry({ corps = true, army = true });
	local rows = m.Populate({ item });
	local r = rows[1];
	local name, sent = m.Click(r.Button);
	Check("army: main button trains the army", name == "BuildUnitArmy" and sent == item);
	Check("army: main button names it", Has(r.LabelText.Text, "Infantry") and Has(r.LabelText.Text, "(Army[ICON_Army])"));
	Check("army: its turns and tooltip", r.CostText.Text == "9[ICON_Turn]" and r.Button.tip == "tip army" and r.Disabled.tip == "tip army");
	Check("army: main button shown and active", r.Button.Hide == false and r.Button.Disabled == false and r.Disabled.Hide == true);
	Check("army: first slot of the drawer is the corps", m.Click(r.TrainCorpsButton) == "BuildUnitCorps"
		and r.CorpsLabelIcon.Text == "(Corps[ICON_Corps])" and r.CorpsCostText.Text == "6[ICON_Turn]"
		and r.TrainCorpsButton.tip == "tip corps" and r.CorpsDisabled.Hide == true);
	Check("army: second slot is the single unit", m.Click(r.TrainArmyButton) == "BuildUnit"
		and r.ArmyLabelIcon.Text == "" and r.ArmyCostText.Text == "3[ICON_Turn]"
		and r.TrainArmyButton.tip == "tip single" and r.ArmyDisabled.Hide == true);
	Check("army: both slots shown, drawer closed, arrow shown", r.CorpsButtonContainer.Hide == false
		and r.ArmyButtonContainer.Hide == false and r.ArmyCorpsDrawer.Hide == true and r.CorpsArmyDropdownArea.Hide == false);
	Check("army: no meter without progress", r.ProductionProgressArea.Hide == true
		and r.ProductionCorpsProgressArea.Hide == true and r.ProductionArmyProgressArea.Hide == true);
	Check("army: right click still opens the Civilopedia entry", type(r.Button.cb_R) == "function");
end

-- Corps only (Nationalism without Mobilization): the corps on the main button.
do
	local m = NewPanel(true);
	local rows = m.Populate({ Infantry({ corps = true }) });
	local r = rows[1];
	Check("corps: main button trains the corps", m.Click(r.Button) == "BuildUnitCorps"
		and Has(r.LabelText.Text, "(Corps[ICON_Corps])") and r.CostText.Text == "6[ICON_Turn]" and r.Button.tip == "tip corps");
	Check("corps: the drawer holds the single unit", m.Click(r.TrainCorpsButton) == "BuildUnit"
		and r.CorpsLabelIcon.Text == "" and r.CorpsCostText.Text == "3[ICON_Turn]" and r.TrainCorpsButton.tip == "tip single");
	Check("corps: army slot left hidden and untouched", r.ArmyButtonContainer.Hide == true and r.TrainArmyButton.cb_L == nil);
end

-- Army disabled (strategic resources missing), corps not: the corps takes the main button.
do
	local m = NewPanel(true);
	local rows = m.Populate({ Infantry({ corps = true, army = true, set = { ArmyDisabled = true } }) });
	local r = rows[1];
	Check("army disabled: main button trains the corps", m.Click(r.Button) == "BuildUnitCorps" and Has(r.LabelText.Text, "(Corps[ICON_Corps])"));
	Check("army disabled: the army is first in the drawer, greyed", m.Click(r.TrainCorpsButton) == "BuildUnitArmy"
		and r.CorpsLabelIcon.Text == "(Army[ICON_Army])" and r.CorpsCostText.Text == "9[ICON_Turn]"
		and r.CorpsDisabled.Hide == false and r.CorpsDisabled.tip == "tip army");
	Check("army disabled: the single unit is second, not greyed", m.Click(r.TrainArmyButton) == "BuildUnit"
		and r.ArmyLabelIcon.Text == "" and r.ArmyDisabled.Hide == true);
end

-- Nothing above the single unit can be trained: the row stays the game's.
do
	local m = NewPanel(true);
	local rows = m.Populate({
		Infantry({ corps = true, army = true, set = { CorpsDisabled = true, ArmyDisabled = true } }),
		Infantry({ corps = true, army = true, set = { Disabled = true, CorpsDisabled = true, ArmyDisabled = true } }),
		Infantry({}),
	});
	local r = rows[1];
	Check("corps and army disabled: row left as the game's", r.LabelText.Text == "Infantry" and m.Click(r.Button) == "BuildUnit"
		and m.Click(r.TrainCorpsButton) == "BuildUnitCorps" and m.Click(r.TrainArmyButton) == "BuildUnitArmy"
		and r.CorpsDisabled.Hide == false and r.ArmyDisabled.Hide == false);
	Check("everything disabled: row left as the game's", rows[2].LabelText.Text == "Infantry" and rows[2].Disabled.Hide == false
		and rows[2].Button.Disabled == true);
	Check("no corps, no army: row left as the game's", rows[3].LabelText.Text == "Infantry" and m.Click(rows[3].Button) == "BuildUnit"
		and rows[3].CorpsArmyDropdownArea.Hide == true);
end

-- Progress: each button measures the production placed against the cost of its own formation.
do
	local m = NewPanel(true);
	local rows = m.Populate({ Infantry({ corps = true, army = true, set = { Progress = 50 } }) });
	local r = rows[1];
	Check("progress: army on the main button, 50 / 300", r.ProductionProgressArea.Hide == false and r.ProductionProgress.Percent == 50 / 300);
	Check("progress: corps, 50 / 200", r.ProductionCorpsProgressArea.Hide == false and r.ProductionCorpsProgress.Percent == 50 / 200);
	Check("progress: single unit, 50 / 100 (cost read from the build queue)", r.ProductionArmyProgressArea.Hide == false
		and r.ProductionArmyProgress.Percent == 50 / 100);
	rows = m.Populate({ Infantry({ corps = true, army = true, set = { Progress = 150 } }) });
	r = rows[1];
	Check("progress: meter hidden once the cost is covered", r.ProductionProgressArea.Hide == false
		and r.ProductionCorpsProgressArea.Hide == false and r.ProductionArmyProgressArea.Hide == true);
end

-- A row is reused by the next fill of the list: nothing of the army stays on it.
do
	local m = NewPanel(true);
	local rows = m.Populate({ Infantry({ corps = true, army = true, set = { Progress = 50 } }) });
	local first = rows[1];
	rows = m.Populate({ Infantry({}) });
	local r = rows[1];
	Check("reuse: same instance", r == first);
	Check("reuse: single unit again", r.LabelText.Text == "Infantry" and r.CostText.Text == "3[ICON_Turn]"
		and r.Button.tip == "tip single" and m.Click(r.Button) == "BuildUnit");
	Check("reuse: drawer hidden", r.CorpsArmyDropdownArea.Hide == true and r.CorpsButtonContainer.Hide == true
		and r.ArmyButtonContainer.Hide == true);
	rows = m.Populate({ Infantry({ corps = true }) });
	Check("reuse: corps after that", m.Click(rows[1].Button) == "BuildUnitCorps" and m.Click(rows[1].TrainCorpsButton) == "BuildUnit");
end

-- Several rows, civilians among them: each row gets its own item.
do
	local m = NewPanel(true);
	local settler = { Type = "UNIT_SETTLER", Name = "Settler", ToolTip = "tip settler", Hash = 11, Kind = "KIND_UNIT",
		TurnsLeft = 4, Disabled = false, Civilian = true, Cost = 80, Progress = 0, ReligiousStrength = 0,
		IsCurrentProduction = false, Corps = false, Army = false };
	local a, b = Infantry({ corps = true, army = true }), Infantry({ corps = true });
	local current = Infantry({ corps = true, army = true, set = { IsCurrentProduction = true } });
	local rows, civilians = m.Populate({ settler, a, current, b });
	local n1, s1 = m.Click(rows[1].Button);
	local n2, s2 = m.Click(rows[2].Button);
	Check("list: two military rows (the current production is not listed)", #rows == 2);
	Check("list: first row trains the army of its unit", n1 == "BuildUnitArmy" and s1 == a);
	Check("list: second row trains the corps of its unit", n2 == "BuildUnitCorps" and s2 == b);
	local c = civilians.all[1];
	Check("list: civilian row untouched", civilians.m_iAllocatedInstances == 1 and c.LabelText.Text == "Settler"
		and m.Click(c.Button) == "BuildUnit");
end

-- The production queue lists the same rows: same order there.
do
	local m = NewPanel(true);
	local rows = m.Populate({ Infantry({ corps = true, army = true }) }, QUEUE_LIST);
	Check("queue: main button trains the army", m.Click(rows[1].Button) == "BuildUnitArmy"
		and m.Click(rows[1].TrainArmyButton) == "BuildUnit");
end

-- Purchases are left alone: a click buys at once.
do
	local m = NewPanel(true);
	local item = Infantry({ corps = true, army = true, set = { Yield = "YIELD_GOLD", Cost = 900, CorpsCost = 1800, ArmyCost = 2700 } });
	local rows = m.Populate({ item }, GOLD_LIST);
	local r = rows[1];
	Check("purchase: main button buys the single unit", m.Click(r.Button) == "PurchaseUnit" and r.LabelText.Text == "Infantry"
		and r.Button.tip == "tip single");
	Check("purchase: drawer as the game's", m.Click(r.TrainCorpsButton) == "PurchaseUnitCorps"
		and m.Click(r.TrainArmyButton) == "PurchaseUnitArmy" and r.ArmyLabelIcon.Text == "(Army[ICON_Army])");
	-- And the production list, filled before or after, keeps the block's order.
	rows = m.Populate({ Infantry({ corps = true, army = true }) });
	Check("purchase then production: army on the main button", m.Click(rows[1].Button) == "BuildUnitArmy");
end

-- A failure of the game's PopulateUnits goes through, and the next fill works.
do
	local m = NewPanel(true);
	local ok = pcall(m.env.PopulateUnits, { City = m.city, UnitItems = 5 }, PRODUCTION_LIST, { GetInstance = function() return NewInstance(SansGestionnaires); end });
	Check("game error: passed on", not ok and m.env.BGD_Formations_Etat.lignes == nil);
	local rows = m.Populate({ Infantry({ corps = true, army = true }) });
	Check("game error: next fill dressed", m.Click(rows[1].Button) == "BuildUnitArmy");
end

-- A row the block cannot dress stays the game's, and the others are dressed.
do
	local printed = {};
	local print_jeu = print;
	print = function(s) printed[#printed + 1] = tostring(s); end;
	local m = NewPanel(true);
	local broken = Infantry({ corps = true, army = true, set = { Type = "UNIT_BROKEN" } });
	local ok, rows = pcall(m.Populate, { broken, Infantry({ corps = true, army = true }) });
	print = print_jeu;
	local noted = false;
	for _, s in ipairs(printed) do if Has(s, "Formations echec") then noted = true; end end
	Check("row failure: noted in the log, list still filled", ok and noted and #rows == 2);
	Check("row failure: that row stays the game's", ok and rows[1].LabelText.Text == "Infantry" and m.Click(rows[1].Button) == "BuildUnit");
	Check("row failure: next row dressed", ok and m.Click(rows[2].Button) == "BuildUnitArmy");
end

-- Loaded twice (a mod that includes the file again): one wrapper only.
do
	local m = NewPanel(true);
	local etat = m.env.BGD_Formations_Etat;
	local populate = m.env.PopulateUnits;
	assert(load(BLOCK, "ui_formations.lua", "t", m.env))();
	Check("loaded twice: wrapped once", m.env.BGD_Formations_Etat == etat and m.env.PopulateUnits == populate);
end

-- With the other blocks of the file: loaded without a failure, same order.
do
	local printed = {};
	local print_jeu = print;
	print = function(s) printed[#printed + 1] = tostring(s); end;
	local ok, name = pcall(function()
		local m = NewPanel(true, true);
		local rows = m.Populate({ Infantry({ corps = true, army = true }) });
		return m.Click(rows[1].Button);
	end);
	print = print_jeu;
	local echec = not ok and tostring(name) or nil;
	for _, s in ipairs(printed) do if s:find("echec", 1, true) then echec = s; end end
	Check("with ui_quartiers and ui_debordement: loaded without failure" .. (echec and (" (" .. echec .. ")") or ""),
		ok and name == "BuildUnitArmy" and echec == nil);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
