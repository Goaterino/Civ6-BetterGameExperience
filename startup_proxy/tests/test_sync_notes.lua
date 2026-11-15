-- Off-game test of src/partie.lua: problems prevented (chat lines) and the host taking a desync over
-- (sync notes, reload of everyone):
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

-- One machine: `host` or client. `game` is shared by the machines of one game: player values
-- (PlayerConfiguration).
local function NewMachine(host, game, opts)
	opts = opts or {};
	local m = { chat = {}, notes = "", clock = 1000, snapshots = 0, loading = false, noSelf = nil, journal = {},
		freezes = 0, wipes = 0, refused = 0, resyncing = {}, hotJoining = {} };
	m.events = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
	local me = host and 0 or 1;
	local env = setmetatable({
		Events = m.events,
		LuaEvents = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end }),
		os = { clock = function() return m.clock; end },
		Locale = { GetCurrentLanguage = function() return { Type = "en_US" }; end, Lookup = function(s) return s; end },
		GameConfiguration = {
			IsNetworkMultiplayer = function() return true; end,
			IsPlayByCloud = function() return false; end,
			GetMultiplayerPlayerIDs = function() return { 0, 1 }; end,
		},
		PlayerConfigurations = setmetatable({}, { __index = function(_, id)
			game.values[id] = game.values[id] or {};
			local v = game.values[id];
			return {
				GetPlayerName = function() return "P" .. id; end,
				GetValue = function(_, k) return v[k]; end,
				SetValue = function(_, k, x) v[k] = x; end,
				IsHuman = function() return true; end,
			};
		end }),
		Network = {
			GetLocalPlayerID = function() return me; end,
			GetGameHostPlayerID = function() return 0; end,
			IsGameHost = function() return host; end,
			IsPlayerHotJoining = function(id) return m.hotJoining[id] == true; end,
			IsPlayerResyncing = function(id) return m.resyncing[id] == true; end,
			IsPlayerConnected = function() return true; end,
			SendChat = function(text) table.insert(m.chat, text); end,
			RequestSnapshot = function() m.snapshots = m.snapshots + 1; end,
			BroadcastPlayerInfo = function() m.broadcasts = (m.broadcasts or 0) + 1; end,
		},
		Game = { GetCurrentGameTurn = function() return game.turn; end },
		UI = { IsResyncLoadInProgress = function() return m.loading; end },
		ChatTargetTypes = { CHATTARGET_ALL = -1, CHATTARGET_PLAYER = 3 },
		Civ6Proxy_Journal = function(t) table.insert(m.journal, t); end,
		Civ6Proxy_GardeMessages = function() local n = m.freezes; m.freezes = 0; return n; end,
		Civ6Proxy_GardeWipes = function() local n = m.wipes; m.wipes = 0; return n; end,
		Civ6Proxy_DoubleLoadRefused = function() local n = m.refused; m.refused = 0; return n; end,
		Civ6Proxy_LaunchGuarded = function() local s = game.guarded or ""; game.guarded = nil; return s; end,
		Civ6Proxy_SyncNotes = function() local n = m.notes; m.notes = ""; return n; end,
		Civ6Proxy_ReloadAllReady = function() return opts.notReady and 0 or 1; end,
		Civ6Proxy_RejoinKeepCheck = function() m.kept = (m.kept or 0) + 1; end,
	}, { __index = _G });
	assert(load(PARTIE, "=partie.lua", "t", env))();
	m.env = env;
	return m;
end

