-- Test hors jeu de src/salon.lua :
--   lua startup_proxy/tests/test_salon.lua
-- Plusieurs machines dans un même salon, chacune avec sa copie des configurations de joueur, comme
-- dans le jeu : SetValue n'écrit que sur sa machine, BroadcastPlayerInfo recopie la configuration du
-- joueur local chez les autres et y lève PlayerInfoChanged. Une machine « sans proxy » ne charge
-- pas le bloc. On vérifie les annonces, le verdict de l'hôte et le message du chat au lancement.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/salon.lua");
-- Relais de l'hôte en cours de partie (InGame.lua) et écran de chargement (LoadScreen.lua).
local PARTIE = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/partie.lua");
local CHARGEMENT = ReadFile(dir .. "/../src/chargement.lua");

local failures = 0;
local function check(cond, what)
	if not cond then
		failures = failures + 1;
		print("ÉCHEC : " .. what);
	end
end

local function NewEvent()
	local e = { handlers = {} };
	e.Add = function(h) table.insert(e.handlers, h); end
	e.Fire = function(...) for _, h in ipairs(e.handlers) do h(...); end end
	return e;
end

-- `joueurs` : { { nom, proxy = true|false, version = "x" } ... }, l'hôte en premier (id 0).
-- `options.lang`, `options.hotseat`, `options.valeurs` (valeurs déjà présentes, par joueur, comme
-- après le chargement d'une sauvegarde).
local function NewLobby(joueurs, options)
	options = options or {};
	-- `chat` : les messages visibles, hors annonces d'arrivée (`annonces`) et messages cachés
	-- « .bgd_ » (`caches`) ; `tout` : tous, dans l'ordre.
	local lobby = { machines = {}, chat = {}, annonces = {}, caches = {}, tout = {}, host = 0, tokens = 0 };

	for id = 0, #joueurs - 1 do
		local m = { id = id, values = {}, events = {}, luaEvents = {} };
		for pid = 0, #joueurs - 1 do
			m.values[pid] = {};
			for k, v in pairs((options.valeurs or {})[pid] or {}) do m.values[pid][k] = v; end
		end
		lobby.machines[id] = m;
	end

	local function Broadcast(from)
		for _, m in pairs(lobby.machines) do
			if m.id ~= from then
				m.values[from] = {};
				for k, v in pairs(lobby.machines[from].values[from]) do m.values[from][k] = v; end
			end
		end
		for _, m in pairs(lobby.machines) do
			m.events.PlayerInfoChanged.Fire(from);
		end
	end

	for id, m in pairs(lobby.machines) do
		local j = joueurs[id + 1];
		local env = setmetatable({}, { __index = _G });
		m.env = env;
		m.events = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
		m.luaEvents = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
		env.Events = m.events;
		env.LuaEvents = m.luaEvents;
		-- Le salon est caché une fois la partie lancée (`m.cache`).
		env.ContextPtr = { IsHidden = function() return m.cache == true; end };
		env.Locale = {
			GetCurrentLanguage = function() return { Type = options.lang or "fr_FR" }; end,
			Lookup = function(s) return s; end,
		};
		env.SlotStatus = { SS_TAKEN = 1, SS_OPEN = 2, SS_OBSERVER = 3 };
		env.ChatTargetTypes = { CHATTARGET_ALL = -1, CHATTARGET_PLAYER = -3 };
		-- `options.controles` : le XML du salon a le bouton et le minuteur (Controls), comme en jeu.
		if options.controles then
			local function Ctrl()
				local c = { cache = false, inactif = false, callbacks = {} };
				function c:SetHide(v) self.cache = v; end
				function c:IsHidden() return self.cache; end
				function c:SetDisabled(v) self.inactif = v; end
				function c:SetText(t) self.texte = t; end
				function c:SetToolTipString(t) self.aide = t; end
				function c:RegisterCallback(ev, f) self.callbacks[ev] = f; end
				function c:RegisterEndCallback(f) self.fin = f; end
				function c:SetToBeginning() end
				function c:Play() self.joue = true; end
				return c;
			end
			env.Controls = { BGD_CheckButton = Ctrl(), BGD_SalonMinuteur = Ctrl() };
			if options.reset then env.Controls.ResetButton = Ctrl(); env.Controls.ResetButton.cache = options.reset == "cache"; end
			env.Mouse = { eLClick = 1, eMouseEnter = 2 };
			env.UI = { PlaySound = function() end };
		end
		env.GameConfiguration = {
			GetValue = function(key) if key == "BGD_TEAM_PINGS" then return options.pings; end end,
			IsNetworkMultiplayer = function() return not options.hotseat; end,
			IsHotseat = function() return options.hotseat == true; end,
			IsPlayByCloud = function() return false; end,
			GetMultiplayerPlayerIDs = function()
				local ids = {};
				for pid = 0, #joueurs - 1 do table.insert(ids, pid); end
				return ids;
			end,
		};
		env.PlayerConfigurations = setmetatable({}, { __index = function(_, pid)
			if m.values[pid] == nil then return nil; end
			return {
				GetValue = function(_, k) return m.values[pid][k]; end,
				SetValue = function(_, k, v)
					assert(pid == id, "une machine n'écrit que sa propre configuration");
					m.values[pid][k] = v;
				end,
				IsHuman = function() return joueurs[pid + 1].ia ~= true; end,
				GetSlotStatus = function() return joueurs[pid + 1].observer and 3 or 1; end,
				GetPlayerName = function() return joueurs[pid + 1].nom; end,
				GetReady = function() return joueurs[pid + 1].pret == true; end,
			};
		end });
		env.Network = {
			IsGameHost = function() return id == lobby.host; end,
			GetGameHostPlayerID = function() return lobby.host; end,
			GetLocalPlayerID = function() return id; end,
			IsPlayerConnected = function(pid) return joueurs[pid + 1].absent ~= true; end,
			BroadcastPlayerInfo = function() Broadcast(id); end,
			SendChat = function(text, target, to)
				local c = { from = id, to = to, text = text, target = target };
				table.insert(lobby.tout, c);
				if string.sub(text, 1, 5) == ".bgd_" then
					table.insert(lobby.caches, c);
				elseif text:find("has BetterGameExperience .- enabled%.%[ENDCOLOR%]$") then
					table.insert(lobby.annonces, c);
				else
					table.insert(lobby.chat, c);
				end
				-- Remis à tous (CHATTARGET_ALL) ou au seul destinataire (privé), l'expéditeur compris.
				if lobby.coupe then return; end
				for _, d in pairs(lobby.machines) do
					if target ~= -3 or d.id == to or d.id == id then
						if d.env.Events then d.events.MultiplayerChat.Fire(id, to, text, target); end
					end
				end
			end,
		};
		env.StartLaunchCountdown = function() m.countdowns = (m.countdowns or 0) + 1; end
		if j.proxy ~= false then
			-- `indispo` : le proxy de démarrage est là, mais cette machine n'appliquerait pas les
			-- correctifs POL (proxy de GameCore absent ou désactivé).
			env.Civ6Proxy_Version = function()
				if j.indispo then return nil, j.indispo; end
				return j.version or "0.1.0";
			end
			env.Civ6Proxy_FixerPol = function(actif) m.pol = (actif == 1); end
			env.Civ6Proxy_Jeton = function() lobby.tokens = lobby.tokens + 1; return "j" .. lobby.tokens; end
			env.Civ6Proxy_Journal = function(t) m.log = (m.log or "") .. t .. "\n"; end
			env.Civ6Proxy_VersionInstallee = function() return j.version or "0.1.0"; end
			-- `options.avant` : globales et contrôles du StagingRoom.lua de MPH, posés avant le bloc.
			if options.avant then options.avant(env); end
			local chunk = assert(load(BLOCK, "=salon.lua", "t", env));
			chunk();
		end
	end

	-- Ouverture du salon : l'hôte d'abord, puis ceux qui le rejoignent.
	function lobby.open()
		for id = 0, #joueurs - 1 do
			local m = lobby.machines[id];
			local ev = (id == lobby.host) and m.luaEvents.HostGame_ShowStagingRoom or m.luaEvents.JoiningRoom_ShowStagingRoom;
			ev.Fire();
		end
	end
	-- `n` tics du minuteur sur chaque machine où il tourne.
	function lobby.tic(n)
		for _ = 1, n or 1 do
			for _, m in pairs(lobby.machines) do
				local t = m.env.Controls and m.env.Controls.BGD_SalonMinuteur;
				if t and t.joue and t.fin then t.joue = false; t.fin(); end
			end
		end
	end
	function lobby.bouton(id)
		return lobby.machines[id].env.Controls.BGD_CheckButton;
	end
	function lobby.countdown()
		for _, m in pairs(lobby.machines) do m.env.StartLaunchCountdown(); end
	end
	function lobby.verdict()
		return lobby.machines[lobby.host].values[lobby.host].BGD_VERDICT;
	end
	-- Le drapeau du verdict, "1" ou "0" (nil sans verdict).
	function lobby.actif()
		local v = lobby.verdict();
		return v and v:match("^[^:]*:([01]):");
	end
	return lobby;
end

-- Conclusion de chaque machine équipée au lancement : l'état passé au proxy (Civ6Proxy_FixerPol),
-- qui doit concorder avec la ligne du journal.
local function Pol(lobby, id)
	local m = lobby.machines[id];
	local last;
	for v in (m.log or ""):gmatch("lancement : correctifs POL (%S+)") do last = v; end
	local etat = (m.pol == true) and "actifs" or "désactivés";
	if last ~= nil and last ~= etat then
		return "journal " .. last .. " / état " .. etat;
	end
	return etat;
end

local function LastChat(lobby)
	local c = lobby.chat[#lobby.chat];
	return c and c.text or "";
end

-- 1. Tous équipés : verdict 1, message vert, une seule fois.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } });
	l.open();
	check(l.actif() == "1", "tous équipés : verdict 1 (" .. tostring(l.verdict()) .. ")");
	l.countdown();
	check(#l.chat == 1 and l.chat[1].from == 0, "un seul message, de l'hôte");
	check(LastChat(l):find("^%[COLOR_GREEN%]BGE %- ") and LastChat(l):find("POL fixes enabled", 1, true), "message vert : " .. LastChat(l));
	l.countdown();
	check(#l.chat == 1, "pas de second message identique");
	check(l.machines[1].countdowns == 2, "le compte à rebours d'origine est toujours lancé");
	for id = 0, 2 do check(Pol(l, id) == "actifs", "machine " .. id .. " : POL actifs (" .. tostring(Pol(l, id)) .. ")"); end
end

-- 2. Un joueur sans proxy : verdict 0, message rouge qui le nomme.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", proxy = false }, { nom = "Zoé" } });
	l.open();
	check(l.actif() == "0", "sans proxy : verdict 0");
	l.countdown();
	local t = LastChat(l);
	check(t:find("^%[COLOR_RED%]BGE %- Without BetterGameExperience: Bob%."), "message rouge : " .. t);
	check(t:find("POL fixes disabled for this game.", 1, true), "correctifs POL désactivés : " .. t);
	check(not t:find("Zoé", 1, true), "Zoé est équipée");
	check(#l.chat == 1, "Zoé, vue par l'hôte, n'écrit rien");
	check(Pol(l, 0) == "désactivés" and Pol(l, 2) == "désactivés", "POL désactivés chez l'hôte et Zoé");
end

-- 3. Valeur restée d'une sauvegarde chez un joueur qui n'a plus le proxy : ancien jeton, ne compte pas.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", proxy = false } },
		{ valeurs = { [1] = { BGD_VERSION = "0.1.0@ancien#vieux" } } });
	l.open();
	check(l.actif() == "0", "valeur d'une sauvegarde : verdict 0");
	l.countdown();
	check(LastChat(l):find("Without BetterGameExperience: Bob", 1, true), "Bob compté sans proxy : " .. LastChat(l));
end

-- 4. Autre version, en anglais.
do
	local l = NewLobby({ { nom = "Host" }, { nom = "Bob", version = "0.0.9" } }, { lang = "en_US" });
	l.open();
	check(l.actif() == "0", "autre version : verdict 0");
	l.countdown();
	local t = LastChat(l);
	check(t:find("BetterGameExperience version other than 0.1.0: Bob (0.0.9).", 1, true), "version citée : " .. t);
	check(t:find("POL fixes disabled for this game.", 1, true), "anglais : " .. t);
end

-- 5. IA et joueurs déconnectés ne comptent pas ; un client n'envoie rien au lancement.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "IA", proxy = false, ia = true }, { nom = "Parti", proxy = false, absent = true } });
	l.open();
	check(l.actif() == "1", "IA et absent ignorés : verdict 1");
	l.countdown();
	check(#l.chat == 1, "seul l'hôte écrit");
end

-- 6. Hôte sans proxy : aucun verdict ; le premier client équipé le dit. Bob (id 1) n'a pas le
-- proxy : c'est Zoé (id 2) qui écrit, une fois.
do
	local l = NewLobby({ { nom = "Hôte", proxy = false }, { nom = "Bob", proxy = false }, { nom = "Zoé" }, { nom = "Léo" } });
	l.open();
	check(l.verdict() == nil, "hôte sans proxy : pas de verdict");
	l.countdown();
	local deZoe = 0;
	for _, c in ipairs(l.chat) do
		check(c.from ~= 0 and c.text:find("^%[COLOR_RED%]BGE %- The host does not have BetterGameExperience"), "message d'un client : " .. c.text);
		if c.from == 2 then deZoe = deZoe + 1; end
	end
	check(deZoe == 1, "Zoé écrit une fois");
	check(Pol(l, 2) == "désactivés" and Pol(l, 3) == "désactivés", "POL désactivés chez les clients");
end

-- 10. Verdict périmé sorti d'une sauvegarde : l'hôte avait le proxy, il ne l'a plus. Il rediffuse
-- sa configuration, verdict « 1 » compris : les clients ne doivent pas y croire.
do
	local l = NewLobby({ { nom = "Hôte", proxy = false }, { nom = "Bob" }, { nom = "Zoé" } },
		{ valeurs = {
			[0] = { BGD_VERSION = "0.1.0@j1#j1", BGD_VERDICT = "0.1.0@j1:1:b1,z1" },
			[1] = { BGD_VERSION = "0.1.0@j1#b1" },
			[2] = { BGD_VERSION = "0.1.0@j1#z1" },
		} });
	l.open();
	for _, m in pairs(l.machines) do m.events.PlayerInfoChanged.Fire(0); end
	l.countdown();
	check(Pol(l, 1) == "désactivés" and Pol(l, 2) == "désactivés", "verdict périmé ignoré : " .. tostring(Pol(l, 1)) .. ", " .. tostring(Pol(l, 2)));
	check(#l.chat >= 1 and l.chat[1].text:find("The host does not have BetterGameExperience", 1, true), "un client prévient");
end

-- 7. Hotseat : rien.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { hotseat = true });
	l.open();
	l.countdown();
	check(l.verdict() == nil and #l.chat == 0, "hotseat : rien");
end

-- 8. Migration d'hôte : le nouvel hôte tire son jeton, les autres se réannoncent avec.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } });
	l.open();
	l.host = 1;
	for _, m in pairs(l.machines) do m.events.MultiplayerHostMigrated.Fire(1); end
	local jeton = l.machines[1].values[1].BGD_VERSION:match("^.-@(.-)#");
	check(l.machines[2].values[2].BGD_VERSION:match("^.-@(.-)#") == jeton, "Zoé reprend le jeton du nouvel hôte");
	check(l.actif() == "1", "verdict du nouvel hôte : 1");
end

-- 9. Un joueur qui arrive plus tard : le verdict passe à 0 dès son arrivée.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", proxy = false, absent = true } });
	l.open();
	check(l.actif() == "1", "avant l'arrivée : 1");
	l.machines[0].env.Network.IsPlayerConnected = function() return true; end
	l.machines[0].events.MultiplayerPlayerConnected.Fire(1);
	check(l.actif() == "0", "après l'arrivée de Bob sans proxy : 0");
end

-- 11. Proxy de démarrage présent, mais correctifs POL indisponibles sur cette machine : Bob ne
-- s'annonce pas, compte comme sans BetterGameExperience, et n'active rien chez lui.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", indispo = "proxys désactivés (actif=off)" } });
	l.machines[1].pol = true;  -- reste d'une partie solo
	l.open();
	check(l.actif() == "0", "Bob indisponible : verdict 0");
	check(l.machines[1].values[1].BGD_VERSION == nil, "Bob ne s'annonce pas");
	l.countdown();
	check(LastChat(l):find("Without BetterGameExperience: Bob", 1, true), "Bob listé : " .. LastChat(l));
	check(Pol(l, 0) == "désactivés" and Pol(l, 1) == "désactivés", "POL coupés chez tous, Bob compris");
	check(l.machines[1].log:find("pas d'annonce : proxys désactivés", 1, true), "raison au journal de Bob");
end

-- 12. Un salon réutilise l'état : une partie avec tous équipés, puis un salon où il manque un joueur.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } });
	l.open();
	l.countdown();
	check(Pol(l, 0) == "actifs" and Pol(l, 1) == "actifs", "premier salon : actifs");
	l.open();
	check(Pol(l, 0) == "actifs", "réouverture, tous encore là : actifs");
end

-- La partie démarre : les salons se cachent, l'hôte charge le relais de InGame.lua.
local function Lancer(l)
	for _, m in pairs(l.machines) do
		m.cache = true;
		m.events.LoadScreenContentReady.Fire();
	end
	local hote = l.machines[l.host];
	hote.env.Civ6Proxy_PolActifs = function() return hote.pol and 1 or 0; end
	assert(load(PARTIE, "=partie.lua", "t", hote.env))();
end

-- 13. Reconnexion en cours de partie d'un joueur équipé : nouveau jeton, que l'hôte relaie ; il
-- conclut comme les autres.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } });
	l.open();
	l.countdown();
	Lancer(l);
	local bob = l.machines[1];
	bob.cache = false;
	bob.luaEvents.JoiningRoom_ShowStagingRoom.Fire();
	check(Pol(l, 1) == "actifs", "Bob reconnecté : actifs (" .. Pol(l, 1) .. ")");
	check(l.machines[0].log:find("partie : verdict relayé", 1, true), "relais journalisé chez l'hôte");
	check(#l.chat == 1, "pas de message pour une reconnexion équipée");
end

-- 13b. The host's final verdict reaches a client after its lobby was hidden for the launch (too late
-- for PlayerInfoChanged): at load, the launch data decides, the same as everyone else.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } });
	l.open();
	local z = l.machines[2];
	local final = z.values[0].BGD_VERDICT;
	local perso = z.values[2].BGD_VERSION:match("#(.*)$");
	local i, j = final:find(perso, 1, true);
	z.values[0].BGD_VERDICT = final:sub(1, i - 1) .. "autre" .. final:sub(j + 1);
	z.events.PlayerInfoChanged.Fire(0);
	l.countdown();
	check(Pol(l, 2) == "désactivés", "stale verdict at the countdown: Zoé off (" .. Pol(l, 2) .. ")");
	z.cache = true;
	z.values[0].BGD_VERDICT = final;
	local said = #l.tout;
	z.events.PlayerInfoChanged.Fire(0);
	check(z.pol == true, "verdict that arrives while the lobby is hidden for the launch: on");
	check(#l.tout == said, "hidden lobby: nothing said");
	-- A verdict off, or without our token, never turns a hidden lobby off or on.
	z.values[0].BGD_VERDICT = final:gsub(":1:", ":0:");
	z.events.PlayerInfoChanged.Fire(0);
	check(z.pol == true, "hidden lobby: never turned off");
	-- The same verdict arriving during the load instead (queued): decided at the load screen.
	z.pol = false;
	z.values[0].BGD_VERDICT = final;
	-- The load screen of each machine, with that machine's copy of the player configurations.
	local function Charger(m, reseau, fermeture)
		local ev = NewEvent();
		local env = setmetatable({
			Events = { LoadScreenContentReady = ev, LoadScreenClose = fermeture or NewEvent() },
			GameConfiguration = {
				IsNetworkMultiplayer = function() return reseau ~= false; end,
				IsHotseat = function() return false; end,
				IsPlayByCloud = function() return false; end,
			},
			Network = { GetLocalPlayerID = function() return m.id; end, GetGameHostPlayerID = function() return l.host; end },
			PlayerConfigurations = setmetatable({}, { __index = function(_, pid)
				local v = m.values[pid];
				return v and { GetValue = function(_, k) return v[k]; end };
			end }),
			Civ6Proxy_FixerPol = function(a) m.pol = (a == 1); end,
			Civ6Proxy_Journal = function(t) m.log = (m.log or "") .. t .. "\n"; end,
		}, { __index = _G });
		assert(load(CHARGEMENT, "=chargement.lua", "t", env))();
		ev.Fire();
	end
	for _, m in pairs(l.machines) do Charger(m); end
	check(z.pol == true and l.machines[0].pol == true and l.machines[1].pol == true, "at load: on, like everyone else");
	check(z.log:find("load: POL fixes on (launch data)", 1, true) ~= nil, "decision at load logged");
	-- A verdict that does not name this machine, or is off, turns nothing on.
	z.pol = false;
	z.values[0].BGD_VERDICT = final:sub(1, i - 1) .. "autre" .. final:sub(j + 1);
	Charger(z);
	check(z.pol == false, "at load, verdict without our token: stays off");
	z.values[0].BGD_VERDICT = final:gsub(":1:", ":0:");
	Charger(z);
	check(z.pol == false, "at load, verdict off: stays off");
	-- The verdict arrives between the end of the load and the closing of the load screen.
	local fermeture = NewEvent();
	z.values[0].BGD_VERDICT = final:sub(1, i - 1) .. "autre" .. final:sub(j + 1);
	Charger(z, true, fermeture);
	check(z.pol == false, "verdict not there yet when the game is loaded: off");
	z.values[0].BGD_VERDICT = final;
	fermeture.Fire();
	check(z.pol == true, "verdict there when the load screen closes: on");
end

-- 13c. With the lobby's timer: a line that may be one message early waits a few ticks, and is not
-- said when the verdict arrives meanwhile; a host without BGE is still reported.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } }, { controles = true });
	l.open();
	l.tic();
	local z = l.machines[2];
	local final = z.values[0].BGD_VERDICT;
	local perso = z.values[2].BGD_VERSION:match("#(.*)$");
	local i, j = final:find(perso, 1, true);
	z.values[0].BGD_VERDICT = final:sub(1, i - 1) .. "autre" .. final:sub(j + 1);
	z.events.PlayerInfoChanged.Fire(0);
	l.countdown();
	check(#l.chat == 1 and l.chat[1].from == 0, "late verdict: only the host's line at the countdown");
	z.values[0].BGD_VERDICT = final;
	z.events.PlayerInfoChanged.Fire(0);
	l.tic(6);
	check(#l.chat == 1, "late verdict arrived: no false line from the client");
	check(Pol(l, 2):find("actifs") ~= nil and z.pol == true, "late verdict arrived: Zoé on");

	local s = NewLobby({ { nom = "Hôte", proxy = false }, { nom = "Bob" } }, { controles = true });
	s.open();
	s.tic();
	s.countdown();
	check(#s.chat == 0, "host without BGE: not said at once");
	s.tic(4);
	check(#s.chat == 1 and s.chat[1].text:find("The host does not have BetterGameExperience", 1, true) ~= nil, "host without BGE: said after the delay");
	s.tic(4);
	check(#s.chat == 1, "host without BGE: said once");

	-- The host misses a player: said after the delay, or at the launch if it comes first.
	local h = NewLobby({ { nom = "Hôte" }, { nom = "Bob", proxy = false } }, { controles = true, avant = function(env)
		env.Network.LaunchGame = function() end
	end });
	h.open();
	h.tic();
	h.countdown();
	check(#h.chat == 0, "host, player without BGE: not said at once");
	h.machines[0].env.Network.LaunchGame();
	check(#h.chat == 1 and h.chat[1].text:find("Without BetterGameExperience: Bob", 1, true) ~= nil, "host: said at the launch at the latest");
	h.tic(6);
	check(#h.chat == 1, "host: said once");

	-- Joining a game in progress: the client says nothing about the host.
	local g = NewLobby({ { nom = "Hôte", proxy = false }, { nom = "Bob" } });
	g.machines[1].env.Network.IsPlayerHotJoining = function() return true; end
	g.open();
	g.countdown();
	check(#g.chat == 0, "game in progress: no line about the host");
	check(g.machines[1].pol ~= true, "game in progress: off until the host's verdict names this machine");
end

-- 13d. A game exe BGE does not know: said to everyone on arrival, after the announcement.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { lang = "en_US", avant = function(env)
		env.Civ6Proxy_ExeInconnu = function() return env.Network.GetLocalPlayerID() == 1 and 1 or 0; end
	end });
	l.open();
	local n, text = 0, "";
	for _, c in ipairs(l.chat) do
		if c.text:find("not the current Steam version", 1, true) then n = n + 1; text = c.text; end
	end
	check(n == 1 and text == "[COLOR_RED]BGE - Bob's copy of the game is not the current Steam version: BGE's network fixes are inactive on their PC.[ENDCOLOR]", "unknown exe said once, by that player: " .. text);
end

-- 14. En cours de partie, correctifs POL actifs : un joueur sans proxy arrive. L'hôte prévient une
-- fois, quand il est prêt.
do
	local joueurs = { { nom = "Hôte" }, { nom = "Bob" }, { nom = "Léo", proxy = false, absent = true } };
	local l = NewLobby(joueurs);
	l.open();
	l.countdown();
	Lancer(l);
	local hote = l.machines[0];
	joueurs[3].absent = false;
	hote.events.MultiplayerPlayerConnected.Fire(2);
	check(#l.chat == 1, "pas encore prêt : pas de message");
	joueurs[3].pret = true;
	hote.events.PlayerInfoChanged.Fire(2);
	hote.events.PlayerInfoChanged.Fire(2);
	check(#l.chat == 2 and LastChat(l):find("^%[COLOR_RED%]BGE %- Léo: joined without BetterGameExperience"), "message de l'hôte : " .. LastChat(l));
	check(Pol(l, 0) == "actifs", "l'état ne change pas en pleine partie");
end

-- 15. Correctifs POL coupés pour la partie : l'arrivée d'un joueur sans proxy ne change rien.
do
	local joueurs = { { nom = "Hôte" }, { nom = "Bob", proxy = false }, { nom = "Léo", proxy = false, absent = true } };
	local l = NewLobby(joueurs);
	l.open();
	l.countdown();
	Lancer(l);
	joueurs[3].absent, joueurs[3].pret = false, true;
	l.machines[0].events.PlayerInfoChanged.Fire(2);
	check(#l.chat == 1, "POL coupés : pas d'avertissement en partie");
end

-- 15 bis. En pleine partie, une migration d'hôte ou une connexion ne rouvre pas le salon : l'état
-- reste celui du lancement sur chaque machine.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } });
	l.open();
	l.countdown();
	Lancer(l);
	l.host = 1;
	for _, m in pairs(l.machines) do
		m.events.MultiplayerHostMigrated.Fire(1);
		m.events.MultiplayerPlayerConnected.Fire(2);
	end
	for id = 0, 2 do check(Pol(l, id) == "actifs", "migration en partie, machine " .. id .. " : " .. Pol(l, id)); end
end

-- 16. Écran de chargement : actifs en solo et en hotseat, inchangé en réseau.
do
	for _, cas in ipairs({ { reseau = false, attendu = true }, { reseau = true, attendu = false } }) do
		local etat = false;
		local ev = NewEvent();
		local env = setmetatable({
			Events = { LoadScreenContentReady = ev, LoadScreenClose = NewEvent() },
			GameConfiguration = {
				IsNetworkMultiplayer = function() return cas.reseau; end,
				IsHotseat = function() return false; end,
			},
			Civ6Proxy_FixerPol = function(a) etat = (a == 1); end,
			Civ6Proxy_Journal = function() end,
		}, { __index = _G });
		assert(load(CHARGEMENT, "=chargement.lua", "t", env))();
		ev.Fire();
		check(etat == cas.attendu, "chargement, réseau " .. tostring(cas.reseau));
	end
end

-- 17. Diagnostic du bouton Prêt : une ligne à l'apparition de l'erreur d'un joueur, rien tant
-- qu'elle dure, une ligne quand elle est levée ; le bouton d'origine est toujours appelé.
do
	local log, appels, erreur = {}, 0, nil;
	local leader = { Domain = "Players:Expansion2_Players", Values = { { Value = "LEADER_A" } } };
	local env = setmetatable({
		UpdateReadyButton = function() appels = appels + 1; end,
		GetPlayerParameterError = function(id) if id == 1 then return erreur; end end,
		GetGameParametersError = function() return nil; end,
		GetPlayerParameters = function(id) return { Parameters = { PlayerLeader = leader } }; end,
		PlayerConfigurations = { [0] = {}, [1] = {
			GetLeaderTypeName = function() return "LEADER_B"; end,
			GetSlotStatus = function() return 1; end,
			IsHuman = function() return true; end,
			IsParticipant = function() return true; end,
		} },
		GameConfiguration = {
			GetMultiplayerPlayerIDs = function() return { 0, 1 }; end,
			GetGameState = function() return 7; end,
		},
		DB = { ConfigurationChanges = function() return 12; end },
		Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end },
		Civ6Proxy_Journal = function(t) table.insert(log, t); end,
	}, { __index = _G });
	assert(load(BLOCK, "=salon.lua", "t", env))();
	local n = #log;
	env.UpdateReadyButton();
	check(appels == 1 and #log == n, "diag : rien sans erreur");
	erreur = { Id = "MissingDomainValue" };
	env.UpdateReadyButton();
	env.UpdateReadyButton();
	local l = log[n + 1] or "";
	check(#log == n + 1 and l:find("joueur 1 : erreur MissingDomainValue/nil", 1, true)
		and l:find("dirigeant LEADER_B", 1, true) and l:find("1 valeurs, dirigeant absent", 1, true)
		and l:find("base 12", 1, true), "diag : erreur décrite une fois (" .. l .. ")");
	erreur = nil;
	env.UpdateReadyButton();
	check(#log == n + 2 and log[n + 2]:find("joueur 1 : erreur levée", 1, true) and appels == 4, "diag : erreur levée");
end

-- 18. Cité-État d'une partie chargée : son dirigeant hors liste ne bloque plus Prêt ; une
-- civilisation majeure dans le même cas, ou une autre erreur de cité-État, si.
do
	local FULL, MINOR = 101, 202;
	local log = {};
	local erreurs = {
		[1] = { Id = "MissingDomainValue" },
		[8] = { Id = "MissingDomainValue" },
		[9] = { Id = "InvalidDomainValue", Reason = "LOC_X" },
	};
	local function Pc(niveau) return { GetCivilizationLevelTypeID = function() return niveau; end }; end
	local env = setmetatable({
		GetPlayerParameterError = function(id) return erreurs[id]; end,
		CivilizationLevelTypes = { CIVILIZATION_LEVEL_FULL_CIV = FULL },
		PlayerConfigurations = { [1] = Pc(FULL), [8] = Pc(MINOR), [9] = Pc(MINOR) },
		GameConfiguration = { GetMultiplayerPlayerIDs = function() return {}; end },
		Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end },
		Civ6Proxy_Journal = function(t) table.insert(log, t); end,
	}, { __index = _G });
	assert(load(BLOCK, "=salon.lua", "t", env))();
	local n = #log;
	check(env.GetPlayerParameterError(8) == nil, "cité-État : dirigeant hors liste ignoré");
	env.GetPlayerParameterError(8);
	check(#log == n + 1 and log[n + 1]:find("joueur 8", 1, true), "cité-État : journalisé une fois");
	check(env.GetPlayerParameterError(1) == erreurs[1], "civilisation majeure : erreur gardée");
	check(env.GetPlayerParameterError(9) == erreurs[9], "cité-État, autre erreur : gardée");
end

-- 19. Annonce d'arrivée : chaque machine équipée et active dit à tous sa version, une fois par
-- ouverture ; pas la machine désactivée, ni à une migration d'hôte ; rien en hotseat.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", version = "0.0.9" }, { nom = "Zoé", indispo = "actif=off" }, { nom = "Léo", proxy = false } }, { lang = "en_US" });
	l.open();
	check(#l.annonces == 2, "deux annonces (" .. #l.annonces .. ")");
	check(l.annonces[1].from == 0 and l.annonces[1].text == "[COLOR_GREEN]BGE - Hôte has BetterGameExperience 0.1.0 enabled.[ENDCOLOR]", "annonce de l'hôte : " .. tostring(l.annonces[1] and l.annonces[1].text));
	check(l.annonces[2].from == 1 and l.annonces[2].text:find("Bob has BetterGameExperience 0.0.9 enabled.", 1, true), "version exacte de Bob");
	check(l.annonces[1].target == -1, "pour tous");
	l.host = 1;
	for _, m in pairs(l.machines) do m.events.MultiplayerHostMigrated.Fire(1); end
	check(#l.annonces == 2, "pas de nouvelle annonce à une migration");
	l.open();
	check(#l.annonces == 4, "nouvelle ouverture : nouvelles annonces");
	local h = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { hotseat = true });
	h.open();
	check(#h.annonces == 0, "hotseat : pas d'annonce");
	-- Traduite à l'arrivée chez un joueur français.
	local env = setmetatable({ Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end } }, { __index = _G });
	assert(load(ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua"), "=t", "t", env))();
	check(env.BGD_TxtChat(l.annonces[1].text) == "[COLOR_GREEN]BGE - Hôte a BetterGameExperience 0.1.0 activé.[ENDCOLOR]", "annonce traduite : " .. tostring(env.BGD_TxtChat(l.annonces[1].text)));
end

-- 20. Avec le minuteur : l'annonce attend le premier tic.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { controles = true });
	l.open();
	check(#l.annonces == 0, "pas d'annonce avant le premier tic");
	l.tic();
	check(#l.annonces == 2, "annonces au premier tic");
	l.tic(3);
	check(#l.annonces == 2, "une seule fois");
end

-- 21. Bouton BGE Check : visible chez l'hôte actif seulement, et hors draft de MPH.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { controles = true });
	l.open();
	check(not l.bouton(0):IsHidden() and l.bouton(1):IsHidden(), "bouton chez l'hôte seul");
	check(l.bouton(0).texte == "BGE CHECK" and l.bouton(0).aide ~= nil, "texte et aide du bouton");
	local d = NewLobby({ { nom = "Hôte", indispo = "actif=off" }, { nom = "Bob" } }, { controles = true });
	d.open();
	check(d.bouton(0):IsHidden(), "hôte désactivé : pas de bouton");
	local r = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { controles = true, reset = "visible" });
	r.open();
	check(r.bouton(0):IsHidden(), "RESET de MPH affiché : bouton caché");
	local h = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { controles = true, hotseat = true });
	h.open();
	check(h.bouton(0):IsHidden(), "hotseat : pas de bouton");
end

-- 22. BGE Check, tous à jour : demandes et réponses cachées et privées, bilan vert, pour tous.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" }, { nom = "IA", ia = true, proxy = false } }, { controles = true, lang = "en_US" });
	l.open();
	l.tic();
	l.bouton(0).callbacks[1]();
	check(#l.caches == 4, "2 demandes, 2 réponses (" .. #l.caches .. ")");
	for _, c in ipairs(l.caches) do
		check(c.target == -3, "message caché en privé : " .. c.text);
	end
	check(l.caches[1].to == 1 and l.caches[1].text:find("^%.bgd_check %w+$"), "demande à Bob : " .. l.caches[1].text);
	check(l.caches[2].to == 0 and l.caches[2].text:find("^%.bgd_version %w+ 0%.1%.0 1$"), "réponse de Bob : " .. l.caches[2].text);
	local t = LastChat(l);
	check(#l.chat == 1 and l.chat[1].from == 0 and l.chat[1].target == -1, "un bilan, de l'hôte, pour tous");
	check(t == "[COLOR_GREEN]BGE - BGE Check: every player has BetterGameExperience 0.1.0 enabled (Hôte, Bob, Zoé).[ENDCOLOR]", "bilan vert : " .. t);
	check(not l.bouton(0).inactif, "bouton de nouveau disponible");
end

-- 23. BGE Check, écarts : autre version, installé mais inactif, sans réponse (sans proxy) ; bilan
-- au délai, et en français chez un joueur français.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", version = "0.0.9" }, { nom = "Zoé", indispo = "actif=off" }, { nom = "Léo", proxy = false }, { nom = "Max" } }, { controles = true, lang = "en_US" });
	l.open();
	l.tic();
	l.bouton(0).callbacks[1]();
	check(#l.chat == 0, "Léo ne répond pas : pas encore de bilan");
	check(l.bouton(0).inactif, "bouton grisé pendant le check");
	l.bouton(0).callbacks[1]();
	check(#l.caches == 7, "un second clic ne relance rien (" .. #l.caches .. ")");
	l.tic(11);
	check(#l.chat == 0, "pas de bilan avant le délai");
	l.tic();
	local t = LastChat(l);
	check(#l.chat == 1, "bilan au délai");
	check(t == "[COLOR_RED]BGE - BGE Check (host: 0.1.0). Same version: Hôte, Max. Other version: Bob (0.0.9). Installed but inactive: Zoé (0.1.0). No answer (without BetterGameExperience): Léo.[ENDCOLOR]", "bilan rouge : " .. t);
	local env = setmetatable({ Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end } }, { __index = _G });
	assert(load(ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua"), "=t", "t", env))();
	check(env.BGD_TxtChat(t) == "[COLOR_RED]BGE - BGE Check (hôte : 0.1.0). Même version : Hôte, Max. Autre version : Bob (0.0.9). Installé mais inactif : Zoé (0.1.0). Sans réponse (sans BetterGameExperience) : Léo.[ENDCOLOR]", "bilan traduit : " .. tostring(env.BGD_TxtChat(t)));
	check(not l.bouton(0).inactif, "bouton libéré après le bilan");
	l.tic(20);
	check(#l.chat == 1, "un seul bilan");
end

-- 24. Réponses refusées : jeton d'un autre check, demande qui ne vient pas de l'hôte ; un joueur
-- parti pendant le check ne bloque pas le bilan.
do
	local joueurs = { { nom = "Hôte" }, { nom = "Bob" }, { nom = "Zoé" } };
	local l = NewLobby(joueurs, { controles = true, lang = "en_US" });
	l.open();
	l.tic();
	l.machines[2].events.MultiplayerChat.Fire(1, 2, ".bgd_check abc", -3);
	check(#l.caches == 0, "demande d'un client : ignorée");
	l.coupe = true;  -- messages non remis : on les injecte à la main
	l.bouton(0).callbacks[1]();
	l.coupe = false;
	l.machines[0].events.MultiplayerChat.Fire(1, 0, ".bgd_version autre 0.1.0 1", -3);
	check(#l.chat == 0, "jeton d'un autre check : ignoré");
	local jeton = l.caches[1].text:match("^%.bgd_check (%w+)$");
	l.machines[0].events.MultiplayerChat.Fire(1, 0, ".bgd_version " .. jeton .. " 0.1.0 1", -3);
	check(#l.chat == 0, "Zoé n'a pas répondu");
	joueurs[3].absent = true;
	l.tic();
	check(#l.chat == 1 and LastChat(l):find("every player has BetterGameExperience 0.1.0 enabled (Hôte, Bob)", 1, true), "Zoé partie : bilan sans elle : " .. LastChat(l));
end

-- 25. Les messages « .bgd_ » ne s'affichent pas ; les autres passent, traduits.
do
	local recus = {};
	local env = setmetatable({
		OnChat = function(f, t, text) table.insert(recus, text); end,
		Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end },
	}, { __index = _G });
	assert(load(BLOCK, "=salon.lua", "t", env))();
	env.OnChat(1, 0, ".bgd_version j1 0.1.0 1");
	env.OnChat(1, -1, "[COLOR_GREEN]BGE - Bob has BetterGameExperience 0.1.0 enabled.[ENDCOLOR]");
	env.OnChat(1, -1, "salut");
	check(#recus == 2 and recus[1] == "[COLOR_GREEN]BGE - Bob a BetterGameExperience 0.1.0 activé.[ENDCOLOR]" and recus[2] == "salut", "chat filtré : " .. table.concat(recus, " | "));
end

-- 26. Salon de MPH : hors draft, libellé « . » vidé et tableau remonté ; draft : mise en page de MPH.
do
	local function Ctrl(cache, y, h, texte)
		local c = { cache = cache, y = y, h = h, texte = texte, calculs = 0 };
		function c:SetHide(v) self.cache = v; end
		function c:IsHidden() return self.cache; end
		function c:SetText(t) self.texte = t; end
		function c:GetText() return self.texte; end
		function c:GetOffsetY() return self.y; end
		function c:SetOffsetY(v) self.y = v; end
		function c:GetSizeY() return self.h; end
		function c:SetSizeY(v) self.h = v; end
		function c:CalculateSize() self.calculs = self.calculs + 1; end
		return c;
	end
	local grilles = 0;
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob" } }, { controles = true, avant = function(env)
		env.Controls.PhaseLabel = Ctrl(false, 99, 0, ".");
		env.Controls.PhaseLabel_Version = Ctrl(false, 150, 0, "v180");
		env.Controls.PrimaryPanelStack = Ctrl(false, 180, 0);
		env.Controls.HeaderStack = Ctrl(false, 161, 28);
		env.Controls.PrimaryStackGrid = Ctrl(false, 0, 320);
		env.Controls.BanPullDown = Ctrl(true, 200, 46);
		env.RealizeGridSize = function() grilles = grilles + 1; end
		env.Resize = function() env.Controls.PrimaryStackGrid.h = 300; end
	end });
	local c = l.machines[0].env.Controls;
	l.open();
	l.tic();
	check(c.PhaseLabel.texte == "", "libellé « . » vidé : " .. tostring(c.PhaseLabel.texte));
	check(c.PrimaryPanelStack.y == 128 and c.PrimaryStackGrid.h == 372 and c.PhaseLabel_Version.y == 98 and c.HeaderStack.y == 109 and grilles > 0,
		"tableau remonté : " .. c.PrimaryPanelStack.y .. " " .. c.PrimaryStackGrid.h .. " " .. c.PhaseLabel_Version.y);
	l.tic();
	check(c.PrimaryPanelStack.y == 128 and c.PrimaryStackGrid.h == 372, "une seule fois");
	l.machines[0].env.Resize();
	check(c.PrimaryStackGrid.h == 352, "redimensionnement de MPH : place rendue gardée : " .. c.PrimaryStackGrid.h);
	c.BanPullDown.cache = false;
	l.tic();
	check(c.PrimaryPanelStack.y == 180 and c.PrimaryStackGrid.h == 300 and c.PhaseLabel_Version.y == 150 and c.HeaderStack.y == 161, "draft : mise en page de MPH");
	c.BanPullDown.cache = true;
	c.PhaseLabel.texte = "FFA Phase 3: Let's Go!";
	l.tic();
	check(c.PrimaryPanelStack.y == 180, "libellé de phase écrit : mise en page de MPH");
end

-- 27. Salon du jeu de base (sans les contrôles de MPH) : rien ne bouge, pas d'erreur.
do
	local l = NewLobby({ { nom = "Hôte" } }, { controles = true });
	l.open();
	l.tic();
	check(not (l.machines[0].log or ""):find("erreur", 1, true), "salon de base : pas d'erreur");
end

do
    local l = NewLobby({ { nom = "Host" }, { nom = "Observer", ia = true, observer = true } }, {pings=true});
    l.open();
    l.countdown();
    check(Pol(l, 0) == "actifs" and Pol(l, 1) == "actifs", "observateur admis par le verdict de l’hôte");
    local mixed = NewLobby({ { nom = "Host" }, { nom = "Observer", ia = true, observer = true, proxy = false } }, {pings=true});
    mixed.open();
    mixed.countdown();
    check(Pol(mixed, 0) == "désactivés", "observateur connecté sans BGE : POL coupés");
    local off = NewLobby({ { nom="Host" }, { nom="Observer", ia=true, observer=true, proxy=false } }, {pings=false});
    off.open();
    off.countdown();
    check(Pol(off, 0) == "actifs", "signaux désactivés : les observateurs ne changent pas les autres POL");
end

-- Images et sons des signaux (mod local « BGE - Ping assets ») : jamais dans la configuration du
-- salon, ajoutés par l'hôte juste avant Network.LaunchGame si les signaux sont cochés et les POL
-- actifs ; le « Prêt » est gardé quand c'est le seul changement.
local MEDIA = "863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd";
-- Configuration de partie partagée : l'hôte modifie la sienne, BroadcastGameConfig la recopie chez
-- tous et y lève GameConfigChanged, que StagingRoom.lua traite en retirant le « Prêt » local.
local function LobbyPings(joueurs, options, modsDepart)
	local partage = { mods = {} };
	for _, id in ipairs(modsDepart or { "BBG" }) do table.insert(partage.mods, id); end
	local lobby;
	options.avant = function(env)
		local m;
		for _, mm in pairs(lobby and lobby.machines or {}) do if mm.env == env then m = mm; end end
		local copie = {};
		for _, id in ipairs(partage.mods) do table.insert(copie, id); end
		env.BGD_mods = copie;
		local base = env.GameConfiguration;
		env.GameConfiguration = setmetatable({
			GetEnabledMods = function()
				local t = {};
				for _, id in ipairs(env.BGD_mods) do table.insert(t, { Id = id }); end
				return t;
			end,
			ClearEnabledMods = function() env.BGD_mods = {}; end,
			AddEnabledMods = function(id)
				assert(type(id) == "string");
				for _, x in ipairs(env.BGD_mods) do if x == id then return; end end
				table.insert(env.BGD_mods, id);
			end,
		}, { __index = base });
		env.Modding = { GetModHandle = function(id) return id == MEDIA and 42 or nil; end };
		-- `medias = false` : le proxy de démarrage n'a pas pu poser les images et sons.
		local j = joueurs[env.Network.GetLocalPlayerID() + 1];
		env.Civ6Proxy_MediasPings = function() return j.medias == false and 0 or 1; end
		env.BGD_pret = true;
		env.BGD_lances = 0;
		env.SetLocalReady = function(v) env.BGD_pret = v; end
		env.Network.LaunchGame = function()
			env.BGD_lances = env.BGD_lances + 1;
			env.BGD_modsAuLancement = #env.BGD_mods;
			for _, id in ipairs(env.BGD_mods) do if id == MEDIA then env.BGD_mediaAuLancement = true; end end
		end
		env.Network.BroadcastGameConfig = function()
			for _, d in pairs(lobby.machines) do
				if d.env ~= env then
					d.env.BGD_mods = {};
					for _, id in ipairs(env.BGD_mods) do table.insert(d.env.BGD_mods, id); end
				end
				-- Gestionnaire de StagingRoom.lua, inscrit avant le bloc, puis celui du bloc.
				d.env.SetLocalReady(false);
				d.events.GameConfigChanged.Fire();
			end
		end
		-- Comme dans le jeu : Network est en lecture seule.
		local reel = env.Network;
		env.Network = setmetatable({}, { __index = reel,
			__newindex = function() error("Attempt to modify read-only table"); end });
	end
	lobby = NewLobby(joueurs, options);
	return lobby;
end
local function Media(env)
	for _, id in ipairs(env.BGD_mods) do if id == MEDIA then return true; end end
	return false;
end

-- Tous équipés, signaux cochés : rien au salon, le mod part avec le lancement, « Prêt » gardé.
do
	local l = LobbyPings({ { nom = "Hôte" }, { nom = "Bob" } }, { pings = true });
	l.open();
	check(not Media(l.machines[0].env) and not Media(l.machines[1].env), "signaux : rien dans la configuration du salon");
	l.countdown();
	check(not Media(l.machines[0].env), "signaux : rien au début du compte à rebours");
	l.machines[0].env.Network.LaunchGame();
	local h, b = l.machines[0].env, l.machines[1].env;
	check(h.BGD_lances == 1 and h.BGD_mediaAuLancement == true and h.BGD_modsAuLancement == 2, "signaux : mod ajouté avant le lancement");
	check(Media(b), "signaux : configuration reçue par le client");
	check(h.BGD_pret == true and b.BGD_pret == true, "signaux : « Prêt » gardé chez tous");
	-- Un vrai changement de réglage retire toujours le « Prêt ».
	h.Network.BroadcastGameConfig();
	check(b.BGD_pret == false, "signaux : autre changement, « Prêt » retiré");
	-- Un second lancement n'ajoute rien de plus.
	h.Network.LaunchGame();
	check(h.BGD_modsAuLancement == 2, "signaux : ajouté une seule fois");
end

-- Un joueur sans BGE, ou signaux décochés : jamais ajouté.
do
	local l = LobbyPings({ { nom = "Hôte" }, { nom = "Bob", proxy = false } }, { pings = true });
	l.open();
	l.machines[0].env.Network.LaunchGame();
	check(l.machines[0].env.BGD_lances == 1 and not Media(l.machines[0].env), "signaux : joueur sans BGE, mod non ajouté");
	local off = LobbyPings({ { nom = "Hôte" }, { nom = "Bob" } }, { pings = false });
	off.open();
	off.machines[0].env.Network.LaunchGame();
	check(not Media(off.machines[0].env), "signaux décochés : mod non ajouté");
end

-- Images et sons absents chez un joueur (dossier du jeu non modifiable) : jamais ajoutés.
do
	local l = LobbyPings({ { nom = "Hôte" }, { nom = "Bob", medias = false } }, { pings = true });
	l.open();
	check(l.actif() == "1", "médias absents : les POL restent actifs");
	l.machines[0].env.Network.LaunchGame();
	check(l.machines[0].env.BGD_lances == 1 and not Media(l.machines[0].env), "médias absents chez un client : mod non ajouté");
	check((l.machines[0].log or ""):find("absents chez Bob", 1, true) ~= nil, "médias absents : le journal nomme le joueur");
	local h = LobbyPings({ { nom = "Hôte", medias = false }, { nom = "Bob" } }, { pings = true });
	h.open();
	h.machines[0].env.Network.LaunchGame();
	check(not Media(h.machines[0].env), "médias absents chez l'hôte : mod non ajouté");
end

-- Sauvegarde d'une partie jouée avec les signaux : l'hôte retire le mod à l'ouverture, garde les autres.
do
	local l = LobbyPings({ { nom = "Hôte" }, { nom = "Bob" } }, { pings = true }, { "BBG", MEDIA, "MPH" });
	l.open();
	local h, b = l.machines[0].env, l.machines[1].env;
	check(not Media(h) and not Media(b), "sauvegarde : mod retiré chez tous");
	check(#h.BGD_mods == 2 and h.BGD_mods[1] == "BBG" and h.BGD_mods[2] == "MPH", "sauvegarde : autres mods gardés dans l'ordre");
	h.Network.LaunchGame();
	check(Media(b), "sauvegarde : remis au lancement");
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("salon.lua : tous les tests passent");
