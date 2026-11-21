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
		freezes = 0, wipes = 0, refused = 0, resyncing = {}, hotJoining = {}, state = 1,
		slots = {}, active = {}, away = {}, endedFor = {} };
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
			IsPaused = function() return m.paused == true; end,
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
				GetSlotStatus = function() return m.slots[id] or 3; end,
			};
		end }),
		Network = {
			GetLocalPlayerID = function() return me; end,
			GetGameHostPlayerID = function() return 0; end,
			IsGameHost = function() return host; end,
			IsPlayerHotJoining = function(id) return m.hotJoining[id] == true; end,
			IsPlayerResyncing = function(id) return m.resyncing[id] == true; end,
			IsPlayerConnected = function(id) return m.away[id] ~= true; end,
			SendChat = function(text) table.insert(m.chat, text); end,
			RequestSnapshot = function() m.snapshots = m.snapshots + 1; end,
			BroadcastPlayerInfo = function() m.broadcasts = (m.broadcasts or 0) + 1; end,
		},
		Game = { GetCurrentGameTurn = function() return game.turn; end },
		SlotStatus = { SS_COMPUTER = 1, SS_TAKEN = 3 },
		PlayerManager = { GetAliveMajorIDs = function() return { 0, 1, 2 }; end },
		Players = setmetatable({}, { __index = function(_, id) return { IsTurnActive = function() return m.active[id] == true; end }; end }),
		Civ6Proxy_EndTurnFor = function(id) table.insert(m.endedFor, id); m.active[id] = false; return 1; end,
		UI = { IsResyncLoadInProgress = function() return m.loading; end },
		ChatTargetTypes = { CHATTARGET_ALL = -1, CHATTARGET_PLAYER = 3 },
		Civ6Proxy_Journal = function(t) table.insert(m.journal, t); end,
		Civ6Proxy_GardeMessages = function() local n = m.freezes; m.freezes = 0; return n; end,
		Civ6Proxy_GardeWipes = function() local n = m.wipes; m.wipes = 0; return n; end,
		Civ6Proxy_DoubleLoadRefused = function() local n = m.refused; m.refused = 0; return n; end,
		Civ6Proxy_LaunchGuarded = function() local s = game.guarded or ""; game.guarded = nil; return s; end,
		Civ6Proxy_SyncNotes = function() local n = m.notes; m.notes = ""; return n; end,
		Civ6Proxy_ReloadAllReady = function() return opts.notReady and 0 or 1; end,
		-- The native side: `m.state` is what it answers; a start makes a reload run.
		Civ6Proxy_ReloadAllState = function() return m.state; end,
		Civ6Proxy_ReloadAllStart = function(turn)
			if m.state == 1 then
				m.started = (m.started or 0) + 1;
				m.startedTurn = turn;
				m.state = 2;
				return 1;
			end
			return m.state;
		end,
		Civ6Proxy_ReloadAllRecheck = function() m.rechecks = (m.rechecks or 0) + 1; end,
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
	check(Logged(c, "freeze prevented for player 1") and not Logged(c, "P1"), "the log has numbers, not names");
	-- Host: players still loading when its load screen closes got no second copy (launch guard):
	-- logged only, since that copy may well have landed during their load.
	local h = NewMachine(true, game);
	game.guarded = "1,2";
	h.resyncing[2] = true;
	h.events.LoadScreenClose.Fire();
	check(#h.chat == 0, "launch guard: nothing said");
	check(Logged(h, "launch guard: no second copy for player(s) 2,") and not Logged(h, "player(s) 1"), "launch guard logged, still loading only");
end

do
	-- A client's request after a desync, at the turn-end sync: the host says why, tells the clients
	-- of 0.12.0 and starts the reload of everyone itself.
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	h.notes = "1|41|1|3|0|units, contracts";
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.chat[1] == "[COLOR_Civ6Yellow]BGE - P1's game is missing 3 objects and has 0 extra compared to the host's: everyone reloads the host's game to prevent P1's desync.[ENDCOLOR]", "line: " .. tostring(h.chat[1]));
	check(h.chat[2] == ".bgd_reload 124", "clients told: " .. tostring(h.chat[2]));
	check(h.started == 1 and h.startedTurn == 124 and h.snapshots == 0, "the reload is started natively, not by a request of the host's own");
	-- The requests of the running reload start nothing more.
	h.events.MultiplayerSnapshotRequested.Fire(2);
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(#h.chat == 2 and h.started == 1, "one reload");
	-- A later desync is taken over too; counts of another turn are not said.
	h.state = 1;
	game.turn = 125;
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.started == 2 and #h.chat == 3 and Last(h) == ".bgd_reload 125", "no counts for this turn: no line, reload");
	h.state = 1;
	h.notes = "1|5|1|0|2|units";
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.started == 3 and h.chat[4]:find("missing 0 objects and has 2 extra", 1, true) ~= nil, "again: " .. tostring(h.chat[4]));
	-- This turn's reloads did not hold: the game's own reload, logged.
	h.state = 3;
	h.notes = "1|2|1|1|0|units";
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.started == 3 and #h.chat == 5 and Logged(h, "still differs after turn 125's reloads"), "third time: left to the game");
	h.state = 4;
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.started == 3 and #h.chat == 5 and Logged(h, "while others still load"), "others still loading: left to the game");
	h.state = 5;
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.started == 3 and #h.chat == 5 and Logged(h, "not every player has this version"), "mixed versions: left to the game");
	-- A player leaving: the host looks again at who it waits for.
	h.events.MultiplayerPrePlayerDisconnected.Fire(1);
	check(h.rechecks == 1, "recheck on a leave");