local function Last(m)
	return m.chat[#m.chat] or "";
end

local function Logged(m, needle)
	for _, t in ipairs(m.journal) do
		if t:find(needle, 1, true) then return true; end
	end
	return false;
end

do
	-- Problems prevented: said with the player's name, once a minute per kind.
	local game = { values = {}, turn = 120 };
	local c = NewMachine(false, game);
	c.freezes = 3;
	c.events.GameCoreEventPublishComplete.Fire();
	check(Last(c) == "[COLOR_GREEN]BGE - Game freeze prevented for P1.[ENDCOLOR]", "freeze: " .. Last(c));
	c.freezes = 1;
	c.events.GameCoreEventPublishComplete.Fire();
	check(#c.chat == 1, "a second freeze within a minute is only logged");
	c.wipes = 1;
	c.refused = 1;
	c.events.GameCoreEventPublishComplete.Fire();
	check(c.chat[2] == "[COLOR_GREEN]BGE - Game settings wipe prevented for P1.[ENDCOLOR]", "wipe: " .. tostring(c.chat[2]));
	check(c.chat[3] == "[COLOR_GREEN]BGE - Double load prevented for P1.[ENDCOLOR]", "double load: " .. tostring(c.chat[3]));
	-- Host: players still loading when its load screen closes would have got a second copy.
	local h = NewMachine(true, game);
	game.guarded = "1,2";
	h.resyncing[2] = true;
	h.events.LoadScreenClose.Fire();
	check(Last(h) == "[COLOR_GREEN]BGE - Double load prevented for P2.[ENDCOLOR]", "launch guard: " .. Last(h));
	game.guarded = "1";
	h.clock = h.clock + 100;
	h.events.LoadScreenClose.Fire();
	check(Last(h):find("P1", 1, true) == nil, "a player who had finished loading is not named");
end

do
	-- A client's request after a desync, at the turn-end sync: the host says why, tells the clients
	-- and asks for its own game, at once (the game makes one copy for all that asked).
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	h.events.TurnEnd.Fire();
	h.notes = "1|41|1|3|0|units, contracts";
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.chat[1] == "[COLOR_Civ6Yellow]BGE - P1's game is missing 3 objects and has 0 extra compared to the host's: everyone reloads the host's game to prevent P1's desync.[ENDCOLOR]", "line: " .. tostring(h.chat[1]));
	check(h.chat[2] == ".bgd_reload 124", "clients told: " .. tostring(h.chat[2]));
	check(h.snapshots == 1, "host asks for its own game at once");
	-- The clients' own requests of this reload start nothing more.
	h.events.MultiplayerSnapshotRequested.Fire(2);
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(#h.chat == 2 and h.snapshots == 1, "one reload");
	-- Every desync is taken over, also the same player's again, later.
	h.clock = h.clock + 61;
	h.events.TurnEnd.Fire();
	h.notes = "1|5|1|0|2|units";
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.snapshots == 2 and h.chat[3]:find("missing 0 objects and has 2 extra", 1, true) ~= nil, "again: " .. tostring(h.chat[3]));
	-- Without counts (none noted for that turn end), no line, still the reload.
	h.clock = h.clock + 61;
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.snapshots == 3 and #h.chat == 5 and Last(h) == ".bgd_reload 124", "no counts: no line, reload");
end

do
	-- Joins, rejoins, a remap, the launch: not desyncs. Setting off: the game's own reload.
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.snapshots == 0, "nothing before the first turn end");
	h.events.TurnEnd.Fire();
	h.events.MultiplayerPlayerConnected.Fire(1);
	h.events.MultiplayerSnapshotRequested.Fire(1);
	h.hotJoining[2] = true;
	h.events.MultiplayerSnapshotRequested.Fire(2);
	check(h.snapshots == 0 and #h.chat == 0, "a (re)join is not a desync");
	local h2 = NewMachine(true, game, { notReady = true });
	h2.events.TurnEnd.Fire();
	h2.notes = "1|0|1|3|0|";
	h2.events.MultiplayerSnapshotRequested.Fire(1);
	check(h2.snapshots == 0 and #h2.chat == 0, "reload_all off: the game reloads that player alone");
end

do
	-- Client: the host reloads everyone. It asks once, and keeps its check after the load.
	local game = { values = {}, turn = 124 };
	local c = NewMachine(false, game);
	c.events.MultiplayerChat.Fire(0, 1, ".bgd_reload 124");
	check(c.snapshots == 1 and c.kept == 1, "asks for the copy, keeps its next check");
	-- The desynced client already asked: it does not ask again (one copy, one load).
	local d = NewMachine(false, game);
	d.resyncing[1] = true;
	d.events.MultiplayerChat.Fire(0, 1, ".bgd_reload 124");
	check(d.snapshots == 0 and d.kept == 1, "already waiting: no second request, check kept");
	-- Only the host's signal counts; a client takes nothing over.
	c.events.MultiplayerChat.Fire(2, 1, ".bgd_reload 124");
	check(c.snapshots == 1, "a client's signal is ignored");
	c.events.TurnEnd.Fire();
	c.notes = "3|25|1|4|0|units";
	c.events.GameCoreEventPublishComplete.Fire();
	c.events.MultiplayerSnapshotRequested.Fire(3);
	check(#c.chat == 0 and c.snapshots == 1, "a client announces and reloads nothing itself");
end

if failures > 0 then
	print(failures .. " failure(s)");
	os.exit(1);
end
print("sync notes: all tests pass");
