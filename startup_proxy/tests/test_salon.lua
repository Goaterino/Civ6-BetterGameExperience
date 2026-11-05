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
local BLOCK = ReadFile(dir .. "/../src/salon.lua");
-- Relais de l'hôte en cours de partie (InGame.lua) et écran de chargement (LoadScreen.lua).
local PARTIE = ReadFile(dir .. "/../src/partie.lua");
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
	local lobby = { machines = {}, chat = {}, host = 0, tokens = 0 };

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
		env.SlotStatus = { SS_TAKEN = 1, SS_OPEN = 2 };
		env.ChatTargetTypes = { CHATTARGET_ALL = -1 };
		env.GameConfiguration = {
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
				GetSlotStatus = function() return 1; end,
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
			SendChat = function(text, target, to) table.insert(lobby.chat, { from = id, text = text, target = target }); end,
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
	check(LastChat(l):find("^%[COLOR_GREEN%]BGD %- ") and LastChat(l):find("correctifs POL actifs", 1, true), "message vert : " .. LastChat(l));
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
	check(t:find("^%[COLOR_RED%]BGD %- Sans BetterGameDLLs : Bob%."), "message rouge : " .. t);
	check(t:find("Correctifs POL désactivés pour cette partie.", 1, true), "correctifs POL désactivés : " .. t);
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
	check(LastChat(l):find("Sans BetterGameDLLs : Bob", 1, true), "Bob compté sans proxy : " .. LastChat(l));
end

-- 4. Autre version, en anglais.
do
	local l = NewLobby({ { nom = "Host" }, { nom = "Bob", version = "0.0.9" } }, { lang = "en_US" });
	l.open();
	check(l.actif() == "0", "autre version : verdict 0");
	l.countdown();
	local t = LastChat(l);
	check(t:find("BetterGameDLLs version other than 0.1.0: Bob (0.0.9).", 1, true), "version citée : " .. t);
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
		check(c.from ~= 0 and c.text:find("^%[COLOR_RED%]BGD %- L'hôte n'a pas BetterGameDLLs"), "message d'un client : " .. c.text);
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
	check(#l.chat >= 1 and l.chat[1].text:find("L'hôte n'a pas BetterGameDLLs", 1, true), "un client prévient");
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
-- s'annonce pas, compte comme sans BetterGameDLLs, et n'active rien chez lui.
do
	local l = NewLobby({ { nom = "Hôte" }, { nom = "Bob", indispo = "proxys désactivés (actif=off)" } });
	l.machines[1].pol = true;  -- reste d'une partie solo
	l.open();
	check(l.actif() == "0", "Bob indisponible : verdict 0");
	check(l.machines[1].values[1].BGD_VERSION == nil, "Bob ne s'annonce pas");
	l.countdown();
	check(LastChat(l):find("Sans BetterGameDLLs : Bob", 1, true), "Bob listé : " .. LastChat(l));
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
	check(#l.chat == 2 and LastChat(l):find("^%[COLOR_RED%]BGD %- Léo : arrivée sans BetterGameDLLs"), "message de l'hôte : " .. LastChat(l));
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
			Events = { LoadScreenContentReady = ev },
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

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("salon.lua : tous les tests passent");
