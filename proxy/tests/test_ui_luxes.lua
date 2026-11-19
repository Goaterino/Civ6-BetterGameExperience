-- Off-game test of the `ui_luxes` setting (luxury trade notifications):
--   lua proxy/tests/test_ui_luxes.lua
-- Two contexts sharing LuaEvents, as in the game:
-- - the top panel: src/mods/bandeau.lua loaded after a TopPanel reduced to what the block wraps;
--   its luxury refresh publishes the surplus luxuries of the other players;
-- - the notification panel: src/ui_luxes.lua loaded as the proxy inserts it (texts first), in a
--   NotificationPanel reduced to its instance manager and the functions the block wraps or calls.
-- The test reads what each notification shows, what its clicks do to the working deal, and when
-- it is withdrawn.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local TEXTES = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua");
local BLOCK = ReadFile(dir .. "/../src/ui_luxes.lua");
local EFFACER = ReadFile(dir .. "/../src/mods/notifications.lua");
local BANDEAU = ReadFile(dir .. "/../src/mods/bandeau.lua");

local LOCAL, ALLIE, AUTRE, SPECTATEUR = 0, 1, 2, 3;

-- Luxuries (and one strategic resource), by Index.
local RESSOURCES = {
	{ Index = 10, ResourceType = "RESOURCE_SILK", Name = "Silk", ResourceClassType = "RESOURCECLASS_LUXURY" },
	{ Index = 11, ResourceType = "RESOURCE_TEA", Name = "Tea", ResourceClassType = "RESOURCECLASS_LUXURY" },
	{ Index = 12, ResourceType = "RESOURCE_WINE", Name = "Wine", ResourceClassType = "RESOURCECLASS_LUXURY" },
	{ Index = 13, ResourceType = "RESOURCE_JADE", Name = "Jade", ResourceClassType = "RESOURCECLASS_LUXURY" },
	{ Index = 20, ResourceType = "RESOURCE_IRON", Name = "Iron", ResourceClassType = "RESOURCECLASS_STRATEGIC" },
};
local SILK, TEA, WINE, JADE, IRON = 10, 11, 12, 13, 20;

local LEADERS = {
	[LOCAL] = { type = "LEADER_A", name = "Alpha" },
	[ALLIE] = { type = "LEADER_B", name = "Gilgamesh" },
	[AUTRE] = { type = "LEADER_C", name = "Cleopatra" },
	[SPECTATEUR] = { type = "LEADER_SPECTATOR", name = "Spectator" },
};