end

do
	-- Outside a turn-end sync (join, remap, launch): not a desync. Setting off: the game's own reload.
	local game = { values = {}, turn = 124 };
	local h = NewMachine(true, game);
	h.state = 0;
	h.events.MultiplayerSnapshotRequested.Fire(1);
	check(h.started == nil and #h.chat == 0 and Logged(h, "outside a turn-end sync"), "no sync round: nothing started");
	local h2 = NewMachine(true, game, { notReady = true });
	h2.notes = "1|0|1|3|0|";
	h2.events.MultiplayerSnapshotRequested.Fire(1);
	check(h2.started == nil and #h2.chat == 0, "reload_all off: the game reloads that player alone");
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
	c.notes = "3|25|1|4|0|units";
	c.events.GameCoreEventPublishComplete.Fire();
	c.events.MultiplayerSnapshotRequested.Fire(3);
	c.events.MultiplayerPrePlayerDisconnected.Fire(2);
	check(#c.chat == 0 and c.snapshots == 1 and c.started == nil and c.rechecks == nil, "a client announces and reloads nothing itself");
end

do
	-- A player who dropped keeps a human slot: the host ends its turn once every connected human
	-- has ended its own, once per turn.
	local game = { values = {}, turn = 50 };
	local h = NewMachine(true, game);
	h.active = { [0] = true, [1] = true, [2] = true };
	h.away[2] = true;
	h.events.PlayerTurnDeactivated.Fire();
	check(#h.endedFor == 0, "connected players still play: nothing ended");
	h.active[0] = false;
	h.events.PlayerTurnDeactivated.Fire();
	check(#h.endedFor == 0, "one connected player still plays");
	h.active[1] = false;
	h.paused = true;
	h.events.PlayerTurnDeactivated.Fire();
	check(#h.endedFor == 0, "paused: the game would ignore it");
	h.paused = false;
	h.events.PlayerTurnDeactivated.Fire();
	check(#h.endedFor == 1 and h.endedFor[1] == 2 and Logged(h, "player 2 is not connected: its turn was ended by the host"), "the absent player's turn is ended");
	h.active[2] = true;
	h.events.GameCoreEventPublishComplete.Fire();
	check(#h.endedFor == 1, "once per turn");
	game.turn = 51;
	h.events.GameCoreEventPublishComplete.Fire();
	check(#h.endedFor == 2, "again the next turn");
	-- A computer's slot (the game's own takeover, or a kick) is the AI's business.
	local k = NewMachine(true, game);
	k.active = { [2] = true };
	k.away[2] = true;
	k.slots[2] = 1;
	k.events.PlayerTurnDeactivated.Fire();
	check(#k.endedFor == 0, "a computer slot is left alone");
	-- Only the host does it.
	local c = NewMachine(false, game);
	c.active = { [2] = true };
	c.away[2] = true;
	c.events.PlayerTurnDeactivated.Fire();
	check(#c.endedFor == 0, "a client ends nobody's turn");
end

if failures > 0 then
	print(failures .. " failure(s)");
	os.exit(1);
end
print("sync notes: all tests pass");
