-- ===========================================================================
-- BetterGameDLLs : relais de l'hôte en cours de partie, ajouté à la fin de InGame.lua (ou de son
-- remplaçant du même nom, celui de MPH par exemple) par le proxy de démarrage
-- (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_PolActifs()     -> 1 si les correctifs POL sont actifs sur cette machine, sinon 0
--   Civ6Proxy_Journal(texte)  -> écrit dans RolloverProxy.log
--   Civ6Proxy_GardeMessages() -> reads fixed by the network message guard since the last call
--                                (see below)
--   Civ6Proxy_RelayedPairs()  -> pairs of players relayed through the host, "a-b,c-d"
--   Civ6Proxy_RelinkStart(partner, steamID) -> 1 if the direct link attempt started
--   Civ6Proxy_RelinkState(partner) -> 0 in progress, 1 linked directly, 2 failed
--   Civ6Proxy_RelayForgetPair(a, b) -> removes the pair from the local relay table (see below)
--   Civ6Proxy_SyncNotes()     -> major objects repaired for a client at a turn end (host, see below)
--
-- Un joueur qui rejoint la partie en cours, ou s'y reconnecte, passe par le salon : il tire un
-- nouveau jeton et ne croit le verdict de l'hôte que s'il l'y trouve (salon.lua). Or l'hôte est en
-- jeu, son salon ne répond plus. Ce bloc le remplace : chaque fois qu'un humain connecté annonce la
-- version et le jeton de l'hôte, l'hôte ajoute son jeton au verdict de la partie, sans changer
-- l'état des correctifs POL (on ne peut pas les activer ou les couper en pleine partie sans
-- désynchroniser). Le nouveau venu conclut alors comme tout le monde.
-- Only the machine that launched the game has a verdict. A machine that becomes host by migration
-- writes its own: its version, the lobby token of its announcement (the one every player of this
-- game announces), its own POL state, and the tokens of the players present. Without it, nobody
-- joining after a migration would find its token and all would run with POL off.
-- Si les correctifs POL sont actifs et qu'un humain se met prêt sans s'être annoncé (sans le proxy
-- ou d'une autre version), l'hôte le dit dans le chat : sa partie va se désynchroniser. Les couper pour tous demanderait de
-- le faire au même point de la simulation sur chaque machine, ce qui n'est pas encore fait.
-- ===========================================================================

local BGD_PARTIE = { avertis = {} };

local function BGD_PartieJournal(texte)
	if Civ6Proxy_Journal ~= nil then
		Civ6Proxy_Journal("partie : " .. texte);
	end
end

local function BGD_PartieRelais()
	if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud()
		or not Network.IsGameHost() then
		return;
	end
	local localID = Network.GetLocalPlayerID();
	local cfg = PlayerConfigurations[localID];
	if cfg == nil then return; end
	-- Le verdict de la partie : "<version>@<jeton>:<1|0>:<jetons>", écrit par notre salon.
	local verdict = cfg:GetValue("BGD_VERDICT");
	local version, jeton, actif, liste = string.match(tostring(verdict), "^([^@#:]*)@([^@#:]*):([01]):(.*)$");
	-- Extra parentheses: GetValue of a missing key returns no value at all, and tostring() fails
	-- when given none (a player without BGE in a new game).
	local _, jetonAnnonce = string.match(tostring((cfg:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#");
	local repris = false;
	if version == nil or jeton ~= jetonAnnonce then
		-- No verdict of this game in our configuration: we did not launch it. Host now, with an
		-- announcement of this game: we take the verdict over.
		local vAnnonce = string.match(tostring((cfg:GetValue("BGD_VERSION"))), "^([^@#:]*)@[^@#:]*#[^@#:,]*$");
		if vAnnonce == nil or jetonAnnonce == nil or jetonAnnonce == "" then
			return;  -- pas d'annonce de notre salon : partie lancée sans notre proxy
		end
		version, jeton, liste = vAnnonce, jetonAnnonce, "";
		actif = (Civ6Proxy_PolActifs() == 1) and "1" or "0";
		verdict = version .. "@" .. jeton .. ":" .. actif .. ":";
		repris = true;
	end
	if (actif == "1") ~= (Civ6Proxy_PolActifs() == 1) then
		BGD_PartieJournal("verdict " .. verdict .. " différent de l'état local, verdict laissé tel quel");
		return;
	end
	local jetons = {};
	for j in string.gmatch(liste, "[^,]+") do jetons[j] = true; end
	local ajoutes, sans = {}, {};
	local pings = GameConfiguration.GetValue("BGD_TEAM_PINGS");
	for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
		local pc = PlayerConfigurations[id];
		if id ~= localID and pc ~= nil and (pc:IsHuman() or ((pings == true or pings == 1) and pc:GetSlotStatus() == SlotStatus.SS_OBSERVER)) and Network.IsPlayerConnected(id) then
			local v, jh, perso = string.match(tostring((pc:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
			if v == version and jh == jeton then
				if not jetons[perso] then
					jetons[perso] = true;
					table.insert(ajoutes, perso);
				end
				BGD_PARTIE.avertis[id] = nil;
			elseif actif == "1" and pc:GetReady() and not BGD_PARTIE.avertis[id] then
				-- Prêt sans annonce : un joueur équipé s'annonce en ouvrant le salon, avant d'être prêt.
				BGD_PARTIE.avertis[id] = true;
				table.insert(sans, Locale.Lookup(pc:GetPlayerName()));
			end
		end
	end
	if #ajoutes > 0 or repris then
		local nouveau = version .. "@" .. jeton .. ":" .. actif .. ":" .. liste
			.. ((liste ~= "") and "," or "") .. table.concat(ajoutes, ",");
		cfg:SetValue("BGD_VERDICT", nouveau);
		Network.BroadcastPlayerInfo();
		BGD_PartieJournal((repris and "verdict taken over as the new host: " or "verdict relayé : ") .. nouveau);
	end
	if #sans > 0 then
		-- En anglais : chaque machine équipée l'affiche dans sa langue (ui_depart_chat.lua).
		local texte = "[COLOR_RED]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_ARRIVEE", table.concat(sans, ", ")) .. "[ENDCOLOR]";
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message : " .. texte);
	end
end

if Civ6Proxy_PolActifs ~= nil then
	local function protege()
		local ok, err = pcall(BGD_PartieRelais);
		if not ok then
			BGD_PartieJournal("erreur : " .. tostring(err));
		end
	end
	Events.PlayerInfoChanged.Add(protege);
	Events.MultiplayerPlayerConnected.Add(protege);
	-- A player who joined while the host was loading announced itself before this block existed.
	Events.LoadScreenClose.Add(protege);
	Events.MultiplayerHostMigrated.Add(protege);
end

-- Problems BGE prevented on this machine, said to everyone in English (each machine with BGE shows
-- it in its language, ui_depart_chat.lua): a damaged network message that would have frozen the
-- game, a wipe of the game settings (messages.rs), a second copy of the game after the load
-- (snapshots.rs). The host also names the players still loading when its load screen closes: they
-- would have received a second copy, which its launch guard stopped. Something prevented before
-- the game (lobby, load) is said at the first occasion. At most one line of each kind per minute;
-- the rest is in the log. Without chat (solo, hotseat, PlayByCloud), the log only.
--   Civ6Proxy_GardeMessages(), Civ6Proxy_GardeWipes(), Civ6Proxy_DoubleLoadRefused() -> how many
--   since the last call; Civ6Proxy_LaunchGuarded() -> "1,3" (host, once)
do
	local last = {};
	local function Name(id)
		local pc = PlayerConfigurations[id];
		return pc and Locale.Lookup(pc:GetPlayerName()) or ("#" .. tostring(id));
	end
	local function Prevented(kind, key, who)
		BGD_PartieJournal(kind .. " prevented for " .. who);
		local now = os.clock();
		if last[kind] ~= nil and now - last[kind] < 60 then
			return;
		end
		last[kind] = now;
		if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud() then
			return;
		end
		local texte = "[COLOR_GREEN]BGE - " .. BGD_TxtAnglais(key, who) .. "[ENDCOLOR]";
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message : " .. texte);
	end
	local KINDS = {
		{ "freeze", Civ6Proxy_GardeMessages, "LOC_BGD_CHAT_GARDE" },
		{ "settings wipe", Civ6Proxy_GardeWipes, "LOC_BGD_CHAT_GARDE_CONFIG" },
		{ "double load", Civ6Proxy_DoubleLoadRefused, "LOC_BGD_CHAT_DOUBLE_LOAD" },
	};
	local function Local()
		for _, k in ipairs(KINDS) do
			local n = k[2] ~= nil and k[2]() or 0;
			if n ~= nil and n > 0 then
				Prevented(k[1], k[3], Name(Network.GetLocalPlayerID()));
			end
		end
	end
	local function LaunchGuarded()
		if Civ6Proxy_LaunchGuarded == nil or not Network.IsGameHost() then
			return;
		end
		local names = {};
		for id in string.gmatch(tostring(Civ6Proxy_LaunchGuarded() or ""), "%d+") do
			id = tonumber(id);
			if Network.IsPlayerHotJoining(id) or Network.IsPlayerResyncing(id) then
				table.insert(names, Name(id));
			end
		end
		if #names > 0 then
			Prevented("double load", "LOC_BGD_CHAT_DOUBLE_LOAD", table.concat(names, ", "));
		end
	end
	local function Safely(f)
		return function()
			local ok, err = pcall(f);
			if not ok then
				BGD_PartieJournal("erreur : " .. tostring(err));
			end
		end
	end
	Events.GameCoreEventPublishComplete.Add(Safely(Local));
	Events.LoadScreenClose.Add(Safely(LaunchGuarded));
end

-- Links relayed through the host (startup_proxy/src/relink.rs): two players whose direct link
-- failed when one of them arrived go through the host for the rest of the game, and both are
-- disconnected if the host leaves. During the simultaneous turn start (ui_depart.lua: everyone
-- waits, orders are held, nothing is in flight between them), the lower player ID of each pair
-- rebuilds the direct link: it says so in the chat, starts the attempt, holds its "ready" signal
-- until the result (BGD_RelinkHolding, which ui_depart.lua checks before sending `.bgd_pret`; the
-- host waits at most 60 s, an attempt lasts at most 45 s), then says the result. On success,
-- `.bgd_relinked <a> <b>` goes to everyone (hidden by ui_depart_chat.lua), and every machine with
-- BGE removes the pair from its relay table. At most MAX_TRIES attempts per pair per game. The
-- simultaneous turn start requires every human to have the same BGE version, so both players of
-- the pair have it.
if Civ6Proxy_RelayedPairs ~= nil and Civ6Proxy_RelinkStart ~= nil and Civ6Proxy_RelinkState ~= nil
	and Civ6Proxy_RelayForgetPair ~= nil then
	local SIGNAL = ".bgd_relinked ";
	local MAX_TRIES = 3;
	local R = { tries = {}, turn = -1, queue = {}, current = nil };

	local function Name(id)
		local pc = PlayerConfigurations[id];
		return pc and Locale.Lookup(pc:GetPlayerName()) or ("#" .. tostring(id));
	end

	-- "<player>: BGE - …" to everyone, in English (each machine with BGE translates it on arrival).
	local function Say(color, key, p, ...)
		local text = color .. "BGE - " .. BGD_TxtAnglais(key, Name(p.a), Name(p.b), ...) .. "[ENDCOLOR]";
		Network.SendChat(text, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message: " .. text);
	end

	local function Key(p)
		return p.a .. "-" .. p.b;
	end

	-- The pairs of the local table where this player has the lower ID, with the partner connected.
	local function MyPairs()
		local me = Network.GetLocalPlayerID();
		local t = {};
		for a, b in string.gmatch(tostring(Civ6Proxy_RelayedPairs() or ""), "(%d+)-(%d+)") do
			a, b = tonumber(a), tonumber(b);
			if a == me and Network.IsPlayerConnected(b) then
				table.insert(t, { a = a, b = b });
			end
		end
		return t;
	end

	local function Start(p)
		R.tries[Key(p)] = (R.tries[Key(p)] or 0) + 1;
		local pc = PlayerConfigurations[p.b];
		local steam = pc and tostring(pc:GetNetworkIdentifer() or "") or "";
		if not string.match(steam, "^%d+$") or Civ6Proxy_RelinkStart(p.b, steam) ~= 1 then
			BGD_PartieJournal("relink: attempt " .. Key(p) .. " not started (see the relink lines)");
			return false;
		end
		R.current = p;
		BGD_PartieJournal("relink: attempt " .. Key(p) .. " (" .. R.tries[Key(p)] .. "/" .. MAX_TRIES .. ")");
		Say("[COLOR_Civ6Yellow]", "LOC_BGD_CHAT_RELINK_DETECTED", p);
		return true;
	end

	-- True while an attempt is in progress: ui_depart.lua does not send `.bgd_pret` yet.
	-- Called when this machine would be ready, then twice a second while it holds.
	function BGD_RelinkHolding()
		local ok, holding = pcall(function()
			if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud() then
				return false;
			end
			if R.current ~= nil then
				local p = R.current;
				local state = Civ6Proxy_RelinkState(p.b);
				if state == 0 then
					return true;
				end
				R.current = nil;
				if state == 1 then
					Network.SendChat(SIGNAL .. p.a .. " " .. p.b, ChatTargetTypes.CHATTARGET_ALL, -1);
					Say("[COLOR_GREEN]", "LOC_BGD_CHAT_RELINK_FIXED", p);
				elseif (R.tries[Key(p)] or 0) >= MAX_TRIES then
					Say("[COLOR_RED]", "LOC_BGD_CHAT_RELINK_GAVE_UP", p, MAX_TRIES);
				else
					Say("[COLOR_RED]", "LOC_BGD_CHAT_RELINK_FAILED", p);
				end
			end
			local turn = Game.GetCurrentGameTurn();
			if R.turn ~= turn then
				R.turn = turn;
				R.queue = {};
				for _, p in ipairs(MyPairs()) do
					if (R.tries[Key(p)] or 0) < MAX_TRIES then
						table.insert(R.queue, p);
					end
				end
			end
			while #R.queue > 0 do
				if Start(table.remove(R.queue, 1)) then
					return true;
				end
			end
			return false;
		end);
		if not ok then
			BGD_PartieJournal("relink: error: " .. tostring(holding));
			R.current = nil;
			return false;
		end
		return holding;
	end

	-- Success announced by the initiator: the pair leaves the local table (the host's included).
	Events.MultiplayerChat.Add(function(fromPlayer, toPlayer, text, eTargetType)
		local ok, err = pcall(function()
			if type(text) ~= "string" or string.sub(text, 1, #SIGNAL) ~= SIGNAL then
				return;
			end
			local a, b = string.match(text, "^%.bgd_relinked (%d+) (%d+)");
			if a ~= nil and Civ6Proxy_RelayForgetPair(tonumber(a), tonumber(b)) == 1 then
				BGD_PartieJournal("relink: pair " .. a .. "-" .. b .. " removed (linked directly)");
			end
		end);
		if not ok then
			BGD_PartieJournal("relink: error: " .. tostring(err));
		end
	end);
end

-- Desyncs the host takes over (sync_notes.rs, reload_all.rs). At a turn-end sync check, a client
-- whose game has objects the host's has not (or the other way round) would reload alone, and a
-- game loaded alone drifts away from the running ones. While the host announces that it takes over
-- (its BGD_RELOAD value), clients with BGE do not reload by themselves: they acknowledge the check
-- and wait. At the turn start (ui_depart.lua's hold, BGD_ReloadAllStep), the host says it, tells
-- the clients (`.bgd_reload`) and loads its own game; its fresh interface sends `.bgd_load` and
-- every client loads that game, once. The turn start waits until everyone is back. A client that
-- gets no `.bgd_load` reloads by itself after CLIENT_WAIT seconds.
--   Civ6Proxy_SyncNotes() -> "player|major|checks in a row|missing|extra|categories" separated by
--                            ";", since the last call (host only)
--   Civ6Proxy_ReloadAllReady() -> 1 if this machine, as host, can take a reload over
--   Civ6Proxy_ReloadAllMark(), Civ6Proxy_ReloadAllTake() -> seconds since the mark or -1
--   Civ6Proxy_ReloadAllNoSelfReload(on) -> 1 if set (client)
if Civ6Proxy_SyncNotes ~= nil then
	local ALL_WAIT = 30;       -- seconds for the host's own load to start
	local ALL_MAX_AGE = 600;   -- seconds after which the clients' step is no longer done
	local ALL_HOLD = 120;      -- seconds the turn start waits for the clients' loads (depart.rs safety)
	local CLIENT_WAIT = 180;   -- client: seconds to wait for `.bgd_load` before reloading by itself
	local RELOAD, LOAD = ".bgd_reload ", ".bgd_load ";
	local KEY = "BGD_RELOAD";
	local S = { pending = nil, asked = nil, allClients = nil, waiting = nil, noSelf = nil };

	local function IsHost()
		return Network.GetLocalPlayerID() == Network.GetGameHostPlayerID();
	end

	local function Name(id)
		local pc = PlayerConfigurations[id];
		return pc and Locale.Lookup(pc:GetPlayerName()) or ("#" .. tostring(id));
	end

	local function NetworkGame()
		return GameConfiguration.IsNetworkMultiplayer() and not GameConfiguration.IsPlayByCloud();
	end

	local function Safely(f)
		return function(...)
			local ok, err = pcall(f, ...);
			if not ok then
				BGD_PartieJournal("sync: error: " .. tostring(err));
			end
		end
	end

	-- Host: announces whether it takes reloads over. Client: follows the host's announcement.
	local function Announce()
		if not NetworkGame() then
			return;
		end
		if IsHost() then
			local cfg = PlayerConfigurations[Network.GetLocalPlayerID()];
			local ready = (Civ6Proxy_ReloadAllReady ~= nil and Civ6Proxy_ReloadAllReady() == 1) and 1 or 0;
			if cfg ~= nil and cfg:GetValue(KEY) ~= ready then
				cfg:SetValue(KEY, ready);
				Network.BroadcastPlayerInfo();
				BGD_PartieJournal("sync: the host " .. (ready == 1 and "takes reloads over" or "leaves reloads to the game"));
			end
			return;
		end
		if Civ6Proxy_ReloadAllNoSelfReload == nil then
			return;
		end
		local host = PlayerConfigurations[Network.GetGameHostPlayerID()];
		local v = host and host:GetValue(KEY);
		local on = (v == 1 or v == true) and 1 or 0;
		local set = Civ6Proxy_ReloadAllNoSelfReload(on) == 1;
		if set and S.noSelf ~= on then
			S.noSelf = on;
			BGD_PartieJournal("sync: " .. (on == 1 and "a failed check waits for the host" or "a failed check reloads this game, as the game does"));
		end
	end

	-- Host: the hook's notes. Objects on one side only: the host takes over at the turn start.
	local function Drain()
		local notes = Civ6Proxy_SyncNotes();
		if notes == nil or notes == "" or not IsHost() then
			return;
		end
		for note in string.gmatch(notes, "[^;]+") do
			local p, n, streak, missing, extra, list = string.match(note, "^(%-?%d+)|(%d+)|(%d+)|(%d+)|(%d+)|(.*)$");
			p, missing, extra = tonumber(p), tonumber(missing), tonumber(extra);
			if p ~= nil and missing ~= nil and extra ~= nil then
				BGD_PartieJournal("sync: player " .. p .. ": " .. n .. " major archive(s) repaired (" .. list .. "), " .. streak
					.. " in a row; " .. missing .. " object(s) missing, " .. extra .. " extra");
				local cfg = PlayerConfigurations[Network.GetLocalPlayerID()];
				local takesOver = cfg ~= nil and cfg:GetValue(KEY) == 1;
				if missing + extra > 0 and takesOver then
					S.pending = S.pending or { players = {} };
					S.pending.players[p] = { missing = missing, extra = extra };
				end
			end
		end
	end

	local function Send(text)
		Network.SendChat(text, ChatTargetTypes.CHATTARGET_ALL, -1);
	end

	local function Loading(me)
		return (UI ~= nil and UI.IsResyncLoadInProgress ~= nil and UI.IsResyncLoadInProgress())
			or Network.IsPlayerResyncing(me) or Network.IsPlayerHotJoining(me);
	end

	-- Host: says why, tells the clients and loads its own game.
	local function Start()
		local p = S.pending;
		S.pending = nil;
		for id, c in pairs(p.players) do
			local text = "[COLOR_Civ6Yellow]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_SYNC_RELOAD_ALL", Name(id), c.missing, c.extra) .. "[ENDCOLOR]";
			BGD_PartieJournal("message: " .. text);
			Send(text);
		end
		Civ6Proxy_ReloadAllMark();
		S.asked = os.clock();
		Send(RELOAD .. tostring(Game.GetCurrentGameTurn()));
		BGD_PartieJournal("sync: the host loads its own game, then every client loads it");
		Network.RequestSnapshot();
	end

	-- Host, fresh interface after its own load: every client loads the same game.
	local function AfterLoad()
		if not IsHost() or S.asked ~= nil or Civ6Proxy_ReloadAllTake == nil or Loading(Network.GetLocalPlayerID()) then
			return;
		end
		local age = Civ6Proxy_ReloadAllTake();
		if age == nil or age < 0 then
			return;
		end
		if age > ALL_MAX_AGE then
			BGD_PartieJournal("sync: reload of everyone older than " .. ALL_MAX_AGE .. " s ignored");
			return;
		end
		BGD_PartieJournal("sync: the host has loaded its own game; every client loads it now");
		S.allClients = os.clock();
		Send(LOAD .. tostring(Game.GetCurrentGameTurn()));
	end

	-- For ui_depart.lua, at every tick of the turn start: does it have to wait? Host: starts a
	-- pending takeover; waits while its own load is asked for and while the clients load. Client:
	-- waits for `.bgd_load` after `.bgd_reload`, and reloads by itself if it never comes.
	-- Events may be held at the turn start: the notes and the after-load step are read here too.
	function BGD_ReloadAllStep()
		if IsHost() then
			Drain();
			AfterLoad();
			if S.pending ~= nil and S.asked == nil then
				Start();
			end
			-- The old interface still runs ALL_WAIT seconds after asking: the load never started.
			if S.asked ~= nil and os.clock() - S.asked > ALL_WAIT then
				-- The host's own load never started: the clients load its running game.
				S.asked = nil;
				Civ6Proxy_ReloadAllTake();
				S.allClients = os.clock();
				BGD_PartieJournal("sync: the host's own load did not start within " .. ALL_WAIT .. " s; the clients load its game");
				Send(LOAD .. tostring(Game.GetCurrentGameTurn()));
			end
			return S.asked ~= nil or (S.allClients ~= nil and os.clock() - S.allClients < ALL_HOLD);
		end
		if S.waiting ~= nil and os.clock() - S.waiting > CLIENT_WAIT then
			S.waiting = nil;
			BGD_PartieJournal("sync: no game from the host after " .. CLIENT_WAIT .. " s; this game reloads by itself");
			Network.RequestSnapshot();
		end
		return S.waiting ~= nil;
	end

	Events.MultiplayerChat.Add(Safely(function(fromPlayer, toPlayer, text)
		if type(text) ~= "string" or IsHost() or fromPlayer ~= Network.GetGameHostPlayerID() then
			return;
		end
		if string.sub(text, 1, #RELOAD) == RELOAD then
			S.waiting = os.clock();
			BGD_PartieJournal("sync: the host loads its own game; this game waits for it");
		elseif string.sub(text, 1, #LOAD) == LOAD then
			S.waiting = nil;
			BGD_PartieJournal("sync: the host has loaded its game; this game loads it");
			Network.RequestSnapshot();
		end
	end));

	Events.PlayerInfoChanged.Add(Safely(Announce));
	Events.MultiplayerHostMigrated.Add(Safely(Announce));
	Events.LoadScreenClose.Add(Safely(Announce));
	Events.LoadScreenClose.Add(Safely(AfterLoad));
	-- A host migration clears the client switch: set it again before every check.
	Events.TurnEnd.Add(Safely(Announce));
	Events.GameCoreEventPublishComplete.Add(Safely(Drain));
	Events.GameCoreEventPublishComplete.Add(Safely(AfterLoad));
	-- Without the simultaneous turn start (no hold), the turn start itself starts a pending takeover.
	Events.TurnBegin.Add(Safely(function()
		if IsHost() and S.pending ~= nil and S.asked == nil then
			Start();
		end
	end));
end
