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
-- (PlayerConfiguration), and the DLL's mark of the host (kept across its own load).
local function NewMachine(host, game, opts)
	opts = opts or {};
	local m = { chat = {}, notes = "", clock = 1000, snapshots = 0, loading = false, noSelf = nil, journal = {},
		freezes = 0, wipes = 0, refused = 0, resyncing = {} };
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
			IsPlayerHotJoining = function() return false; end,
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
		Civ6Proxy_ReloadAllMark = function() game.mark = 0; end,
		Civ6Proxy_ReloadAllTake = function() local a = game.mark; game.mark = nil; return a or -1; end,
		Civ6Proxy_ReloadAllNoSelfReload = function(on) m.noSelf = on; return 1; end,
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
	-- The host announces that it takes reloads over; clients follow it.
	local game = { values = {}, turn = 120 };
	local h = NewMachine(true, game);
	local c = NewMachine(false, game);
	h.events.LoadScreenClose.Fire();
	check(game.values[0].BGD_RELOAD == 1 and h.broadcasts == 1, "host announces it");
	h.events.PlayerInfoChanged.Fire(0);
	check(h.broadcasts == 1, "announced once");
	c.events.PlayerInfoChanged.Fire(0);
	check(c.noSelf == 1, "client waits for the host on a failed check");
	-- A host that cannot take over (setting off): clients reload by themselves.
	local game2 = { values = {}, turn = 120 };
	local h2 = NewMachine(true, game2, { notReady = true });
	local c2 = NewMachine(false, game2);
	h2.events.LoadScreenClose.Fire();
	c2.events.TurnEnd.Fire();
	check(game2.values[0].BGD_RELOAD == 0 and c2.noSelf == 0, "no takeover announced: the game's own reload");
	-- Notes without objects on one side: nothing pending, the start goes.
	h.notes = "1|25|1|0|0|units, cities";
	h.events.GameCoreEventPublishComplete.Fire();
	check(h.env.BGD_ReloadAllStep() == false and #h.chat == 0 and h.snapshots == 0, "value repairs: no reload, no line");
end

do
	-- Objects on one side only: at the turn start the host says it, tells the clients, loads its own
	-- game; after its load, every client loads it once; the start waits until then.
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	local c = NewMachine(false, game);
	h.events.LoadScreenClose.Fire();
	h.notes = "1|41|1|3|0|units, contracts";
	h.events.GameCoreEventPublishComplete.Fire();
	check(#h.chat == 0 and h.snapshots == 0, "nothing during the turn end");
	game.turn = 125;
	check(h.env.BGD_ReloadAllStep() == true, "turn start held");
	check(h.chat[1] == "[COLOR_Civ6Yellow]BGE - P1's game is missing 3 objects and has 0 extra compared to the host's: everyone reloads the host's game to prevent P1's desync.[ENDCOLOR]", "line: " .. tostring(h.chat[1]));
	check(h.chat[2] == ".bgd_reload 125", "clients told: " .. tostring(h.chat[2]));
	check(h.snapshots == 1 and game.mark == 0, "host asks for its own load, the mark kept in the DLL");
	h.env.BGD_ReloadAllStep();
	check(h.snapshots == 1, "asked once");
	-- The client waits, without starting alone.
	c.events.MultiplayerChat.Fire(0, 1, ".bgd_reload 125");
	check(c.env.BGD_ReloadAllStep() == true and c.snapshots == 0, "client waits for the host's game");
	-- The host's fresh interface after its load.
	local h2 = NewMachine(true, game);
	h2.loading = true;
	h2.events.GameCoreEventPublishComplete.Fire();
	check(#h2.chat == 0, "nothing while the host is still loading");
	h2.loading = false;
	h2.events.LoadScreenClose.Fire();
	h2.events.GameCoreEventPublishComplete.Fire();
	check(#h2.chat == 0, "the clients wait for the host's turn start, after its turn updates");
	check(game.mark == nil, "the mark is taken once");
	check(h2.env.BGD_ReloadAllStep() == true and Last(h2) == ".bgd_load 125", "at the held turn start, clients load: " .. Last(h2));
	h2.events.GameCoreEventPublishComplete.Fire();
	h2.events.LocalPlayerTurnBegin.Fire();
	check(#h2.chat == 1, "not twice");
	check(h2.env.BGD_ReloadAllStep() == true, "start held while the client loads");
	h2.events.MultiplayerSnapshotProcessed.Fire(0);
	check(h2.env.BGD_ReloadAllStep() == true, "the host's own processed message changes nothing");
	h2.events.MultiplayerSnapshotProcessed.Fire(1);
	local holding, loaded = h2.env.BGD_ReloadAllStep();
	check(holding == false and loaded ~= nil and loaded[1] == true, "client loaded: start, counted as ready");
	game.turn = 126;
	local _, later = h2.env.BGD_ReloadAllStep();
	check(later == nil, "counted as ready for that turn only");
	game.turn = 125;
	c.events.MultiplayerChat.Fire(0, 1, ".bgd_load 125");
	check(c.snapshots == 1 and c.env.BGD_ReloadAllStep() == false, "client loads the host's game once");
	-- Only the host's signals count.
	c.events.MultiplayerChat.Fire(2, 1, ".bgd_load 125");
	check(c.snapshots == 1, "a client's signal is ignored");
	-- Every desync is taken over, also the same player's again.
	h2.notes = "1|5|1|0|2|units";
	h2.events.GameCoreEventPublishComplete.Fire();
	game.turn = 127;
	check(h2.env.BGD_ReloadAllStep() == true and h2.snapshots == 1, "taken over again");
	check(h2.chat[2]:find("missing 0 objects and has 2 extra", 1, true) ~= nil, "second line: " .. tostring(h2.chat[2]));
end

do
	-- After the host's load, no turn start comes: the clients load after 20 s anyway; without the
	-- hold, the local player's turn start sends them at once.
	local game = { values = {}, turn = 124, mark = 0 };
	local h = NewMachine(true, game);
	h.events.LoadScreenClose.Fire();
	check(#h.chat == 0, "waiting for the turn start");
	h.clock = h.clock + 21;
	h.events.GameCoreEventPublishComplete.Fire();
	check(Last(h) == ".bgd_load 124", "after 20 s: " .. Last(h));
	local h2 = NewMachine(true, { values = {}, turn = 124, mark = 0 });
	h2.events.LoadScreenClose.Fire();
	h2.events.LocalPlayerTurnBegin.Fire();
	check(Last(h2) == ".bgd_load 124", "at the local player's turn start: " .. Last(h2));
end

do
	-- The host's own load never starts: after 30 s the clients load its running game.
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	h.events.LoadScreenClose.Fire();
	h.notes = "1|0|1|1|0|";
	h.env.BGD_ReloadAllStep();
	check(h.snapshots == 1, "asked");
	h.clock = h.clock + 31;
	check(h.env.BGD_ReloadAllStep() == true and Last(h) == ".bgd_load 124", "clients load the running game: " .. Last(h));
	check(game.mark == nil, "mark dropped");
	h.clock = h.clock + 120;
	check(h.env.BGD_ReloadAllStep() == false, "a client that never loads holds the start 120 s at most");
	-- A client told to wait that never gets the host's game reloads by itself after 180 s.
	local c = NewMachine(false, game);
	c.events.MultiplayerChat.Fire(0, 1, ".bgd_reload 124");
	c.clock = c.clock + 181;
	check(c.env.BGD_ReloadAllStep() == false and c.snapshots == 1, "client reloads by itself");
end

do
	-- Without the simultaneous start (no hold), the local player's turn start (after every AI
	-- turn) starts the takeover; the turn's very start, before the AI turns, does not.
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	h.events.LoadScreenClose.Fire();
	h.notes = "1|0|1|2|0|";
	h.events.GameCoreEventPublishComplete.Fire();
	h.events.TurnBegin.Fire();
	check(h.snapshots == 0, "not before the AI turns");
	h.events.LocalPlayerTurnBegin.Fire();
	check(h.snapshots == 1, "taken over at the local player's turn start");
	-- A client sends no line and takes nothing over.
	local c = NewMachine(false, game);
	c.notes = "3|25|1|4|0|units";
	c.events.GameCoreEventPublishComplete.Fire();
	c.events.LocalPlayerTurnBegin.Fire();
	check(#c.chat == 0 and c.snapshots == 0, "a client announces nothing");
end

if failures > 0 then
	print(failures .. " failure(s)");
	os.exit(1);
end
print("sync notes: all tests pass");