-- LuaEvents / Events: any name is an event.
local function NewEvents()
	return setmetatable({}, { __index = function(t, k)
		local ev = { list = {} };
		function ev.Add(f) ev.list[#ev.list + 1] = f; end
		function ev.Remove(f)
			for i, g in ipairs(ev.list) do
				if g == f then table.remove(ev.list, i); return; end
			end
		end
		setmetatable(ev, { __call = function(self, ...)
			local copie = {};
			for i, f in ipairs(self.list) do copie[i] = f; end
			for _, f in ipairs(copie) do f(...); end
		end });
		rawset(t, k, ev);
		return ev;
	end });
end

-- Whatever a block calls that the test does not look at: any method, called, gives 0.
local function Any()
	local t = { calls = {} };
	return setmetatable(t, { __index = function(_, k)
		return function(_, ...) t.calls[k] = { ... }; return 0; end
	end });
end

-- ===========================================================================
-- The world both contexts read: players, resources, deals.
-- ===========================================================================
local function NewWorld()
	local w = {
		localPlayer = LOCAL,
		equipes = { [LOCAL] = 0, [ALLIE] = 0, [AUTRE] = 1 },	-- team game: 0 and 1 together
		vivants = { LOCAL, ALLIE, AUTRE },
		guerre = {},			-- [player] = true: at war with the local player
		rencontres = { [ALLIE] = true, [AUTRE] = true },
		humains = {},
		noms = {},
		-- [player] = { [Index] = amount }
		montants = {
			[LOCAL] = { [SILK] = 1 },
			[ALLIE] = { [SILK] = 3, [TEA] = 2, [WINE] = 1 },
			[AUTRE] = { [JADE] = 2 },
		},
		-- [from player] = { [Index] = MaxAmount } (DealManager.GetPossibleDealItems)
		cessibles = {
			[LOCAL] = { [SILK] = 1 },
			[ALLIE] = { [SILK] = 3, [TEA] = 2, [WINE] = 1 },
			[AUTRE] = { [JADE] = 2 },
		},
		config = {},			-- GameConfiguration.GetValue
		diplomatie = true,		-- CAPABILITY_DIPLOMACY
		enAttente = {},			-- [player] = true: pending deal with the local player
		accords = {},			-- [player] = working deal
		invalides = {},			-- [Index] = true: a deal item of this resource is not valid
		journal = {},			-- what the deal API was asked
		demandes = 0,			-- DealManager.GetPossibleDealItems calls
		luaEvents = NewEvents(),
	};

	local function Player(id)
		local p = {};
		function p:GetTeam() return w.equipes[id]; end
		function p:IsMajor() return w.equipes[id] ~= nil; end
		function p:GetCities() return { Members = function() return function() return nil; end end }; end
		function p:GetDiplomacy()
			return {
				IsAtWarWith = function(_, other) return w.guerre[other] == true or w.guerre[id] == true; end,
				HasMet = function(_, other) return w.rencontres[other] == true; end,
			};
		end
		function p:GetResources()
			return {
				GetResourceAmount = function(_, resourceType)
					for _, r in ipairs(RESSOURCES) do
						if r.ResourceType == resourceType then return (w.montants[id] or {})[r.Index] or 0; end
					end
					return 0;
				end,
				IsResourceVisible = function() return false; end,
			};
		end
		return p;
	end

	function w.Globals(env)
		env.print = function(s) w.journal[#w.journal + 1] = "print " .. tostring(s); end;
		env.Mouse = { eLClick = "L", eRClick = "R" };
		env.LuaEvents = w.luaEvents;
		env.Game = { GetLocalPlayer = function() return w.localPlayer; end };
		env.Players = setmetatable({}, { __index = function(_, id)
			if type(id) ~= "number" or (w.equipes[id] == nil and id ~= SPECTATEUR) then return nil; end
			return Player(id);
		end });
		env.PlayerManager = { GetAliveMajorIDs = function() return w.vivants; end };
		env.PlayerConfigurations = setmetatable({}, { __index = function(_, id)
			local l = LEADERS[id];
			if l == nil then return nil; end
			return {
				GetLeaderTypeName = function() return l.type; end,
				IsHuman = function() return w.humains[id] == true; end,
				GetPlayerName = function() return w.noms[id] or l.name; end,
			};
		end });
		local leaders = {};
		for _, l in pairs(LEADERS) do leaders[l.type] = { Name = l.name }; end
		local resources = setmetatable({}, {
			__index = function(_, k)
				for _, r in ipairs(RESSOURCES) do
					if r.Index == k or r.ResourceType == k then return r; end
				end
				return nil;
			end,
			__call = function()
				local i = 0;
				return function() i = i + 1; return RESSOURCES[i]; end
			end,
		});
		env.GameInfo = { Leaders = leaders, Resources = resources };
		env.Locale = {
			Lookup = function(k, a)
				if a ~= nil then return tostring(k) .. "(" .. tostring(a) .. ")"; end
				return k;
			end,
			ToUpper = string.upper,
			ToNumber = function(v) return tostring(v); end,
			GetCurrentLanguage = function() return { Type = "en_US" }; end,
		};
		env.GameConfiguration = { GetValue = function(k) return w.config[k]; end };
		env.GameCapabilities = { HasCapability = function(k) return k ~= "CAPABILITY_DIPLOMACY" or w.diplomatie; end };
		env.DealDirection = { OUTGOING = "OUT" };
		env.DealItemTypes = { RESOURCES = "RES" };
		env.DealManager = {
			GetWorkingDeal = function(direction, a, b)
				local other = (a == w.localPlayer) and b or a;
				if w.accords[other] == nil then
					local deal = { items = {}, suivant = 0 };
					function deal:AddItemOfType(itemType, from)
						self.suivant = self.suivant + 1;
						local item = { id = self.suivant, itemType = itemType, from = from };
						function item:SetValueType(v) self.value = v; end
						function item:SetAmount(n) self.amount = n; end
						function item:SetDuration(n) self.duration = n; end
						function item:IsValid() return not w.invalides[self.value]; end
						function item:GetID() return self.id; end
						self.items[#self.items + 1] = item;
						return item;
					end
					function deal:RemoveItemByID(id)
						for i, item in ipairs(self.items) do
							if item.id == id then table.remove(self.items, i); return; end
						end
					end
					w.accords[other] = deal;
				end
				return w.accords[other];
			end,
			ClearWorkingDeal = function(direction, a, b)
				w.journal[#w.journal + 1] = "clear " .. b;
				w.accords[b] = nil;
			end,
			HasPendingDeal = function(a, b) return w.enAttente[b] == true; end,
			ViewPendingDeal = function(a, b) w.journal[#w.journal + 1] = "view " .. b; end,
			GetPossibleDealItems = function(from)
				w.demandes = w.demandes + 1;
				local t = {};
				for _, r in ipairs(RESSOURCES) do
					local n = (w.cessibles[from] or {})[r.Index];
					if n ~= nil then t[#t + 1] = { ForType = r.Index, MaxAmount = n }; end
				end
				return t;
			end,
		};
		env.DiplomacyManager = {
			RequestSession = function(a, b, kind) w.journal[#w.journal + 1] = "session " .. b .. " " .. kind; end,
		};
		env.UI = { PlaySound = function() end };
		env.UIManager = { GetScreenSizeVal = function() return 1920, 1080; end };
		env.IconManager = {
			FindIconAtlas = function(_, name, size)
				if name == "ICON_RESOURCE_TOYS" and size == 38 then return 76, 152, "Resources38"; end
				return nil;
			end,
		};
	end

	-- What the deal with `player` holds: "TEA*1/30 WINE*1/30".
	function w.Accord(player)
		local deal = w.accords[player];
		if deal == nil then return nil; end
		local t = {};
		for _, item in ipairs(deal.items) do
			local r = nil;
			for _, row in ipairs(RESSOURCES) do
				if row.Index == item.value then r = row; end
			end
			t[#t + 1] = r.Name .. "*" .. item.amount .. "/" .. item.duration .. "<" .. item.from;
		end
		return table.concat(t, " ");
	end

	function w.Journal()
		local s = table.concat(w.journal, " ; ");
		w.journal = {};
		return s;
	end
	return w;
end

-- ===========================================================================
-- Notification panel: NotificationPanel.lua reduced to what the block uses.
-- ===========================================================================
local function NewPanel(w, opts)
	opts = opts or {};
	local m = { stacks = 0, late = 0, shutdowns = 0, affiches = 0, destroyed = 0 };
	local env = setmetatable({}, { __index = _G });
	w.Globals(env);

	-- A control: any call on a destroyed one is an error, as in the game.
	local METHODES = {};
	function METHODES:SetHide(h) self.hidden = h; end
	function METHODES:RegisterCallback(ev, f) self.cb[ev] = f; end
	function METHODES:ClearCallback(ev) self.cb[ev] = nil; end
	function METHODES:RegisterMouseEnterCallback(f) self.enter = f; end
	function METHODES:RegisterMouseExitCallback(f) self.exit = f; end
	function METHODES:ClearMouseOverCallback() self.enter = nil; end
	function METHODES:ClearMouseExitCallback() self.exit = nil; end
	function METHODES:SetToolTipString(s) self.tip = s; end
	function METHODES:SetSizeVal(x, y) self.x, self.y = x, y; end
	function METHODES:SetSizeX(x) self.x = x; end
	function METHODES:SetSizeY(y) self.y = y; end
	function METHODES:SetTexture(x, y, sheet)
		self.texture = (y == nil) and x or (x .. "," .. y .. "," .. sheet);
	end
	function METHODES:SetOffsetY(y) self.offsetY = y; end
	function METHODES:SetWrapWidth(wrap) self.wrap = wrap; end
	function METHODES:SetString(s) self.text = s; end
	-- 6 px per character, 14 px per line.
	function METHODES:GetSizeX()
		if self.text == nil then return self.x; end
		return math.min(#self.text * 6, self.wrap or math.huge);
	end
	function METHODES:GetSizeY()
		if self.text == nil then return self.y; end
		return 14 * math.max(1, math.ceil(#self.text * 6 / (self.wrap or math.huge)));
	end
	function METHODES:Stop() self.stopped = true; end
	function METHODES:IsStopped() return self.stopped; end
	function METHODES:SetToBeginning() self.pos = "begin"; end
	function METHODES:SetToEnd() self.pos = "end"; end
	function METHODES:SetEndVal(x, y) self.endX = x; end
	local function Control(name, x, y)
		local c = { name = name, cb = {}, x = x or 0, y = y or 0, hidden = false };
		return setmetatable(c, { __index = function(_, k)
			local f = METHODES[k];
			-- A field not set yet (text, texture...), or a method the game does not have: nil.
			if f == nil then return nil; end
			return function(self, ...)
				if self.destroyed then error("destroyed control " .. name); end
				return f(self, ...);
			end
		end });
	end

	-- InstanceManager of the ItemInstance of NotificationPanel.xml.
	local IDS = { "Top", "MouseInArea", "MouseOutArea", "IconBG", "IconBGInvalidPhase", "CountImage", "TitleCount",
		"LeftArrow", "RightArrow", "PagePipStack", "Pages", "TitleInfo", "Summary", "ExpandedArea", "NotificationSlide" };
	-- `pile`: the children of the stack (ScrollStack, StackGrowth="Top": the first one is at the
	-- bottom). As the game's InstanceManager, taking or releasing an instance gives it back to its
	-- parent (ChangeParent), which moves it to the end.
	local im = { m_AllocatedInstances = {}, libres = {}, crees = 0, pile = {} };
	local function EnFin(inst)
		for i, other in ipairs(im.pile) do
			if other == inst then table.remove(im.pile, i); break; end
		end
		table.insert(im.pile, inst);
	end
	function im:GetInstance()
		local inst = table.remove(self.libres);
		if inst == nil then
			inst = { Icon = Control("Icon", 40, 40) };
			for _, id in ipairs(IDS) do inst[id] = Control(id); end
			-- A wrong-phase game notification left its tooltip on the instance.
			inst.MouseInArea.tip = "wrong phase";
			self.crees = self.crees + 1;
		end
		inst.Top.hidden = false;
		table.insert(self.m_AllocatedInstances, inst);
		EnFin(inst);
		return inst;
	end
	function im:ReleaseInstance(inst)
		for i, other in ipairs(self.m_AllocatedInstances) do
			if other == inst then
				table.remove(self.m_AllocatedInstances, i);
				EnFin(inst);
				inst.Top:SetHide(true);
				table.insert(self.libres, inst);
				return;
			end
		end
		error("ReleaseInstance: not allocated");
	end
	function im:DestroyInstances()
		for _, list in ipairs({ self.m_AllocatedInstances, self.libres }) do
			for _, inst in ipairs(list) do
				for _, c in pairs(inst) do
					if type(c) == "table" then c.destroyed = true; end
				end
				m.destroyed = m.destroyed + 1;
			end
		end
		self.m_AllocatedInstances, self.libres, self.pile = {}, {}, {};
	end
	if not opts.sansPanneau then env.m_genericItemIM = im; end
	m.im = im;

	-- NotificationPanel.lua. The game's own notifications: one instance each (m.jeu), released
	-- through ReleaseNotificationEntry before the panel destroys its instances (ClearNotifications).
	m.jeu = {};
	local function Vider()
		while #m.jeu > 0 do env.ReleaseNotificationEntry(); end
		im:DestroyInstances();
	end
	env.ProcessStackSizes = function() m.stacks = m.stacks + 1; end;
	env.OnMouseEnterNotification = function(inst) m.entre = inst; end;
	env.OnMouseExitNotification = function(inst) m.sorti = inst; end;
	env.OnNotificationRefreshRequested = Vider;
	env.OnStartObserverMode = Vider;
	env.OnLocalPlayerChanged = function() m.changes = (m.changes or 0) + 1; end;
	env.OnShutdown = function() m.shutdowns = m.shutdowns + 1; end;
	env.LateInitialize = function() m.late = m.late + 1; end;
	env.ReleaseNotificationEntry = function()
		local inst = table.remove(m.jeu);
		if inst ~= nil then im:ReleaseInstance(inst); end
	end;
	if opts.effacer then
		-- What notifications.lua wraps, and the game's own list (empty: the button has nothing of
		-- the game's to dismiss).
		env.OnDefaultAddNotification = function()
			local inst = im:GetInstance();
			inst.Icon:SetTexture("Notifications40");
			table.insert(m.jeu, inst);
		end;
		env.NotificationManager = { Find = function() return nil; end };
		env.m_notifications = {};
	end

	function m.Load()
		local source = TEXTES .. (opts.effacer and EFFACER or "") .. BLOCK;
		assert(load(source, "=NotificationPanel", "t", env))();
	end
	if not opts.sansCharger then m.Load(); end
	m.env = env;

	-- What an instance is, by its icon: the "clear" button, a luxury notification, the game's.
	local function Role(inst)
		local texture = inst.Icon.texture;
		if texture == "QueueTrash" then return "clear"; end
		if texture == "76,152,Resources38" then return "lux"; end
		return "game";
	end
	-- Luxury notifications shown, in the order they were taken.
	function m.Instances()
		local t = {};
		for _, inst in ipairs(im.m_AllocatedInstances) do
			if not inst.Top.hidden and Role(inst) == "lux" then t[#t + 1] = inst; end
		end
		return t;
	end
	-- The queue as the player sees it, bottom first: "clear lux game".
	function m.File()
		local t = {};
		for _, inst in ipairs(im.pile) do
			if not inst.Top.hidden then t[#t + 1] = Role(inst); end
		end
		return table.concat(t, " ");
	end
	-- The "clear" button: its instance, shown or not.
	function m.Bouton()
		for _, inst in ipairs(im.m_AllocatedInstances) do
			if Role(inst) == "clear" then return inst; end
		end
		return nil;
	end
	function m.Summaries()
		local t = {};
		for _, inst in ipairs(m.Instances()) do t[#t + 1] = inst.Summary.text; end
		return table.concat(t, " | ");
	end
	return m;
end

-- ===========================================================================
-- Top panel: TopPanel.lua reduced to what bandeau.lua wraps or calls.
-- ===========================================================================
local function NewBandeau(w)
	local b = { refreshes = 0 };
	local env = setmetatable({}, { __index = _G });
	w.Globals(env);
	env.Events = NewEvents();
	env.TTManager = { GetTypeControlTable = function() end };
	local function Instance()
		return setmetatable({}, { __index = function(t, k)
			local c = Any();
			rawset(t, k, c);
			return c;
		end });
	end
	env.m_YieldButtonSingleManager = { GetInstance = function() b.single = Instance(); return b.single; end };
	env.m_YieldButtonDoubleManager = { GetInstance = Instance };
	env.Controls = Instance();
	env.ContextPtr = { BuildInstanceForControl = function(_, _, t) t.AmenitiesValue = Any(); t.Top = Any(); end };
	env.YieldTypes = { FOOD = 0, PRODUCTION = 1 };
	env.Round = function(v) return v; end;
	env.RefreshYields = function() end;
	env.RefreshResources = function() end;
	env.LateInitialize = function() end;
	env.OnRefreshYields = function() b.refreshes = b.refreshes + 1; end;
	assert(load(BANDEAU, "=TopPanel", "t", env))();
	env.LateInitialize();
	b.env = env;
	-- The tooltip of the luxury button (the last single button taken).
	function b.Tooltip()
		return b.single.YieldBacking.calls.SetToolTipString[1];
	end
	return b;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Offers as bandeau.lua publishes them.
local function Offres(...)
	local t = {};
	for _, o in ipairs({ ... }) do t[#t + 1] = { joueur = o[1], luxes = o[2] }; end
	return t;
end

local TEXTE_THE = "Gilgamesh has spare luxuries you do not own: [ICON_RESOURCE_TEA] Tea. Click to open the deal.";

-- The notification: what it shows.
do
	local w = NewWorld();
	local m = NewPanel(w);
	Check("loaded: nothing shown, no failure", #m.im.m_AllocatedInstances == 0 and w.Journal() == "");
	local demandes = 0;
	w.luaEvents.BGD_Luxes_Demande.Add(function() demandes = demandes + 1; end);
	m.env.LateInitialize();
	Check("LateInitialize: the game's, then the list is asked for", m.late == 1 and demandes == 1);

	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	local shown = m.Instances();
	local inst = shown[1];
	Check("one offer: one notification", #shown == 1 and m.im.crees == 1 and m.stacks == 1);
	Check("title in capitals, summary names the player and the luxury",
		inst.TitleInfo.text == "LUXURY TRADE AVAILABLE" and inst.Summary.text == TEXTE_THE);
	Check("icon: the toys of the luxury button, in a 38 px slot",
		inst.Icon.texture == "76,152,Resources38" and inst.Icon.x == 38 and inst.Icon.y == 38);
	Check("dressed as a single game notification", inst.IconBG.hidden == false and inst.IconBGInvalidPhase.hidden == true
		and inst.CountImage.hidden and inst.TitleCount.hidden and inst.LeftArrow.hidden and inst.RightArrow.hidden
		and inst.PagePipStack.hidden and inst.Pages.hidden and inst.TitleInfo.offsetY == 5
		and inst.NotificationSlide.stopped and inst.NotificationSlide.pos == "begin" and inst.MouseInArea.tip == "");
	inst.MouseInArea.enter();
	inst.MouseOutArea.exit();
	Check("hover: the game's slide-out", m.entre == inst and m.sorti == inst);
	local largeur = #TEXTE_THE * 6;
	Check("one line: sized as RealizeNotificationSize", largeur > 500 or (inst.ExpandedArea.x == largeur + 20
		and inst.NotificationSlide.endX == largeur - 215 + 20 and inst.MouseOutArea.x == largeur + 70));
	Check("long summary: wrapped, height follows the lines", largeur <= 500
		or (inst.Summary.wrap < largeur and inst.ExpandedArea.y == inst.Summary:GetSizeY() + 42 and inst.Summary:GetSizeY() > 14));

	-- Same list again, then a longer one: the same instance, rewritten.
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("same list: nothing moves", #m.Instances() == 1 and m.im.crees == 1 and m.stacks == 1);
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA, WINE } }));
	Check("one more luxury: same notification, new text", #m.Instances() == 1 and m.im.crees == 1 and m.stacks == 1
		and inst.Summary.text == "Gilgamesh has spare luxuries you do not own: [ICON_RESOURCE_TEA] Tea, [ICON_RESOURCE_WINE] Wine. Click to open the deal.");

	-- A second player: its own notification, in player order whatever the list order.
	w.humains[AUTRE], w.noms[AUTRE] = true, "Bob";
	w.luaEvents.BGD_Luxes_Offres(Offres({ AUTRE, { JADE } }, { ALLIE, { TEA } }));
	shown = m.Instances();
	Check("two players: two notifications", #shown == 2 and m.stacks == 2);
	Check("a human: the player's name, then the leader's",
		shown[2].Summary.text == "Bob (Cleopatra) has spare luxuries you do not own: [ICON_RESOURCE_JADE] Jade. Click to open the deal.");

	-- Withdrawn when the list no longer has the player.
	w.luaEvents.BGD_Luxes_Offres(Offres({ AUTRE, { JADE } }));
	shown = m.Instances();
	Check("offer gone: notification withdrawn", #shown == 1 and shown[1].Summary.text:find("Bob", 1, true) == 1 and m.stacks == 3);
	Check("released as the game expects it: icon back to 40 px, hover cleared",
		inst.Top.hidden and inst.Icon.x == 40 and inst.Icon.y == 40 and inst.MouseInArea.enter == nil
		and inst.MouseOutArea.exit == nil and inst.MouseOutArea.hidden);
	w.luaEvents.BGD_Luxes_Offres({});
	Check("empty list: nothing left", #m.Instances() == 0 and #m.im.m_AllocatedInstances == 0);
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("offer back: shown again, instance reused", #m.Instances() == 1 and m.im.crees == 2);
	Check("no failure written", w.Journal() == "");
end

-- Left click: the deal.
do
	local w = NewWorld();
	local m = NewPanel(w);
	m.env.LateInitialize();
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA, WINE } }, { AUTRE, { JADE } }));
	local shown = m.Instances();
	-- A deal left over from an earlier visit of the deal screen.
	m.env.DealManager.GetWorkingDeal("OUT", LOCAL, ALLIE):AddItemOfType("RES", LOCAL):SetValueType(SILK);
	shown[1].MouseInArea.cb.L();
	Check("click: working deal cleared, then the session is requested", w.Journal() == "clear 1 ; session 1 MAKE_DEAL");
	Check("click: one copy of each luxury, from the other player, 30 turns", w.Accord(ALLIE) == "Tea*1/30<1 Wine*1/30<1");
	Check("click: the notification stays until the trade is made", #m.Instances() == 2);

	w.accords = {};
	shown[2].MouseOutArea.cb.L();
	Check("click on the text: the same, for that player", w.Journal() == "clear 2 ; session 2 MAKE_DEAL" and w.Accord(AUTRE) == "Jade*1/30<2"
		and w.Accord(ALLIE) == nil);

	-- A luxury that can no longer be traded is left out; the deal still opens.
	w.accords, w.invalides = {}, { [TEA] = true };
	shown[1].MouseInArea.cb.L();
	Check("invalid item: removed from the deal", w.Accord(ALLIE) == "Wine*1/30<1" and w.Journal() == "clear 1 ; session 1 MAKE_DEAL");
	w.invalides = {};

	-- A deal already proposed to that player: shown, not replaced.
	w.accords, w.enAttente = {}, { [ALLIE] = true };
	shown[1].MouseInArea.cb.L();
	Check("pending deal: viewed, working deal untouched", w.Journal() == "view 1" and w.Accord(ALLIE) == nil);
	w.enAttente = {};

	w.diplomatie = false;
	shown[1].MouseInArea.cb.L();
	Check("no diplomacy: click does nothing", w.Journal() == "");
end

-- Right click, and the "clear" button of notifications.lua.
do
	local w = NewWorld();
	local m = NewPanel(w, { effacer = true });
	m.env.LateInitialize();
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }, { AUTRE, { JADE } }));
	local shown = m.Instances();
	shown[1].MouseInArea.cb.R();
	Check("right click: dismissed, the other one stays", #m.Instances() == 1 and m.Instances()[1] == shown[2] and w.Journal() == "");
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }, { AUTRE, { JADE } }));
	Check("same offer published again: stays dismissed", #m.Instances() == 1);
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA, WINE } }, { AUTRE, { JADE } }));
	Check("a new luxury from that player: back, with both", #m.Instances() == 2
		and m.Summaries():find("[ICON_RESOURCE_TEA] Tea, [ICON_RESOURCE_WINE] Wine", 1, true) ~= nil);
	m.Instances()[2].MouseOutArea.cb.R();
	w.luaEvents.BGD_Luxes_Offres(Offres({ AUTRE, { JADE } }));
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }, { AUTRE, { JADE } }));
	Check("offer gone then back: the dismissal is forgotten", #m.Instances() == 2);

	Check("notifications.lua: BGD_Afficher and the hook are there", type(m.env.BGD_Afficher) == "function"
		and type(m.env.BGD_Luxes_ToutFermer) == "function");
	m.env.BGD_Luxes_ToutFermer();
	Check("clear button: every luxury notification dismissed, the button is all that is left, hidden",
		#m.Instances() == 0 and #m.im.m_AllocatedInstances == 1 and m.Bouton().Top.hidden and m.File() == "");
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }, { AUTRE, { JADE } }));
	Check("clear button: they stay dismissed", #m.Instances() == 0);
	Check("no failure written", w.Journal() == "");
end

-- The "clear" button of notifications.lua stays at the bottom of the queue.
do
	local w = NewWorld();
	local m = NewPanel(w, { effacer = true });
	m.env.LateInitialize();
	Check("nothing in the panel: no button yet", m.Bouton() == nil and m.File() == "");

	-- A luxury notification before any notification of the game: the button comes first.
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("luxury notification first: the button is made before it, and shown", m.File() == "clear lux");
	m.env.OnDefaultAddNotification();
	Check("then one of the game: above both", m.File() == "clear lux game");
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }, { AUTRE, { JADE } }));
	Check("a second luxury notification: at the top", m.File() == "clear lux game lux");

	-- Withdrawn and back: the instance is reused, the button does not move.
	w.luaEvents.BGD_Luxes_Offres({});
	Check("offers gone: the button and the game's", m.File() == "clear game");
	w.luaEvents.BGD_Luxes_Offres(Offres({ AUTRE, { JADE } }));
	Check("offer back on a reused instance: above, the button still at the bottom", m.File() == "clear game lux"
		and m.im.crees == 4);

	-- The button itself.
	local bouton = m.Bouton();
	bouton.MouseInArea.cb.L();
	Check("click on the button: luxury notification dismissed, the game's stays", m.File() == "clear game");
	m.env.ReleaseNotificationEntry();
	Check("nothing left to clear: the button hides", m.File() == "" and bouton.Top.hidden);
	m.env.OnDefaultAddNotification();
	Check("a notification of the game: the same button, back at the bottom", m.File() == "clear game" and m.Bouton() == bouton);

	-- The game destroys the instances of the panel: the button is made again, first.
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("before the refresh", m.File() == "clear game lux");
	m.env.OnNotificationRefreshRequested();
	Check("refresh requested: a new button, below the luxury notification built again",
		m.File() == "clear lux" and m.Bouton() ~= bouton and w.Journal() == "");
	local ok = pcall(m.env.OnDefaultAddNotification);
	Check("after the refresh: the game's notifications come back above", ok and m.File() == "clear lux game");

	bouton = m.Bouton();
	m.env.OnStartObserverMode();
	Check("observer mode: nothing kept", m.File() == "" and m.Bouton() == nil and w.Journal() == "");
	ok = pcall(m.env.OnDefaultAddNotification);
	Check("after observer mode: a new button under the game's notification", ok and m.File() == "clear game"
		and m.Bouton() ~= bouton);
	Check("no failure written", w.Journal() == "");

	-- Without notifications.lua (ui_notifications=off): no button, nothing asked of it.
	local w2 = NewWorld();
	local m2 = NewPanel(w2);
	m2.env.LateInitialize();
	w2.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("without the clear button: the luxury notification alone", m2.File() == "lux" and m2.env.BGD_Bouton == nil
		and w2.Journal() == "");
end

-- The panel's own life: refresh, observer, hotseat, spectator, shutdown.
do
	local w = NewWorld();
	local m = NewPanel(w);
	m.env.LateInitialize();
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }, { AUTRE, { JADE } }));
	m.Instances()[2].MouseInArea.cb.R();
	local avant = m.Instances()[1];
	m.env.OnNotificationRefreshRequested();
	local apres = m.Instances();
	Check("refresh requested: instances destroyed by the game, the notification is built again",
		m.destroyed == 2 and #apres == 1 and apres[1] ~= avant and apres[1].Summary.text == TEXTE_THE and w.Journal() == "");

	local demandes = 0;
	w.luaEvents.BGD_Luxes_Demande.Add(function() demandes = demandes + 1; end);
	m.env.OnLocalPlayerChanged();
	Check("local player changed: the game's, notifications withdrawn, list asked again",
		m.changes == 1 and #m.Instances() == 0 and demandes == 1);

	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	m.env.OnStartObserverMode();
	Check("observer mode: nothing kept", #m.Instances() == 0 and w.Journal() == "");
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("after observer mode: a new list is shown", #m.Instances() == 1);

	w.localPlayer = -1;
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("no local player: nothing shown", #m.Instances() == 0);
	w.localPlayer = SPECTATEUR;
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("spectator: nothing shown", #m.Instances() == 0);
	w.localPlayer, w.diplomatie = LOCAL, false;
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("no diplomacy: nothing shown", #m.Instances() == 0);
	w.diplomatie = true;

	w.luaEvents.BGD_Luxes_Offres("x");
	w.luaEvents.BGD_Luxes_Offres({ { joueur = "x" }, { joueur = ALLIE, luxes = {} }, 3 });
	Check("malformed list: ignored, no failure", #m.Instances() == 0 and w.Journal() == "");

	m.env.OnShutdown();
	w.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("shutdown: the game's, and the list is no longer listened to", m.shutdowns == 1 and #m.Instances() == 0);

	-- Loaded twice (hot reload of the block): wrapped once.
	local w2 = NewWorld();
	local m2 = NewPanel(w2);
	m2.Load();
	m2.env.LateInitialize();
	w2.luaEvents.BGD_Luxes_Offres(Offres({ ALLIE, { TEA } }));
	Check("loaded twice: wrapped once", m2.late == 1 and #m2.Instances() == 1);

	local vide = NewPanel(NewWorld(), { sansPanneau = true });
	Check("without the panel's instance manager: nothing done, no failure", vide.env.BGD_Luxes_Etat == nil and m.late == 1);
end

-- The list: what bandeau.lua publishes, and when.
do
	local w = NewWorld();
	local b = NewBandeau(w);
	local recues = {};
	w.luaEvents.BGD_Luxes_Offres.Add(function(offres) recues[#recues + 1] = offres; end);
	local function Derniere()
		local t = {};
		for _, o in ipairs(recues[#recues]) do t[#t + 1] = o.joueur .. ":" .. table.concat(o.luxes, ","); end
		return table.concat(t, ";");
	end

	b.env.RefreshYields();
	-- Team game: the ally has 3 silk (owned by the local player), 2 tea, 1 wine; the other team has 2 jade.
	Check("published: the ally's surplus luxuries the local player lacks", #recues == 1 and Derniere() == ALLIE .. ":" .. TEA);
	Check("the tooltip lists the same", b.Tooltip():find("Gilgamesh[NEWLINE][ICON_RESOURCE_TEA] Tea", 1, true) ~= nil
		and b.Tooltip():find("Jade", 1, true) == nil);
	b.env.RefreshYields();
	Check("same list: not published again", #recues == 1);

	w.luaEvents.BGD_Luxes_Demande();
	Check("asked by the notification panel: published again", #recues == 2 and Derniere() == ALLIE .. ":" .. TEA);

	-- The ally improves a second wine: only the luxury button is redrawn.
	w.montants[ALLIE][WINE], w.cessibles[ALLIE][WINE] = 2, 2;
	b.env.Events.PlayerResourceChanged(ALLIE, WINE);
	Check("an ally's luxury changes: list recomputed, top panel not refreshed",
		#recues == 3 and Derniere() == ALLIE .. ":" .. TEA .. "," .. WINE and b.refreshes == 0);
	local demandes = w.demandes;
	b.env.Events.PlayerResourceChanged(ALLIE, IRON);
	b.env.Events.PlayerResourceChanged(AUTRE, JADE);
	b.env.Events.PlayerResourceChanged(99, TEA);
	Check("a strategic resource, another team, an unknown player: nothing recomputed", #recues == 3 and w.demandes == demandes);
	b.env.Events.PlayerResourceChanged(LOCAL, TEA);
	Check("the local player's resources: the game's refresh, as before", b.refreshes == 1);

	-- The local player now owns tea (the trade was made).
	w.montants[LOCAL][TEA] = 1;
	b.env.RefreshYields();
	Check("luxury now owned: out of the list", #recues == 4 and Derniere() == ALLIE .. ":" .. WINE);

	-- The ally can only give one copy (the rest is already promised).
	w.cessibles[ALLIE][WINE] = 1;
	b.env.RefreshYields();
	Check("no second copy to give: list empty", #recues == 5 and Derniere() == "");
	w.cessibles[ALLIE][WINE] = 2;
	b.env.RefreshYields();

	w.guerre[ALLIE] = true;
	b.env.Events.DiplomacyDeclareWar(LOCAL, ALLIE);
	Check("war: list empty", #recues == 7 and Derniere() == "");
	w.guerre[ALLIE] = nil;
	b.env.Events.DiplomacyMakePeace(LOCAL, ALLIE);
	Check("peace: back in the list", #recues == 8 and Derniere() == ALLIE .. ":" .. WINE);

	-- Without teams, every met player at peace counts.
	local ffa = NewWorld();
	ffa.equipes = { [LOCAL] = 0, [ALLIE] = 1, [AUTRE] = 2 };
	ffa.rencontres[AUTRE] = nil;
	local bf = NewBandeau(ffa);
	local liste = nil;
	ffa.luaEvents.BGD_Luxes_Offres.Add(function(offres) liste = offres; end);
	bf.env.RefreshYields();
	Check("free for all: met players only", #liste == 1 and liste[1].joueur == ALLIE);
	ffa.rencontres[AUTRE] = true;
	bf.env.Events.DiplomacyMeet(LOCAL, AUTRE);
	Check("free for all: a player met joins the list", #liste == 2 and liste[2].joueur == AUTRE and liste[2].luxes[1] == JADE);

	-- "No luxury trading" of Team PVP Tools.
	local sans = NewWorld();
	sans.config.TPT_NO_TRADING_LUXURIES = true;
	local bs = NewBandeau(sans);
	local n, derniere = 0, nil;
	sans.luaEvents.BGD_Luxes_Offres.Add(function(offres) n, derniere = n + 1, offres; end);
	bs.env.RefreshYields();
	Check("no luxury trading: empty list", n == 1 and #derniere == 0);

	-- Asked before the first refresh (the luxury button does not exist yet): an empty list.
	local tot = NewWorld();
	local bt = NewBandeau(tot);
	local premiere = nil;
	tot.luaEvents.BGD_Luxes_Offres.Add(function(offres) premiere = offres; end);
	tot.luaEvents.BGD_Luxes_Demande();
	Check("asked before the first refresh: empty list, no failure", premiere ~= nil and #premiere == 0 and tot.Journal() == "");
	bt.env.RefreshYields();
	Check("then the first refresh publishes", #premiere == 1);
end

-- Both contexts together.
do
	local w = NewWorld();
	local m = NewPanel(w);
	local b = NewBandeau(w);
	-- The top panel refreshes before the notification panel is ready, as when it loads first.
	b.env.RefreshYields();
	Check("panel not ready: nothing shown yet", #m.Instances() == 0);
	m.env.LateInitialize();
	Check("panel ready: it asks, the notification appears", #m.Instances() == 1 and m.Summaries() == TEXTE_THE);
	m.Instances()[1].MouseInArea.cb.L();
	Check("click: tea in the deal with the ally", w.Accord(ALLIE) == "Tea*1/30<1");
	-- The deal is accepted: the local player owns tea.
	w.montants[LOCAL][TEA] = 1;
	b.env.RefreshYields();
	Check("trade made: notification withdrawn", #m.Instances() == 0 and #m.im.m_AllocatedInstances == 0);
	Check("no failure written", w.Journal():find("print", 1, true) == nil);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
