-- Off-game test of src/ui_accord_luxes.lua (the luxuries a player holds but cannot trade, greyed
-- out in the "available" column of the deal screen):
--   lua proxy/tests/test_ui_accord_luxes.lua [game folder]
-- 1. The real DiplomacyDealView.lua (Expansion2) followed by the texts and the block compiles
--    (HavokScript annotations removed), and what the block wraps and relies on is there.
-- 2. A deal screen reduced to what the luxury list uses: players with copies of their own and
--    copies from elsewhere, possible deal items, a deal that may already hold a luxury, the
--    city-states and the deals in force the tooltip names as sources, and the game's pool of
--    icons. The scenarios run with a list written like the game's, then, when the
--    game is installed, with the real PopulateAvailableResources of DiplomacyDealView.lua.

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
-- The block as the proxy adds it: the texts, BGD_Txt, the block (ACCORD_LUXES_BLOCK, src/ui.rs).
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. "\n" .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. "\n"
	.. ReadFile(dir .. "/../src/ui_accord_luxes.lua");
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
local VIEW = [[\DLC\Expansion2\UI\Replacements\DiplomacyDealView.lua]];

-- 1. The real file.
local okView, view = pcall(ReadFile, GAME .. VIEW);
if okView then
	view = StripTypes(view);
	local chunk, err = load(view .. "\r\n" .. BLOCK, "@DiplomacyDealView.lua");
	Check(chunk ~= nil, "game file followed by the texts and the block: compiles " .. tostring(err or ""));
	for _, name in ipairs({ "PopulateAvailableResources", "GetOtherPlayer", "SetIconToSize" }) do
		Check(view:find("\nfunction " .. name .. "%(") ~= nil, name .. " is a global");
	end
	Check(view:find("\ng_IconOnlyIM = InstanceManager:new( \"IconOnly\"", 1, true) ~= nil, "g_IconOnlyIM is a global, the pool of IconOnly instances");
	Check(view:find("PopulateAvailableResources(player, iconList, \"RESOURCECLASS_LUXURY\")", 1, true) ~= nil,
		"the luxury list is filled by PopulateAvailableResources, looked up by name, with the class the block tests");
	local _, calls = view:gsub("PopulatePlayerAvailablePanel%(", "");
	Check(calls == 3 and view:find("PopulatePlayerAvailablePanel(Controls.MyInventoryStack, g_LocalPlayer)", 1, true) ~= nil
		and view:find("PopulatePlayerAvailablePanel(Controls.TheirInventoryStack, g_OtherPlayer)", 1, true) ~= nil,
		"the available columns are filled from one place, once for each side");
	local okXml, xml = pcall(ReadFile, GAME .. [[\Base\Assets\UI\DiplomacyDealView.xml]]);
	if okXml then
		local instance = xml:match("<Instance%s+Name=\"IconOnly\">(.-)</Instance>") or "";
		local all = true;
		for _, id in ipairs({ "SelectButton", "Icon", "AmountText", "RemoveButton", "StopAskingButton", "UnacceptableIcon" }) do
			all = all and instance:find("ID=\"" .. id .. "\"", 1, true) ~= nil;
		end
		Check(all, "the IconOnly instance has the controls the block sets");
	end
else
	print("game absent, part 1 skipped");
end

-- 2. The list.
local LOCAL, OTHER, THIRD, FOURTH = 0, 1, 2, 3;
local ZANZIBAR, GENEVA = 10, 11;
local NAMES = { [LOCAL] = "LOC_ME", [OTHER] = "LOC_THEM", [THIRD] = "LOC_THIRD", [FOURTH] = "LOC_FOURTH",
	[ZANZIBAR] = "LOC_ZANZIBAR", [GENEVA] = "LOC_GENEVA" };
local RESOURCES = 3;
local LCLICK, RCLICK = 1, 2;
local LUXE, STRATEGIQUE = "RESOURCECLASS_LUXURY", "RESOURCECLASS_STRATEGIC";
local IRON, SILK, TEA, WINE, JADE, COTTON = 1, 2, 3, 4, 5, 6;
local ROWS = {
	{ Index = IRON, ResourceType = "RESOURCE_IRON", ResourceClassType = STRATEGIQUE, Name = "LOC_RESOURCE_IRON_NAME" },
	{ Index = SILK, ResourceType = "RESOURCE_SILK", ResourceClassType = LUXE, Name = "LOC_RESOURCE_SILK_NAME" },
	{ Index = TEA, ResourceType = "RESOURCE_TEA", ResourceClassType = LUXE, Name = "LOC_RESOURCE_TEA_NAME" },
	{ Index = WINE, ResourceType = "RESOURCE_WINE", ResourceClassType = LUXE, Name = "LOC_RESOURCE_WINE_NAME" },
	{ Index = JADE, ResourceType = "RESOURCE_JADE", ResourceClassType = LUXE, Name = "LOC_RESOURCE_JADE_NAME" },
	{ Index = COTTON, ResourceType = "RESOURCE_COTTON", ResourceClassType = LUXE, Name = "LOC_RESOURCE_COTTON_NAME" },
};

