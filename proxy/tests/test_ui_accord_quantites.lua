-- Off-game test of src/ui_accord_quantites.lua (Shift+click and Ctrl+click on an available
-- resource, gold, gold per turn or favor of the deal screen):
--   lua proxy/tests/test_ui_accord_quantites.lua [game folder]
-- 1. The real DiplomacyDealView.lua (Expansion2) followed by the block compiles (HavokScript
--    annotations removed), and the globals the block wraps are there.
-- 2. A deal reduced to what a click on an available resource uses: items with an amount, a
--    maximum (the giver's stock) and a validity (room under the receiver's cap). The scenarios run
--    with a click written like the game's, then, when the game is installed, with the real
--    OnClickAvailableResource of DiplomacyDealView_Expansion2.lua.
-- 3. Gold, gold per turn and favor: clicks that receive the amount to add and cap it themselves,
--    written like the game's, then the real ones of DiplomacyDealView.lua and
--    DiplomacyDealView_Expansion2.lua.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local function StripTypes(text)
	for _, t in ipairs({ "table", "number", "string", "boolean", "object", "userdata", "ifunction", "function" }) do
		text = text:gsub("([%w_%)%]])%s*:%s*" .. t .. "%f[^%w_(]", "%1");
	end
	return text;
end
local BLOCK = ReadFile(dir .. "/../src/ui_accord_quantites.lua");
local failures = 0;
local function Check(cond, what)
	if cond then
		print("ok    " .. what);
	else
		failures = failures + 1;
		print("ECHEC " .. what);
	end
end

local GAME = arg[1] or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI]];

-- 1. The real file.
local okView, view = pcall(ReadFile, GAME .. [[\DLC\Expansion2\UI\Replacements\DiplomacyDealView.lua]]);
if okView then
	view = StripTypes(view);
	local chunk, err = load(view .. "\r\n" .. BLOCK, "@DiplomacyDealView.lua");
	Check(chunk ~= nil, "game file followed by the block: compiles " .. tostring(err or ""));
	for _, name in ipairs({ "OnClickAvailableResource", "InputHandler", "UpdateDealPanel", "UpdateProposedWorkingDeal",
		"OnClickAvailableOneTimeGold", "OnClickAvailableMultiTurnGold" }) do
		Check(view:find("\nfunction " .. name .. "%(") ~= nil, name .. " is a global");
	end
	Check(view:find("ContextPtr:SetInputHandler( InputHandler, true )", 1, true) ~= nil,
		"the game registers InputHandler the way the block does again");
	Check(view:find("OnClickAvailableResource(player, resourceType); end", 1, true) ~= nil,
		"the icon's callback looks the click up by name");
	Check(view:find("OnClickAvailableOneTimeGold(player, ms_DefaultOneTimeGoldAmount); end", 1, true) ~= nil
		and view:find("OnClickAvailableMultiTurnGold(player, ms_DefaultMultiTurnGoldAmount, ms_DefaultMultiTurnGoldDuration); end", 1, true) ~= nil,
		"the gold callbacks look the click up by name and give it the amount");
	Check(view:find("local ms_DefaultOneTimeGoldAmount = 100;", 1, true) ~= nil
		and view:find("local ms_DefaultMultiTurnGoldAmount = 10;", 1, true) ~= nil,
		"a click adds 100 gold, 10 gold per turn");
else
	print("game absent, part 1 skipped");
end

-- 2. The deal.
local LOCAL, OTHER = 0, 1;
local IRON, SILK = 7, 9;
local GOLD, RESOURCES, FAVOR = 1, 3, 5;
local MOUSE_UP, KEY_DOWN, KEY_UP = 10, 1, 2;
local VK_SHIFT, VK_CONTROL = 16, 17;

