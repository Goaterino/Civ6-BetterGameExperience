-- Off-game test of the host's desync chat lines in src/partie.lua (sync notes):
--   lua startup_proxy/tests/test_sync_notes.lua
local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local PARTIE = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/partie.lua");

local failures = 0;
local function check(cond, what)
	if not cond then
		failures = failures + 1;
		print("FAILED: " .. what);
	end
end

local function NewEvent()
	local e = { handlers = {} };
	e.Add = function(h) table.insert(e.handlers, h); end
	e.Fire = function(...) for _, h in ipairs(e.handlers) do h(...); end end
	return e;
end

-- One machine: `host` or client, with the notes its hook would hand out.
local function NewMachine(host)
	local m = { chat = {}, notes = "", clock = 1000, hotJoining = {} };
	m.events = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
	local env = setmetatable({
		Events = m.events,
		LuaEvents = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end }),
		os = { clock = function() return m.clock; end },
		Locale = { GetCurrentLanguage = function() return { Type = "en_US" }; end, Lookup = function(s) return s; end },
		GameConfiguration = { IsNetworkMultiplayer = function() return true; end, IsPlayByCloud = function() return false; end },
		PlayerConfigurations = setmetatable({}, { __index = function(_, id)
			return { GetPlayerName = function() return "P" .. id; end, GetValue = function() return nil; end, IsHuman = function() return true; end };
		end }),
		Network = {
			GetLocalPlayerID = function() return host and 0 or 1; end,
			GetGameHostPlayerID = function() return 0; end,
			IsGameHost = function() return host; end,
			IsPlayerHotJoining = function(id) return m.hotJoining[id] == true; end,
			SendChat = function(text) table.insert(m.chat, text); end,
		},
		ChatTargetTypes = { CHATTARGET_ALL = -1, CHATTARGET_PLAYER = 3 },
		Civ6Proxy_Journal = function() end,
		Civ6Proxy_SyncNotes = function() local n = m.notes; m.notes = ""; return n; end,
	}, { __index = _G });
	assert(load(PARTIE, "=partie.lua", "t", env))();
	return m;
end

local function Last(m)
	return m.chat[#m.chat] or "";
end

do
	local h = NewMachine(true);
	h.events.TurnEnd.Fire();
	-- Small repairs of major objects: logged, not said.
	h.notes = "3|2|1|units";
	h.events.GameCoreEventPublishComplete.Fire();
	check(#h.chat == 0, "a small repair is not announced");
	-- A large one: said once, not again while it repeats.
	h.notes = "3|25|1|cities, units";
	h.events.GameCoreEventPublishComplete.Fire();
	check(Last(h) == "[COLOR_Civ6Yellow]BGE - P3's game was out of sync (cities, units) and was repaired without a reload.[ENDCOLOR]", "large repair: " .. Last(h));
	h.notes = "3|25|2|cities, units";
	h.events.GameCoreEventPublishComplete.Fire();
	check(#h.chat == 1, "a repeated large repair is not announced again");
	-- A reload after a desync, with the categories of that turn end, then the return.
	h.notes = "4|3|1|units, contracts";
	h.events.MultiplayerSnapshotRequested.Fire(4);
	check(Last(h) == "[COLOR_Civ6Yellow]BGE - P4's game was out of sync (units, contracts) and is being reloaded from the host.[ENDCOLOR]", "reload: " .. Last(h));
	h.events.MultiplayerSnapshotRequested.Fire(4);
	check(#h.chat == 2, "one line per reload");
	h.events.MultiplayerSnapshotProcessed.Fire(4);
	check(Last(h) == "[COLOR_GREEN]BGE - P4 is back in sync.[ENDCOLOR]", "back: " .. Last(h));
	-- Without known categories.
	h.clock = h.clock + 100;
	h.events.MultiplayerSnapshotRequested.Fire(4);
	check(Last(h):find("(game state)", 1, true) ~= nil, "reload without categories: " .. Last(h));
	h.events.MultiplayerSnapshotProcessed.Fire(4);
	-- Joins and rejoins are not desyncs.
	local n = #h.chat;
	h.events.MultiplayerPlayerConnected.Fire(5);
	h.events.MultiplayerSnapshotRequested.Fire(5);
	h.hotJoining[6] = true;
	h.events.MultiplayerSnapshotRequested.Fire(6);
	h.events.MultiplayerSnapshotProcessed.Fire(5);
	check(#h.chat == n, "a join or rejoin is not announced");
end

do
	-- A client says nothing, and nothing is said before the first turn end (launch, remap).
	local c = NewMachine(false);
	c.events.TurnEnd.Fire();
	c.notes = "3|25|1|units";
	c.events.GameCoreEventPublishComplete.Fire();
	c.events.MultiplayerSnapshotRequested.Fire(3);
	check(#c.chat == 0, "a client announces nothing");
	local h = NewMachine(true);
	h.events.MultiplayerSnapshotRequested.Fire(3);
	check(#h.chat == 0, "nothing before the first turn end");
end

if failures > 0 then
	print(failures .. " failure(s)");
	os.exit(1);
end
print("sync notes: all tests pass");