-- The luxury list written like the game's (XP2): one icon per possible item of the class with
-- copies to trade, greyed and disabled when the item is not valid, the list hidden when empty.
local function ModelList(env)
	return function(player, iconList, className)
		local availableItemCount = 0;
		local pForDeal = env.DealManager.GetWorkingDeal(env.DealDirection.OUTGOING, env.g_LocalPlayer:GetID(), env.g_OtherPlayer:GetID());
		local possibleResources = env.DealManager.GetPossibleDealItems(player:GetID(), env.GetOtherPlayer(player):GetID(), env.DealItemTypes.RESOURCES, pForDeal);
		if possibleResources ~= nil then
			for _, entry in ipairs(possibleResources) do
				local resourceDesc = env.GameInfo.Resources[entry.ForType];
				if resourceDesc ~= nil and entry.MaxAmount > 0 and resourceDesc.ResourceClassType == className then
					local uiIcon = env.g_IconOnlyIM:GetInstance(iconList.ListStack);
					env.SetIconToSize(uiIcon.Icon, "ICON_" .. resourceDesc.ResourceType);
					uiIcon.AmountText:SetText(tostring(entry.MaxAmount));
					uiIcon.AmountText:SetHide(false);
					uiIcon.RemoveButton:SetHide(true);
					uiIcon.StopAskingButton:SetHide(true);
					uiIcon.Icon:SetColor(1, 1, 1);
					local resourceType = entry.ForType;
					uiIcon.SelectButton:RegisterCallback(env.Mouse.eLClick, function() env.OnClickAvailableResource(player, resourceType); end);
					if entry.IsValid then
						uiIcon.SelectButton:LocalizeAndSetToolTip(resourceDesc.Name);
						uiIcon.SelectButton:SetDisabled(false);
					else
						uiIcon.SelectButton:SetToolTipString(env.Locale.Lookup(resourceDesc.Name) .. "[NEWLINE][COLOR_RED]");
						uiIcon.SelectButton:SetDisabled(true);
						uiIcon.Icon:SetColor(0.5, 0.5, 0.5);
					end
					availableItemCount = availableItemCount + 1;
				end
			end
			iconList.ListStack:CalculateSize();
		end
		iconList.GetTopControl():SetHide(iconList.ListStack:GetSizeX() == 0);
		return availableItemCount;
	end
end

local function NewControl()
	local c = { callbacks = {} };
	function c:SetHide(b) self.hidden = b; end
	function c:SetText(s) self.text = s; end
	function c:SetColor(r, g, b) self.color = { r, g, b }; end
	function c:SetDisabled(b) self.disabled = b; end
	function c:SetToolTipString(s) self.tooltip = s; end
	function c:LocalizeAndSetToolTip(s) self.tooltip = "<" .. s .. ">"; end
	function c:RegisterCallback(event, f) self.callbacks[event] = f; end
	function c:ClearCallback(event) self.callbacks[event] = nil; end
	return c;
end