-- A click written like the game's (XP2): one more on the item already there, else a new item of
-- one unit, removed if it is not valid.
local function ModelClick(env)
	return function(player, resourceType)
		if (env.ms_bIsDemand or env.ms_bIsGift) and env.ms_InitiatedByPlayerID == env.ms_OtherPlayerID then
			return;
		end
		local accumulate = env.GameInfo.Resource_Consumption[env.GameInfo.Resources[resourceType].ResourceType].Accumulate;
		local pDeal = env.DealManager.GetWorkingDeal(1, LOCAL, OTHER);
		for _, pDealItem in ipairs(pDeal:FindItemsByType(env.DealItemTypes.RESOURCES, env.DealItemSubTypes.NONE, player:GetID())) do
			if pDealItem:GetValueType() == resourceType
				and ((accumulate and not pDeal:GetItemParent(pDealItem)) or pDealItem:GetDuration() ~= 0) then
				local n = math.min(pDealItem:GetAmount() + 1, pDealItem:GetMaxAmount());
				if n == pDealItem:GetAmount() then return; end
				pDealItem:SetAmount(n);
				if not pDealItem:IsValid() then
					pDealItem:SetAmount(n - 1);
					return;
				end
				env.UI.PlaySound("UI_GreatWorks_Put_Down");
				env.UpdateDealPanel(player);
				env.UpdateProposedWorkingDeal();
				return;
			end
		end
		local pDealItem = pDeal:AddItemOfType(env.DealItemTypes.RESOURCES, player:GetID());
		pDealItem:SetValueType(resourceType);
		pDealItem:SetAmount(1);
		pDealItem:SetDuration(accumulate and 0 or 30);
		if not pDealItem:IsValid() then
			pDeal:RemoveItemByID(pDealItem:GetID());
		else
			env.UI.PlaySound("UI_GreatWorks_Put_Down");
		end
		env.UpdateDealPanel(player);
		env.UpdateProposedWorkingDeal();
	end
end

-- opts.stock: what the giver has ; opts.room: what the receiver can take (nil: no cap) ;
-- opts.click(env): the click to wrap (ModelClick by default) ; opts.gold, opts.gpt, opts.favor:
-- the most gold, gold per turn and favor a deal accepts ; opts.before(env): run before the block.
local function NewMachine(opts)
	opts = opts or {};
	local m = { stock = opts.stock or 50, room = opts.room, gold = opts.gold or 2500, gpt = opts.gpt or 250,
		favor = opts.favor or 37, items = {}, nextId = 1, valid = 0, sounds = 0,
		calls = {}, inputs = 0, printed = {} };
	local env = setmetatable({}, { __index = _G });
	m.env = env;
	env.print = function(s) m.printed[#m.printed + 1] = tostring(s); end;

	local function NewItem(from, type)
		local it = { id = m.nextId, from = from, type = type or RESOURCES, amount = 0, duration = 0, valueType = -1 };
		m.nextId = m.nextId + 1;
		function it:GetID() return self.id; end
		function it:GetAmount() return self.amount; end
		function it:SetAmount(n) self.amount = n; end
		function it:GetDuration() return self.duration; end
		function it:SetDuration(n) self.duration = n; end
		function it:GetValueType() return self.valueType; end
		function it:SetValueType(v) self.valueType = v; end
		function it:GetMaxAmount()
			if self.type == GOLD then return self.duration == 0 and m.gold or m.gpt; end
			if self.type == FAVOR then return m.favor; end
			return m.stock;
		end
		function it:IsValid()
			m.valid = m.valid + 1;
			return self.amount >= 1 and self.amount <= m.stock and (m.room == nil or self.amount <= m.room);
		end
		return it;
	end
	m.NewItem = NewItem;

	local deal = {};
	function deal:FindItemsByType(type, _, from)
		local t = {};
		for _, it in ipairs(m.items) do
			if it.from == from and it.type == type then t[#t + 1] = it; end
		end
		return t;
	end
	function deal:AddItemOfType(type, from)
		local it = NewItem(from, type);
		m.items[#m.items + 1] = it;
		return it;
	end
	function deal:RemoveItemByID(id)
		for i, it in ipairs(m.items) do
			if it.id == id then table.remove(m.items, i); return; end
		end
	end
	function deal:GetItemParent(it) return it.parent; end

	env.DealManager = { GetWorkingDeal = function() return deal; end };
	env.DealDirection = { OUTGOING = 1 };
	env.DealItemTypes = { GOLD = GOLD, RESOURCES = RESOURCES, FAVOR = FAVOR };
	env.DealItemSubTypes = { NONE = 0 };
	env.GameInfo = {
		Resources = { [IRON] = { ResourceType = "RESOURCE_IRON" }, [SILK] = { ResourceType = "RESOURCE_SILK" } },
		Resource_Consumption = { RESOURCE_IRON = { Accumulate = true }, RESOURCE_SILK = { Accumulate = false } },
	};
	env.Game = { GetLocalPlayer = function() return LOCAL; end };
	env.UI = { PlaySound = function() m.sounds = m.sounds + 1; end };
	m.me = { GetID = function() return LOCAL; end, GetResources = function() return { GetResourceAmount = function() return m.stock; end }; end,
		GetTreasury = function() return {}; end };
	m.them = { GetID = function() return OTHER; end, GetResources = m.me.GetResources, GetTreasury = m.me.GetTreasury };
	env.g_LocalPlayer, env.g_OtherPlayer = m.me, m.them;
	env.GetOtherPlayer = function(p) return p == m.them and m.me or m.them; end
	env.ms_bIsDemand, env.ms_bIsGift, env.ms_InitiatedByPlayerID, env.ms_OtherPlayerID = false, false, LOCAL, OTHER;
	env.clip = function(val, min, max)
		if min and val < min then return min; elseif max and val > max then return max; end
		return val;
	end

	-- The amounts when each refresh runs: the AI is asked about the deal as it is then.
	local function Total()
		local n = 0;
		for _, it in ipairs(m.items) do n = n + it.amount; end
		return n;
	end
	env.UpdateDealPanel = function(player) m.calls[#m.calls + 1] = { "panel", Total(), player }; end
	env.UpdateProposedWorkingDeal = function() m.calls[#m.calls + 1] = { "propose", Total() }; end

	env.KeyEvents = { KeyDown = KEY_DOWN, KeyUp = KEY_UP };
	env.Keys = { VK_SHIFT = VK_SHIFT, VK_CONTROL = VK_CONTROL };
	env.InputHandler = function(input) m.inputs = m.inputs + 1; return input.consumed == true; end
	env.ContextPtr = { SetInputHandler = function(_, f, first) m.handler, m.first = f, first; end };
	env.ContextPtr:SetInputHandler(env.InputHandler, true);

	env.OnClickAvailableResource = (opts.click or ModelClick)(env);
	m.gameClick = env.OnClickAvailableResource;
	if opts.before then opts.before(env, m); end
	if not opts.noBlock then
		assert(load(BLOCK, "@ui_accord_quantites.lua", "t", env))();
	end

	function m.Input(t)
		local input = { consumed = t.consumed };
		function input:GetMessageType() return t.msg or MOUSE_UP; end
		function input:GetKey() return t.key; end
		function input:IsShiftDown() if t.broken then error("no modifier"); end return t.shift == true; end
		function input:IsControlDown() return t.ctrl == true; end
		return m.handler(input);
	end
	-- A click: the button event reaches the context with the modifiers held, then the callback.
	function m.Click(mods, player, resourceType)
		mods = mods or {};
		m.Input({ shift = mods.shift, ctrl = mods.ctrl });
		m.calls = {};
		env.OnClickAvailableResource(player or m.me, resourceType or IRON);
	end
	-- The gold and favor icons: their callbacks give the click the amount of one click.
	function m.ClickGold(mods, player)
		m.Input({ shift = mods and mods.shift, ctrl = mods and mods.ctrl });
		m.calls = {};
		env.OnClickAvailableOneTimeGold(player or m.me, 100);
	end
	function m.ClickGpt(mods, player)
		m.Input({ shift = mods and mods.shift, ctrl = mods and mods.ctrl });
		m.calls = {};
		env.OnClickAvailableMultiTurnGold(player or m.me, 10, 30);
	end
	function m.ClickFavor(mods, player)
		m.Input({ shift = mods and mods.shift, ctrl = mods and mods.ctrl });
		m.calls = {};
		env.OnClickAvailableOneTimeFavor(player or m.me, 1);
	end
	-- The same two refreshes, in the order of the gold clicks: proposal, then panel.
	function m.OneGoldRefresh(total)
		return #m.calls == 2 and m.calls[1][1] == "propose" and m.calls[1][2] == total
			and m.calls[2][1] == "panel" and m.calls[2][2] == total;
	end
	function m.Amount(i) return m.items[i or 1] and m.items[i or 1].amount or nil; end
	-- One panel refresh then one proposal, both at the final amount.
	function m.OneRefresh(total)
		return #m.calls == 2 and m.calls[1][1] == "panel" and m.calls[1][2] == total
			and m.calls[2][1] == "propose" and m.calls[2][2] == total;
	end
	return m;
end

local function Scenarios(label, click)
	local function New(opts)
		opts = opts or {};
		opts.click = click;
		return NewMachine(opts);
	end
	local function Ok(cond, what) Check(cond, label .. ": " .. what); end

	do
		local m = New();
		m.Click();
		Ok(m.Amount() == 1 and m.OneRefresh(1) and #m.items == 1, "plain click: one unit, as the game");
		m.Click();
		Ok(m.Amount() == 2 and m.OneRefresh(2), "plain click again: one more");
		Ok(m.env.UpdateDealPanel ~= nil and #m.printed == 0, "plain click: nothing printed");
	end

	do
		local m = New();
		m.Click({ ctrl = true });
		Ok(m.Amount() == 10 and #m.items == 1, "Ctrl+click: 10");
		Ok(m.OneRefresh(10), "Ctrl+click: one panel refresh then one proposal, at 10 (not at 1)");
		Ok(m.sounds == 1, "Ctrl+click: one sound");
		m.Click({ ctrl = true });
		Ok(m.Amount() == 20 and m.OneRefresh(20), "Ctrl+click again: 20");
		m.Click();
		Ok(m.Amount() == 21, "then a plain click: 21");
	end

	do
		local m = New({ stock = 25 });
		m.Click({ ctrl = true });
		m.Click({ ctrl = true });
		m.Click({ ctrl = true });
		Ok(m.Amount() == 25 and m.OneRefresh(25), "Ctrl+click over the stock: the stock (25)");
		m.Click({ ctrl = true });
		Ok(m.Amount() == 25 and #m.calls == 0, "Ctrl+click at the maximum: nothing, as the game");
	end

	do
		local m = New({ stock = 137 });
		m.Click({ shift = true });
		Ok(m.Amount() == 137 and #m.items == 1 and m.OneRefresh(137), "Shift+click: the whole stock (137)");
		Ok(m.items[1].duration == 0, "Shift+click: stockpiled resource stays a one-time item");
		m.Click({ shift = true });
		Ok(m.Amount() == 137 and #m.calls == 0, "Shift+click again: nothing more");
	end

	do
		local m = New({ stock = 30 });
		m.Click();
		m.Click();
		m.Click();
		m.Click({ shift = true });
		Ok(m.Amount() == 30 and #m.items == 1, "Shift+click on a resource already in the deal: raised to the stock");
	end

	do
		local m = New({ stock = 30 });
		m.Click({ shift = true, ctrl = true });
		Ok(m.Amount() == 30, "Shift and Ctrl: Shift wins");
	end

	do
		-- The other player's resource: asked from them.
		local m = New({ stock = 40 });
		m.Click({ shift = true }, m.them);
		Ok(m.Amount() == 40 and m.items[1].from == OTHER and m.calls[1][3] == m.them,
			"Shift+click on the other player's resource: asked from them, their panel refreshed");
	end

	do
		-- Room under the receiver's cap for 7 only.
		local m = New({ stock = 200, room = 7 });
		m.Click({ shift = true });
		Ok(m.Amount() == 7 and m.items[1]:IsValid() and m.OneRefresh(7), "Shift+click, room for 7 under the cap: 7");
		Ok(m.valid <= 14, "largest valid amount found by bisection (" .. m.valid .. " validity tests)");
		local m2 = New({ stock = 200, room = 7 });
		m2.Click({ ctrl = true });
		Ok(m2.Amount() == 7 and m2.OneRefresh(7), "Ctrl+click, room for 7 under the cap: 7");
		local m3 = New({ stock = 200, room = 1 });
		m3.Click({ shift = true });
		Ok(m3.Amount() == 1 and m3.OneRefresh(1), "Shift+click, room for 1: 1");
	end

	do
		-- No room at all: the game removes its new item, nothing to raise.
		local m = New({ stock = 200, room = 0 });
		m.Click({ shift = true });
		Ok(#m.items == 0 and #m.printed == 0, "Shift+click with no room: no item, no failure");
	end

	do
		-- A luxury: a 30-turn item.
		local m = New({ stock = 3 });
		m.Click({ shift = true }, m.me, SILK);
		Ok(m.Amount() == 3 and m.items[1].duration == 30, "Shift+click on a luxury: every copy, for 30 turns");
	end

	do
		-- Two resources in the deal: only the clicked one moves.
		local m = New({ stock = 20 });
		m.Click({}, m.me, SILK);
		m.Click({ shift = true }, m.me, IRON);
		Ok(#m.items == 2 and m.Amount(1) == 1 and m.Amount(2) == 20, "another resource of the deal is left alone");
	end

	do
		-- A demand or gift made by the other player cannot be changed: the game's click returns.
		local m = New();
		m.env.ms_bIsGift, m.env.ms_bIsDemand, m.env.ms_InitiatedByPlayerID = true, true, OTHER;
		m.Click({ shift = true });
		Ok(#m.items == 0 and #m.calls == 0 and #m.printed == 0, "deal that is not ours: untouched, no refresh");
	end
end

Scenarios("model click", nil);

-- A stockpiled resource that comes with a city (one-time item with a parent) is not the one a
-- click raises: the click adds an item of its own, and that one is raised.
do
	local m = NewMachine({ stock = 12 });
	local child = m.NewItem(LOCAL);
	child.valueType, child.amount, child.parent = IRON, 4, {};
	m.items[1] = child;
	m.Click({ shift = true });
	Check(#m.items == 2 and m.Amount(1) == 4 and m.Amount(2) == 12, "item that comes with a city left alone, the click's own item raised");
end

-- The input handler: the game's still runs and its answer is returned.
do
	local m = NewMachine();
	Check(m.first == true and m.handler ~= m.env.InputHandler, "input handler registered again, same mode");
	Check(m.Input({ consumed = true }) == true and m.Input({}) == false and m.inputs == 2,
		"game's input handler called, its answer returned");
	-- Keys alone, no mouse event in between.
	m.Input({ msg = KEY_DOWN, key = VK_CONTROL });
	m.calls = {};
	m.env.OnClickAvailableResource(m.me, IRON);
	Check(m.Amount() == 10, "Ctrl key down, then the click: 10");
	m.Input({ msg = KEY_UP, key = VK_CONTROL, ctrl = true });
	m.env.OnClickAvailableResource(m.me, IRON);
	Check(m.Amount() == 11, "Ctrl key up, then the click: one unit");
	m.Input({ msg = KEY_DOWN, key = VK_SHIFT });
	m.Input({ msg = KEY_UP, key = VK_SHIFT });
	m.env.OnClickAvailableResource(m.me, IRON);
	Check(m.Amount() == 12, "Shift pressed and released: one unit");
	-- A modifier released while the screen was not receiving input: the click's own event says so.
	m.Input({ msg = KEY_DOWN, key = VK_SHIFT });
	m.Click({});
	Check(m.Amount() == 13, "Shift released out of sight: the click's own event wins, one unit");
	-- An input that cannot be read: the game's handler still runs.
	local before = m.inputs;
	local okInput = pcall(m.Input, { broken = true });
	Check(okInput and m.inputs == before + 1, "unreadable input: no failure, game's handler called");
end

-- Block loaded twice: one wrapper, one handler.
do
	local m = NewMachine();
	local wrapped, handler = m.env.OnClickAvailableResource, m.handler;
	assert(load(BLOCK, "@ui_accord_quantites.lua", "t", m.env))();
	Check(m.env.OnClickAvailableResource == wrapped and m.handler == handler, "not wrapped twice");
	Check(wrapped ~= m.gameClick, "the click is wrapped");
end

-- The game's click fails: the refreshes are the game's again, nothing goes up.
do
	local m = NewMachine({ click = function() return function() error("boom"); end end });
	local panel, propose = m.env.UpdateDealPanel, m.env.UpdateProposedWorkingDeal;
	local ok = pcall(m.Click, { shift = true });
	Check(ok and m.env.UpdateDealPanel == panel and m.env.UpdateProposedWorkingDeal == propose,
		"failing click: no failure of ours, refreshes put back");
	Check(#m.printed == 1 and m.printed[1]:find("boom", 1, true) ~= nil, "failing click: written to Lua.log");
end

-- A context without what the block wraps: nothing done, no failure.
do
	local env = setmetatable({ print = function() end }, { __index = _G });
	local ok = pcall(assert(load(BLOCK, "@ui_accord_quantites.lua", "t", env)));
	Check(ok and env.BGD_AccordQuantites == nil and env.OnClickAvailableResource == nil, "nothing to wrap: nothing done");
end

-- 3. Gold, gold per turn, favor.
-- A click written like the game's: `iAddAmount` more on the item already there, capped by
-- GetMaxAmount, else a new item, removed if it is empty.
local function ModelAmountClick(env, type, multiTurn)
	return function(player, iAddAmount, iDuration)
		if (env.ms_bIsDemand or env.ms_bIsGift) and env.ms_InitiatedByPlayerID == env.ms_OtherPlayerID then
			return;
		end
		local pDeal = env.DealManager.GetWorkingDeal(1, LOCAL, OTHER);
		local bFound = false;
		for _, pDealItem in ipairs(pDeal:FindItemsByType(type, env.DealItemSubTypes.NONE, player:GetID())) do
			if (pDealItem:GetDuration() ~= 0) == multiTurn then
				iAddAmount = env.clip(pDealItem:GetAmount() + iAddAmount, nil, pDealItem:GetMaxAmount());
				if iAddAmount == pDealItem:GetAmount() then return; end
				pDealItem:SetAmount(iAddAmount);
				bFound = true;
				break;
			end
		end
		if not bFound then
			local pDealItem = pDeal:AddItemOfType(type, player:GetID());
			pDealItem:SetDuration(multiTurn and iDuration or 0);
			iAddAmount = env.clip(iAddAmount, nil, pDealItem:GetMaxAmount());
			if iAddAmount > 0 then
				pDealItem:SetAmount(iAddAmount);
				bFound = true;
			else
				pDeal:RemoveItemByID(pDealItem:GetID());
			end
		end
		if bFound then
			env.UpdateProposedWorkingDeal();
			env.UpdateDealPanel(player);
		end
	end
end
local function ModelGold(env)
	env.OnClickAvailableOneTimeGold = ModelAmountClick(env, GOLD, false);
	env.OnClickAvailableMultiTurnGold = ModelAmountClick(env, GOLD, true);
	env.OnClickAvailableOneTimeFavor = ModelAmountClick(env, FAVOR, false);
end

local function GoldScenarios(label, before)
	local function New(opts)
		opts = opts or {};
		opts.before = before;
		return NewMachine(opts);
	end
	local function Ok(cond, what) Check(cond, label .. ": " .. what); end

	do
		local m = New();
		m.ClickGold();
		Ok(m.Amount() == 100 and m.OneGoldRefresh(100) and m.items[1].type == GOLD, "gold, plain click: 100, as the game");
		m.ClickGold();
		Ok(m.Amount() == 200 and #m.items == 1, "gold, plain click again: 200");
	end

	do
		local m = New({ gold = 2500 });
		m.ClickGold({ ctrl = true });
		Ok(m.Amount() == 1000 and m.OneGoldRefresh(1000) and m.items[1].duration == 0, "gold, Ctrl+click: 1000 (10 x 100)");
		m.ClickGold({ ctrl = true });
		Ok(m.Amount() == 2000, "gold, Ctrl+click again: 2000");
		m.ClickGold();
		Ok(m.Amount() == 2100, "gold, then a plain click: 2100");
		m.ClickGold({ ctrl = true });
		Ok(m.Amount() == 2500 and m.OneGoldRefresh(2500), "gold, Ctrl+click over the treasury: the treasury (2500)");
		m.ClickGold({ ctrl = true });
		Ok(m.Amount() == 2500 and #m.calls == 0, "gold, Ctrl+click at the maximum: nothing, as the game");
	end

	do
		local m = New({ gold = 8431 });
		m.ClickGold({ shift = true });
		Ok(m.Amount() == 8431 and #m.items == 1 and m.OneGoldRefresh(8431), "gold, Shift+click: the whole treasury (8431)");
		m.ClickGold({ shift = true });
		Ok(m.Amount() == 8431 and #m.calls == 0, "gold, Shift+click again: nothing more");
		local m2 = New({ gold = 8431 });
		m2.ClickGold();
		m2.ClickGold();
		m2.ClickGold({ shift = true });
		Ok(m2.Amount() == 8431 and #m2.items == 1, "gold already in the deal, Shift+click: raised to the treasury");
		local m3 = New({ gold = 600 });
		m3.ClickGold({ shift = true, ctrl = true });
		Ok(m3.Amount() == 600, "gold, Shift and Ctrl: Shift wins");
	end

	do
		local m = New({ gold = 0 });
		m.ClickGold({ shift = true });
		Ok(#m.items == 0 and #m.calls == 0, "empty treasury, Shift+click: no item, as the game");
		local m2 = New({ gold = 40 });
		m2.ClickGold({ ctrl = true });
		Ok(m2.Amount() == 40, "40 gold, Ctrl+click: 40");
	end

	do
		local m = New({ gold = 3000 });
		m.ClickGold({ ctrl = true }, m.them);
		Ok(m.Amount() == 1000 and m.items[1].from == OTHER, "the other player's gold, Ctrl+click: 1000 asked from them");
	end

	do
		local m = New({ gpt = 250 });
		m.ClickGpt();
		Ok(m.Amount() == 10 and m.items[1].duration == 30 and m.OneGoldRefresh(10), "gold per turn, plain click: 10 for 30 turns");
		m.ClickGpt({ ctrl = true });
		Ok(m.Amount() == 110 and m.items[1].duration == 30, "gold per turn, Ctrl+click: 100 more (10 x 10)");
		m.ClickGpt({ ctrl = true });
		m.ClickGpt({ ctrl = true });
		Ok(m.Amount() == 250, "gold per turn, Ctrl+click over the maximum: the maximum (250)");
		local m2 = New({ gpt = 73 });
		m2.ClickGpt({ shift = true });
		Ok(m2.Amount() == 73 and m2.items[1].duration == 30 and m2.OneGoldRefresh(73), "gold per turn, Shift+click: the maximum (73), for 30 turns");
	end

	do
		-- Gold and gold per turn are two items: each click raises its own.
		local m = New({ gold = 5000, gpt = 60 });
		m.ClickGold({ ctrl = true });
		m.ClickGpt({ shift = true });
		m.ClickGold({ ctrl = true });
		Ok(#m.items == 2 and m.Amount(1) == 2000 and m.Amount(2) == 60, "gold and gold per turn: two items, each raised by its own click");
	end

	do
		local m = New({ favor = 37 });
		m.ClickFavor();
		Ok(m.Amount() == 1 and m.items[1].type == FAVOR and m.OneGoldRefresh(1), "favor, plain click: 1, as the game");
		m.ClickFavor({ ctrl = true });
		Ok(m.Amount() == 11, "favor, Ctrl+click: 10 more");
		m.ClickFavor({ shift = true });
		Ok(m.Amount() == 37 and m.OneGoldRefresh(37), "favor, Shift+click: all of it (37)");
	end

	do
		local m = New();
		m.env.ms_bIsGift, m.env.ms_bIsDemand, m.env.ms_InitiatedByPlayerID = true, true, OTHER;
		m.ClickGold({ shift = true });
		m.ClickGpt({ ctrl = true });
		m.ClickFavor({ shift = true });
		Ok(#m.items == 0 and #m.calls == 0 and #m.printed == 0, "deal that is not ours: gold and favor untouched");
	end

	do
		-- A resource click with the gold already there: the resource alone moves.
		local m = New({ stock = 20, gold = 900 });
		m.ClickGold({ shift = true });
		m.Click({ shift = true });
		Ok(#m.items == 2 and m.Amount(1) == 900 and m.Amount(2) == 20, "gold then a resource, Shift+click on each: 900 gold, 20 of the resource");
	end
end

GoldScenarios("model click", ModelGold);

-- What reaches the game's click: the amount alone changes, the other arguments pass as they are.
do
	local seen;
	local m = NewMachine({ before = function(env)
		env.OnClickAvailableOneTimeGold = function(...) seen = { n = select("#", ...), ... }; return "answer"; end
		env.OnClickAvailableMultiTurnGold = env.OnClickAvailableOneTimeGold;
	end });
	m.Input({ ctrl = true });
	local answer = m.env.OnClickAvailableMultiTurnGold(m.me, 10, 30, "extra");
	Check(seen.n == 4 and seen[1] == m.me and seen[2] == 100 and seen[3] == 30 and seen[4] == "extra" and answer == "answer",
		"Ctrl: amount times 10, player, duration and the rest untouched, answer returned");
	m.Input({});
	m.env.OnClickAvailableOneTimeGold(m.me, 100);
	Check(seen.n == 2 and seen[2] == 100, "no modifier: the amount of the callback");
	m.Input({ shift = true });
	m.env.OnClickAvailableOneTimeGold(m.me, "x");
	Check(seen[2] == "x", "an amount that is not a number is left alone");
	Check(m.env.OnClickAvailableOneTimeFavor == nil, "no favor click (no XP2): none made up");
end

-- 4. The game's own clicks.
local function RealFunction(file, name)
	local okFile, text = pcall(ReadFile, GAME .. file);
	if not okFile then
		return nil;
	end
	local source = text:gsub("\r\n", "\n"):match("\n(function " .. name .. "%(.-\nend)\n");
	if source == nil then
		return nil;
	end
	return function(env)
		assert(load(StripTypes(source), "@" .. file, "t", env))();
		return env[name];
	end
end
local VIEW = [[\DLC\Expansion2\UI\Replacements\DiplomacyDealView.lua]];
local VIEW_XP2 = [[\DLC\Expansion2\UI\Replacements\DiplomacyDealView_Expansion2.lua]];
local xp2 = RealFunction(VIEW_XP2, "OnClickAvailableResource");
if xp2 ~= nil then
	Scenarios("XP2 click", xp2);
else
	print("game absent, XP2 click skipped");
end
local gold, gpt, favor = RealFunction(VIEW, "OnClickAvailableOneTimeGold"), RealFunction(VIEW, "OnClickAvailableMultiTurnGold"),
	RealFunction(VIEW_XP2, "OnClickAvailableOneTimeFavor");
if gold ~= nil and gpt ~= nil and favor ~= nil then
	GoldScenarios("game's click", function(env)
		gold(env);
		gpt(env);
		favor(env);
	end);
else
	print("game absent, gold and favor clicks skipped");
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