-- opts.holdings[player][resource] = { own = copies of the player's tiles, other = copies from
--   elsewhere } ;
-- opts.inDeal = { { from, resource, amount } }: resources the deal already holds ;
-- opts.shrink: MaxAmount of a possible item leaves out what the deal already holds ;
-- opts.invalid[resource]: the possible item is not valid (no room under the receiver's cap) ;
-- opts.list(env): the list to wrap (ModelList by default) ; opts.lang: the game's language ;
-- opts.pool: instances waiting in the pool ; opts.before(env): run before the block ;
-- opts.minors = { { id, suzerain, holds = { [resource] = copies }, exports = { ... } } }: the
--   city-states, with the copies each counts as its own and as exported (the game usually
--   counts them as its own) ;
-- opts.deals = { { a, b, items = { { from, resource, amount, duration, enacted } } } }: the deals
--   in force between two major players ; opts.turn: the current turn ;
-- opts.unmet[id]: a player or city-state the local player has not met ;
-- opts.dealsOfOthers: what GetPlayerDeals does for two players other than the local one, which
--   the game never asks: nil (answers), "nothing" (returns nil) or "error".
local function NewMachine(opts)
	opts = opts or {};
	local m = { holdings = opts.holdings or {}, inDeal = opts.inDeal or {}, printed = {}, clicks = {}, possibleCalls = {},
		amountCalls = 0, pool = opts.pool or {}, allocated = {}, dealCalls = {}, exportCalls = 0 };
	local env = setmetatable({}, { __index = _G });
	m.env = env;
	env.print = function(s) m.printed[#m.printed + 1] = tostring(s); end;

	local function Of(id, resource)
		local h = (m.holdings[id] or {})[resource] or {};
		return h.own or 0, h.other or 0;
	end
	local function InDeal(id, resource)
		local n = 0;
		for _, it in ipairs(m.inDeal) do
			if it.from == id and it.resource == resource then n = n + it.amount; end
		end
		return n;
	end

	local function NewPlayer(id)
		local resources = {};
		function resources:GetResourceAmount(resource)
			m.amountCalls = m.amountCalls + 1;
			if m.amountError then error("GetResourceAmount boom"); end
			local own, other = Of(id, resource);
			return own + other;
		end
		return { GetID = function() return id; end, GetResources = function() return resources; end };
	end
	m.me, m.them = NewPlayer(LOCAL), NewPlayer(OTHER);
	env.g_LocalPlayer, env.g_OtherPlayer = m.me, m.them;

	-- The other players, the city-states, and what the local player knows of them.
	local unmet = opts.unmet or {};
	m.me.GetDiplomacy = function() return { HasMet = function(_, id) return not unmet[id]; end }; end
	local majors = { m.me, m.them, NewPlayer(THIRD), NewPlayer(FOURTH) };
	local minors = {};
	for _, minor in ipairs(opts.minors or {}) do
		local resources = {};
		function resources:GetExportedResourceAmount(resource)
			m.exportCalls = m.exportCalls + 1;
			return (minor.exports or {})[resource] or 0;
		end
		function resources:GetResourceAmount(resource)
			return (minor.holds or {})[resource] or 0;
		end
		minors[#minors + 1] = {
			GetID = function() return minor.id; end,
			GetInfluence = function() return { GetSuzerain = function() return minor.suzerain; end }; end,
			GetResources = function() return resources; end,
		};
	end
	env.PlayerManager = {
		GetAliveMajors = function() return majors; end,
		GetAliveMinors = function()
			if m.minorsError then error("GetAliveMinors boom"); end
			return minors;
		end,
	};
	env.PlayerConfigurations = setmetatable({}, { __index = function(_, id)
		return { GetPlayerName = function() return NAMES[id]; end };
	end });
	env.Game = { GetCurrentGameTurn = function() return opts.turn or 20; end };
	env.GetOtherPlayer = function(p) return p == m.them and m.me or m.them; end

	local deal = {};
	function deal:FindItemsByType(type, _, from)
		local t = {};
		for _, it in ipairs(m.inDeal) do
			if type == RESOURCES and it.from == from then
				t[#t + 1] = {
					GetValueType = function() return it.resource; end,
					GetAmount = function() return it.amount; end,
					GetMaxAmount = function() return (Of(from, it.resource)); end,
				};
			end
		end
		return t;
	end
	env.DealManager = {
		GetWorkingDeal = function(direction, a, b)
			m.dealCall = { direction, a, b };
			return deal;
		end,
		GetPossibleDealItems = function(from, to, type, pDeal)
			m.possibleCalls[#m.possibleCalls + 1] = { from, to, type, pDeal == deal };
			local t = {};
			for _, row in ipairs(ROWS) do
				local own = Of(from, row.Index);
				if own > 0 then
					t[#t + 1] = { ForType = row.Index, MaxAmount = own - (opts.shrink and InDeal(from, row.Index) or 0),
						IsValid = not (opts.invalid or {})[row.Index] };
				end
			end
			return t;
		end,
	};
	-- Deals in force between a and b, asked in either order.
	env.DealManager.GetPlayerDeals = function(a, b)
		m.dealCalls[#m.dealCalls + 1] = { a, b };
		if a ~= LOCAL and b ~= LOCAL then
			if opts.dealsOfOthers == "nothing" then return nil; end
			if opts.dealsOfOthers == "error" then error("GetPlayerDeals boom"); end
		end
		local t = {};
		for _, d in ipairs(opts.deals or {}) do
			if (d.a == a and d.b == b) or (d.a == b and d.b == a) then
				t[#t + 1] = { FindItemsByType = function(_, type, _, from)
					local items = {};
					for _, it in ipairs(d.items) do
						if type == RESOURCES and it.from == from then
							items[#items + 1] = {
								GetDuration = function() return it.duration; end,
								GetValueType = function() return it.resource; end,
								GetAmount = function() return it.amount; end,
								GetEnactedTurn = function() return it.enacted; end,
							};
						end
					end
					return items;
				end };
			end
		end
		if #t == 0 then return nil; end
		return t;
	end
	env.DealDirection = { OUTGOING = 1 };
	env.DealItemTypes = { RESOURCES = RESOURCES };
	env.DealItemSubTypes = { NONE = 0 };
	local byIndex = {};
	for _, row in ipairs(ROWS) do byIndex[row.Index] = row; end
	env.GameInfo = { Resources = setmetatable(byIndex, { __call = function()
		local i = 0;
		return function() i = i + 1; return ROWS[i]; end
	end }) };
	env.Mouse = { eLClick = LCLICK, eRClick = RCLICK };
	env.Locale = {
		Lookup = function(s, n) return "<" .. s .. (n ~= nil and (":" .. tostring(n)) or "") .. ">"; end,
		GetCurrentLanguage = function() return { Type = opts.lang or "en_US" }; end,
	};
	env.SetIconToSize = function(iconControl, iconName) iconControl.icon = iconName; end
	env.OnClickAvailableResource = function(player, resourceType) m.clicks[#m.clicks + 1] = { player, resourceType }; end

	-- The pool: GetInstance takes a waiting instance before building one, as it is.
	local im = { m_AllocatedInstances = m.allocated };
	function im:GetInstance(parent)
		local inst = table.remove(m.pool);
		if inst == nil then
			inst = { SelectButton = NewControl(), Icon = NewControl(), AmountText = NewControl(), RemoveButton = NewControl(),
				StopAskingButton = NewControl(), UnacceptableIcon = NewControl() };
		end
		parent.children[#parent.children + 1] = inst;
		m.allocated[#m.allocated + 1] = inst;
		return inst;
	end
	env.g_IconOnlyIM = im;

	function m.NewList()
		local stack = { children = {}, sized = 0 };
		function stack:CalculateSize() self.sized = self.sized + 1; self.size = #self.children * 56; end
		function stack:GetSizeX() return self.size or 0; end
		local top = NewControl();
		return { ListStack = stack, GetTopControl = function() return top; end, top = top };
	end

	env.PopulateAvailableResources = (opts.list or ModelList)(env);
	m.game = env.PopulateAvailableResources;
	if opts.before then opts.before(env); end
	assert(load(BLOCK, "@ui_accord_luxes.lua", "t", env))();
	return m;
end

local function Greyed(inst)
	local c = inst.Icon.color;
	return inst.SelectButton.disabled == true and c ~= nil and c[1] == 0.5 and c[2] == 0.5 and c[3] == 0.5;
end

-- "SILK:2 (TEA:1)": the icons of a list in order, the greyed ones in brackets.
local function Describe(list)
	local t = {};
	for _, inst in ipairs(list.ListStack.children) do
		local s = tostring(inst.Icon.icon):gsub("^ICON_RESOURCE_", "") .. ":" .. tostring(inst.AmountText.text);
		t[#t + 1] = Greyed(inst) and ("(" .. s .. ")") or s;
	end
	return table.concat(t, " ");
end

local HOLDINGS = {
	[LOCAL] = {
		[IRON] = { own = 20, other = 5 },	-- a strategic resource is never listed
		[SILK] = { own = 2 },				-- tiles only
		[TEA] = { own = 1, other = 1 },		-- one tile, one from a city-state
		[WINE] = { other = 1 },				-- city-state only
		[JADE] = { other = 2 },				-- a deal with a third player, and a copy from nowhere known
	},
	[OTHER] = {
		[COTTON] = { own = 3 },
		[SILK] = { other = 1 },				-- a deal with a third player
	},
};
local MINORS = {
	{ id = ZANZIBAR, suzerain = LOCAL, holds = { [TEA] = 1, [WINE] = 1, [IRON] = 5 } },
	{ id = GENEVA, suzerain = THIRD, holds = { [JADE] = 4 }, exports = { [SILK] = 4 } },	-- another player's: never a source
};
local DEALS = {
	{ a = LOCAL, b = THIRD, items = {
		{ from = THIRD, resource = JADE, amount = 1, duration = 30, enacted = 8 },	-- 18 turns left at turn 20
		{ from = THIRD, resource = JADE, amount = 3, duration = 0, enacted = 8 },	-- one-time: not a copy held
		{ from = LOCAL, resource = JADE, amount = 7, duration = 30, enacted = 8 },	-- given, not received
		{ from = THIRD, resource = COTTON, amount = 1, duration = 30, enacted = 8 },	-- not a greyed luxury
	} },
	{ a = OTHER, b = THIRD, items = { { from = THIRD, resource = SILK, amount = 1, duration = 30, enacted = 15 } } },
};
local HEAD = "[NEWLINE]Cannot be traded. Source:";
local function Line(amount, resource, text) return "[NEWLINE][ICON_Bullet]" .. amount .. " [ICON_RESOURCE_" .. resource .. "] " .. text; end
local CITY, DEAL = "<LOC_HUD_REPORTS_CITY_STATE>", "<LOC_HUD_REPORTS_ROW_DIPLOMATIC_DEALS>";
local function Turns(n) return "<LOC_REPORTS_NUMBER_OF_TURNS:" .. n .. ">"; end

local function Scenarios(label, list)
	local function M(opts)
		opts = opts or {};
		opts.list = list;
		if opts.holdings == nil then
			opts.holdings = HOLDINGS;
			if opts.minors == nil then opts.minors = MINORS; end
			if opts.deals == nil then opts.deals = DEALS; end
		end
		return NewMachine(opts);
	end
	local function Is(what, cond) Check(cond, label .. ": " .. what); end

	-- The local player's column.
	do
		local m = M();
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("the untradeable copies follow the tradeable ones, greyed [" .. Describe(l) .. "]", Describe(l) == "SILK:2 TEA:1 (TEA:1) (WINE:1) (JADE:2)");
		Is("the count the game sums is the game's", count == 2);
		Is("the list is shown and sized after the additions", l.top.hidden == false and l.ListStack:GetSizeX() == 5 * 56);
		local tea, twin, wine = l.ListStack.children[2], l.ListStack.children[3], l.ListStack.children[4];
		Is("a tradeable icon keeps the game's click and state", tea.SelectButton.callbacks[LCLICK] ~= nil and tea.SelectButton.disabled == false
			and tea.Icon.color[1] == 1);
		Is("a greyed icon has no click", twin.SelectButton.callbacks[LCLICK] == nil and twin.SelectButton.callbacks[RCLICK] == nil);
		local jade = l.ListStack.children[5];
		Is("its tooltip names the luxury and the city-state it comes from [" .. tostring(wine.SelectButton.tooltip) .. "]",
			wine.SelectButton.tooltip == "<LOC_RESOURCE_WINE_NAME>" .. HEAD .. Line(1, "WINE", CITY .. " (<LOC_ZANZIBAR>)")
			and twin.SelectButton.tooltip == "<LOC_RESOURCE_TEA_NAME>" .. HEAD .. Line(1, "TEA", CITY .. " (<LOC_ZANZIBAR>)"));
		Is("a deal is named with its partner and the turns left, the rest is another source [" .. tostring(jade.SelectButton.tooltip) .. "]",
			jade.SelectButton.tooltip == "<LOC_RESOURCE_JADE_NAME>" .. HEAD .. Line(1, "JADE", DEAL .. " (<LOC_THIRD> " .. Turns(18) .. ")")
			.. Line(1, "JADE", "Other source"));
		Is("its amount is shown, its other controls hidden", wine.AmountText.hidden == false and wine.RemoveButton.hidden == true
			and wine.StopAskingButton.hidden == true and wine.UnacceptableIcon.hidden == true);
		Is("nothing printed", #m.printed == 0);
		tea.SelectButton.callbacks[LCLICK]();
		Is("the tradeable icon still adds its resource", #m.clicks == 1 and m.clicks[1][1] == m.me and m.clicks[1][2] == TEA);
	end

	-- The other side's column.
	do
		local m = M();
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.them, l, LUXE);
		Is("the other side's column too [" .. Describe(l) .. "]", Describe(l) == "COTTON:3 (SILK:1)" and count == 1);
		local last = m.possibleCalls[#m.possibleCalls];
		Is("possible items asked from them to the local player, for the working deal", last[1] == OTHER and last[2] == LOCAL
			and last[3] == RESOURCES and last[4] == true and m.dealCall[1] == 1 and m.dealCall[2] == LOCAL and m.dealCall[3] == OTHER);
	end

	-- Strategic resources.
	do
		local m = M();
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.me, l, STRATEGIQUE);
		Is("the strategic list is the game's [" .. Describe(l) .. "]", Describe(l) == "IRON:20" and count == 1 and m.amountCalls == 0
			and m.exportCalls == 0 and #m.dealCalls == 0);
	end

	-- Nothing to trade, something held: the game hides the list, the block shows it again.
	do
		local m = M({ holdings = { [LOCAL] = { [WINE] = { other = 1 } } } });
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("a player with only untradeable luxuries has a list [" .. Describe(l) .. "]", Describe(l) == "(WINE:1)" and count == 0
			and l.top.hidden == false and l.ListStack:GetSizeX() == 56);
	end

	-- Nothing held.
	do
		local m = M({ holdings = {} });
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("a player without luxuries has no list", Describe(l) == "" and count == 0 and l.top.hidden == true);
	end

	-- Only tradeable copies: nothing added, the list left as the game sized it.
	do
		local m = M({ holdings = { [LOCAL] = { [SILK] = { own = 2 } } } });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("no greyed icon when every copy can be traded [" .. Describe(l) .. "]", Describe(l) == "SILK:2" and l.ListStack.sized == 1);
	end

	-- The deal already holds luxuries (a proposal received): the count is the same whether
	-- MaxAmount leaves out what is in the deal or not.
	for _, shrink in ipairs({ false, true }) do
		local how = shrink and "MaxAmount without what is in the deal" or "MaxAmount with what is in the deal";
		local m = M({ shrink = shrink, inDeal = { { from = LOCAL, resource = TEA, amount = 1 }, { from = LOCAL, resource = SILK, amount = 1 },
			{ from = OTHER, resource = COTTON, amount = 3 } } });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.me, l, LUXE);
		local greyed = Describe(l):match("%(.*$") or "";
		Is("deal holding 1 tea and 1 silk, " .. how .. ": same greyed icons [" .. Describe(l) .. "]", greyed == "(TEA:1) (WINE:1) (JADE:2)");
		local l2 = m.NewList();
		m.env.PopulateAvailableResources(m.them, l2, LUXE);
		greyed = Describe(l2):match("%(.*$") or "";
		Is("deal holding their 3 cotton, " .. how .. ": no greyed cotton [" .. Describe(l2) .. "]", greyed == "(SILK:1)");
	end

	-- A resource the game itself greys (no room under the receiver's cap) stays the game's.
	do
		local m = M({ invalid = { [SILK] = true } });
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("a luxury the game greys is still counted as tradeable [" .. Describe(l) .. "]",
			Describe(l) == "(SILK:2) TEA:1 (TEA:1) (WINE:1) (JADE:2)" and count == 2);
	end

	-- Recycled instances. A greyed icon takes an instance that was a deal item: nothing of it is
	-- left. The next opening gives a greyed instance back to the game's list: usable again.
	do
		local dirty = { SelectButton = NewControl(), Icon = NewControl(), AmountText = NewControl(), RemoveButton = NewControl(),
			StopAskingButton = NewControl(), UnacceptableIcon = NewControl() };
		local clicked = 0;
		dirty.SelectButton:RegisterCallback(LCLICK, function() clicked = clicked + 1; end);
		dirty.SelectButton:RegisterCallback(RCLICK, function() clicked = clicked + 1; end);
		dirty.SelectButton:SetDisabled(false);
		dirty.SelectButton:SetToolTipString("old");
		dirty.Icon:SetColor(1, 1, 1);
		dirty.AmountText:SetHide(true);
		dirty.RemoveButton:SetHide(false);
		dirty.StopAskingButton:SetHide(false);
		dirty.UnacceptableIcon:SetHide(false);
		local m = M({ holdings = { [LOCAL] = { [WINE] = { other = 1 } } }, pool = { dirty } });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.me, l, LUXE);
		local inst = l.ListStack.children[1];
		Is("a greyed icon on a recycled instance keeps nothing of its previous use", inst == dirty and Greyed(inst)
			and inst.SelectButton.callbacks[LCLICK] == nil and inst.SelectButton.callbacks[RCLICK] == nil and inst.SelectButton.tooltip ~= "old"
			and inst.AmountText.hidden == false and inst.RemoveButton.hidden == true and inst.StopAskingButton.hidden == true
			and inst.UnacceptableIcon.hidden == true and inst.Icon.icon == "ICON_RESOURCE_WINE");

		-- Next opening: ResetInstances gives the instance back, the game's list takes it.
		m.holdings = { [LOCAL] = { [SILK] = { own = 1 } } };
		m.pool[1] = inst;
		local l2 = m.NewList();
		m.env.PopulateAvailableResources(m.me, l2, LUXE);
		local again = l2.ListStack.children[1];
		Is("the game's list makes a greyed instance usable again [" .. Describe(l2) .. "]", again == inst and Describe(l2) == "SILK:1"
			and again.SelectButton.disabled == false and again.Icon.color[1] == 1 and again.SelectButton.callbacks[LCLICK] ~= nil);
	end

	-- The other side's sources: a deal between them and a third player, which the game never asks
	-- for. Answered, the deal is named ; not answered or failing, the copy is another source.
	do
		local m = M();
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.them, l, LUXE);
		local tip = l.ListStack.children[2].SelectButton.tooltip;
		Is("their deal with a third player is named [" .. tostring(tip) .. "]",
			tip == "<LOC_RESOURCE_SILK_NAME>" .. HEAD .. Line(1, "SILK", DEAL .. " (<LOC_THIRD> " .. Turns(25) .. ")") and #m.printed == 0);
		local asked = {};
		for _, c in ipairs(m.dealCalls) do asked[#asked + 1] = c[1] .. "-" .. c[2]; end
		Is("deals asked with each other major, the local player first [" .. table.concat(asked, " ") .. "]",
			table.concat(asked, " ") == LOCAL .. "-" .. OTHER .. " " .. OTHER .. "-" .. THIRD .. " " .. OTHER .. "-" .. FOURTH);
	end
	for _, how in ipairs({ "nothing", "error" }) do
		local m = M({ dealsOfOthers = how });
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.them, l, LUXE);
		local tip = l.ListStack.children[2].SelectButton.tooltip;
		Is("deals of two other players give " .. how .. ": the copy is another source [" .. tostring(tip) .. "]",
			Describe(l) == "COTTON:3 (SILK:1)" and count == 1 and tip == "<LOC_RESOURCE_SILK_NAME>" .. HEAD .. Line(1, "SILK", "Other source"));
		Is("deals of two other players give " .. how .. ": Lua.log", how == "nothing" and #m.printed == 0
			or how == "error" and #m.printed == 2 and m.printed[1]:find("BGE accord luxes : accords : echec", 1, true) ~= nil);
		local l2 = m.NewList();
		m.env.PopulateAvailableResources(m.me, l2, LUXE);
		Is("deals of two other players give " .. how .. ": the local player's own deals are still read",
			l2.ListStack.children[5].SelectButton.tooltip:find(DEAL .. " (<LOC_THIRD> " .. Turns(18) .. ")", 1, true) ~= nil);
	end

	-- A luxury they received from the local player.
	do
		local m = M({ deals = { { a = LOCAL, b = OTHER, items = { { from = LOCAL, resource = SILK, amount = 1, duration = 30, enacted = 19 } } } } });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.them, l, LUXE);
		local tip = l.ListStack.children[2].SelectButton.tooltip;
		Is("a luxury the local player gives them is named so [" .. tostring(tip) .. "]",
			tip == "<LOC_RESOURCE_SILK_NAME>" .. HEAD .. Line(1, "SILK", DEAL .. " (<LOC_ME> " .. Turns(29) .. ")"));
	end

	-- A partner or a city-state the local player has not met is not named.
	do
		local m = M({ unmet = { [THIRD] = true, [GENEVA] = true },
			holdings = { [OTHER] = { [SILK] = { other = 1 }, [WINE] = { other = 2 } } },
			minors = { { id = GENEVA, suzerain = OTHER, holds = { [WINE] = 1 }, exports = { [WINE] = 1 } } }, deals = DEALS });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.them, l, LUXE);
		local silk, wine = l.ListStack.children[1].SelectButton.tooltip, l.ListStack.children[2].SelectButton.tooltip;
		Is("an unmet partner is not named [" .. tostring(silk) .. "]", silk == "<LOC_RESOURCE_SILK_NAME>" .. HEAD .. Line(1, "SILK", DEAL .. " (" .. Turns(25) .. ")"));
		Is("an unmet city-state is not named [" .. tostring(wine) .. "]", wine == "<LOC_RESOURCE_WINE_NAME>" .. HEAD .. Line(2, "WINE", CITY));
	end

	-- More copies found than the icon counts: the sources are listed, nothing else.
	do
		local m = M({ holdings = { [LOCAL] = { [WINE] = { other = 1 } } }, minors = { { id = ZANZIBAR, suzerain = LOCAL, exports = { [WINE] = 2 } } } });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.me, l, LUXE);
		local tip = l.ListStack.children[1].SelectButton.tooltip;
		Is("sources above the icon's count: no other source line [" .. tostring(tip) .. "]",
			Describe(l) == "(WINE:1)" and tip == "<LOC_RESOURCE_WINE_NAME>" .. HEAD .. Line(2, "WINE", CITY .. " (<LOC_ZANZIBAR>)"));
	end

	-- The city-states cannot be read: their copies are another source, the deals are still named.
	do
		local m = M();
		m.minorsError = true;
		local l = m.NewList();
		local count = m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("city-states failing: the list is whole, their copies are another source",
			Describe(l) == "SILK:2 TEA:1 (TEA:1) (WINE:1) (JADE:2)" and count == 2
			and l.ListStack.children[4].SelectButton.tooltip == "<LOC_RESOURCE_WINE_NAME>" .. HEAD .. Line(1, "WINE", "Other source")
			and l.ListStack.children[5].SelectButton.tooltip:find(DEAL .. " (<LOC_THIRD> " .. Turns(18) .. ")", 1, true) ~= nil);
		Is("city-states failing: written to Lua.log", #m.printed == 1 and m.printed[1]:find("BGE accord luxes : cites-etats : echec", 1, true) ~= nil);
	end

	-- The texts follow the game's language.
	do
		local m = M({ lang = "fr_FR" });
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("tooltip in the game's language",
			l.ListStack.children[4].SelectButton.tooltip == "<LOC_RESOURCE_WINE_NAME>[NEWLINE]Non échangeable. Source :" .. Line(1, "WINE", CITY .. " (<LOC_ZANZIBAR>)")
			and l.ListStack.children[5].SelectButton.tooltip:find(Line(1, "JADE", "Autre source"), 1, true) ~= nil);
	end

	-- A failure of the additions leaves the game's list and its count.
	do
		local m = M();
		m.amountError = true;
		local l = m.NewList();
		local ok, count = pcall(m.env.PopulateAvailableResources, m.me, l, LUXE);
		Is("a failure leaves the game's list [" .. Describe(l) .. "]", ok and count == 2 and Describe(l) == "SILK:2 TEA:1");
		Is("and is written to Lua.log", #m.printed == 1 and m.printed[1]:find("BGE accord luxes : echec", 1, true) ~= nil);
	end

	-- A second copy of the block (two files including the view) wraps once.
	do
		local m = M();
		local wrapped = m.env.PopulateAvailableResources;
		assert(load(BLOCK, "@ui_accord_luxes.lua", "t", m.env))();
		local l = m.NewList();
		m.env.PopulateAvailableResources(m.me, l, LUXE);
		Is("the block added twice wraps once", m.env.PopulateAvailableResources == wrapped and Describe(l) == "SILK:2 TEA:1 (TEA:1) (WINE:1) (JADE:2)");
	end
end

Scenarios("model list", ModelList);

-- What the block needs is missing: the game's function is left alone.
for _, missing in ipairs({ "GetOtherPlayer", "SetIconToSize", "g_IconOnlyIM" }) do
	local m = NewMachine({ holdings = HOLDINGS, before = function(env) env[missing] = nil; end });
	Check(m.env.PopulateAvailableResources == m.game and #m.printed == 0, "without " .. missing .. ": nothing wrapped, nothing printed");
end
do
	local env = setmetatable({ print = function() end }, { __index = _G });
	local ok = pcall(assert(load(BLOCK, "@ui_accord_luxes.lua", "t", env)));
	Check(ok and env.PopulateAvailableResources == nil and env.BGD_AccordLuxes == nil, "without PopulateAvailableResources: nothing defined");
end

-- The real list of the game.
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
local real = RealFunction(VIEW, "PopulateAvailableResources");
if real ~= nil then
	Scenarios("game's list", real);
else
	print("game absent, game's list skipped");
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
