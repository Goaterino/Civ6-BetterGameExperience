-- Test hors jeu de src/salon.lua, réglages de la partie :
--   lua startup_proxy/tests/test_config_salon.lua
-- - le « Prêt » n'est retiré qu'à une configuration reçue différente de la précédente ;
-- - l'hôte qui se met prêt renvoie la configuration, puis son empreinte en message caché ;
-- - un client dont les réglages restent différents le dit à tous et demande de rejoindre à nouveau ;
-- - l'ouverture du salon oublie une demande de remap restée d'une partie précédente.
-- Chaque machine a sa copie de la configuration de partie ; BroadcastGameConfig recopie celle de
-- l'hôte chez les autres (sauf si `lobby.perdue`) et lève GameConfigChanged partout.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/salon.lua");

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
	e.Remove = function(h)
		for i, x in ipairs(e.handlers) do
			if x == h then table.remove(e.handlers, i); return; end
		end
	end
	e.Fire = function(...) for _, h in ipairs(e.handlers) do h(...); end end
	return e;
end

local NOMS = { "Hote", "Bob", "Alice" };
local CLES = { { ConfigurationGroup = "Game", ConfigurationId = "CITY_STATE_COUNT" },
	{ ConfigurationGroup = "Map", ConfigurationId = "MAP_SIZE" } };

local function NewLobby(n)
	local lobby = { machines = {}, chat = {}, caches = {}, host = 0 };
	for id = 0, n - 1 do
		lobby.machines[id] = { id = id, game = { CITY_STATE_COUNT = 9 }, map = { MAP_SIZE = "STANDARD" }, pret = {}, oublis = 0 };
	end

	local function Diffuser()
		local h = lobby.machines[lobby.host];
		for _, m in pairs(lobby.machines) do
			if m.id ~= lobby.host and not lobby.perdue then
				m.game, m.map = {}, {};
				for k, v in pairs(h.game) do m.game[k] = v; end
				for k, v in pairs(h.map) do m.map[k] = v; end
			end
		end
		for _, m in pairs(lobby.machines) do m.events.GameConfigChanged.Fire(); end
	end

	for id, m in pairs(lobby.machines) do
		local env = setmetatable({}, { __index = _G });
		m.env = env;
		m.events = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
		m.luaEvents = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
		env.Events, env.LuaEvents = m.events, m.luaEvents;
		env.ContextPtr = { IsHidden = function() return false; end };
		env.Locale = { GetCurrentLanguage = function() return { Type = "en_US" }; end, Lookup = function(s) return s; end };
		env.SlotStatus = { SS_TAKEN = 1, SS_OPEN = 2, SS_OBSERVER = 3 };
		env.ChatTargetTypes = { CHATTARGET_ALL = -1, CHATTARGET_PLAYER = -3 };
		local function Ctrl()
			local c = { cache = false };
			function c:SetHide(v) self.cache = v; end
			function c:IsHidden() return self.cache; end
			function c:SetDisabled() end
			function c:SetText() end
			function c:SetToolTipString() end
			function c:RegisterCallback() end
			function c:RegisterEndCallback(f) self.fin = f; end
			function c:SetToBeginning() end
			function c:Play() self.joue = true; end
			return c;
		end
		env.Controls = { BGD_CheckButton = Ctrl(), BGD_SalonMinuteur = Ctrl() };
		env.Mouse = { eLClick = 1, eMouseEnter = 2 };
		env.UI = { PlaySound = function() end };
		env.DB = {
			ConfigurationChanges = function() return 1; end,
			ConfigurationQuery = function() return CLES; end,
		};
		env.GameConfiguration = {
			GetValue = function(k) return m.game[k]; end,
			IsNetworkMultiplayer = function() return true; end,
			IsHotseat = function() return false; end,
			IsPlayByCloud = function() return false; end,
			GetRuleSet = function() return "RULESET_EXPANSION_2"; end,
			GetEnabledMods = function() return { { Id = "mph" } }; end,
			GetMultiplayerPlayerIDs = function()
				local ids = {};
				for pid = 0, n - 1 do table.insert(ids, pid); end
				return ids;
			end,
		};
		env.MapConfiguration = { GetValue = function(k) return m.map[k]; end, GetScript = function() return "Continents.lua"; end };
		env.PlayerConfigurations = setmetatable({}, { __index = function(_, pid)
			return {
				GetValue = function() return nil; end,
				SetValue = function() end,
				IsHuman = function() return true; end,
				GetSlotStatus = function() return 1; end,
				GetPlayerName = function() return NOMS[pid + 1]; end,
				GetReady = function() return m.pret[pid] == true; end,
				SetReady = function(_, v) m.pret[pid] = v; end,
			};
		end });
		env.Network = {
			IsGameHost = function() return id == lobby.host; end,
			GetGameHostPlayerID = function() return lobby.host; end,
			GetLocalPlayerID = function() return id; end,
			IsPlayerConnected = function() return true; end,
			BroadcastPlayerInfo = function() end,
			BroadcastGameConfig = function()
				lobby.diffusions = (lobby.diffusions or 0) + 1;
				Diffuser();
			end,
			SendChat = function(text, target, to)
				local c = { from = id, to = to, text = text };
				if string.sub(text, 1, 5) == ".bgd_" then
					table.insert(lobby.caches, c);
				elseif not text:find("has BetterGameExperience .- enabled") then
					table.insert(lobby.chat, c);
				end
				for _, d in pairs(lobby.machines) do
					if target ~= -3 or d.id == to or d.id == id then d.events.MultiplayerChat.Fire(id, to, text, target); end
				end
			end,
		};
		env.Civ6Proxy_Version = function() return "0.14.0"; end
		env.Civ6Proxy_FixerPol = function() end
		local jetons = 0;
		env.Civ6Proxy_Jeton = function() jetons = jetons + 1; return "j" .. id .. "x" .. jetons; end
		env.Civ6Proxy_Journal = function(t) m.log = (m.log or "") .. t .. "\n"; end
		env.Civ6Proxy_VersionInstallee = function() return "0.14.0"; end
		env.Civ6Proxy_RemapOublier = function() m.oublis = m.oublis + 1; end
		env.StartLaunchCountdown = function() end
		-- Ce que fait StagingRoom.lua (et celui de MPH), avant le bloc.
		env.SetLocalReady = function(v)
			if v ~= (m.pret[id] == true) then m.pret[id] = v; end
		end
		env.OnGameConfigChanged = function()
			env.SetLocalReady(false);
		end
		m.events.GameConfigChanged.Add(env.OnGameConfigChanged);
		assert(load(BLOCK, "=salon.lua", "t", env))();
	end

	function lobby.open()
		for id = 0, n - 1 do
			local m = lobby.machines[id];
			local ev = (id == lobby.host) and m.luaEvents.HostGame_ShowStagingRoom or m.luaEvents.JoiningRoom_ShowStagingRoom;
			ev.Fire();
		end
	end
	function lobby.tic(k)
		for _ = 1, k or 1 do
			for _, m in pairs(lobby.machines) do
				local t = m.env.Controls.BGD_SalonMinuteur;
				if t.joue and t.fin then t.joue = false; t.fin(); end
			end
		end
	end
	function lobby.pret(id, v)
		lobby.machines[id].env.SetLocalReady(v);
	end
	function lobby.estPret(id)
		return lobby.machines[id].pret[id] == true;
	end
	return lobby;
end

local function Rouges(lobby)
	local t = {};
	for _, c in ipairs(lobby.chat) do
		if c.text:find("^%[COLOR_RED%]") then table.insert(t, c); end
	end
	return t;
end

-- Configuration identique reçue : le « Prêt » reste ; configuration changée : il part.
do
	local lobby = NewLobby(3);
	lobby.open();
	check(lobby.machines[0].oublis == 1 and lobby.machines[1].oublis == 1, "remap oublié à chaque ouverture du salon");
	lobby.pret(1, true);
	lobby.machines[0].env.Network.BroadcastGameConfig();
	check(lobby.estPret(1), "configuration identique : Bob reste prêt");
	check((lobby.machines[1].log or ""):find("gardé %(configuration identique%)") ~= nil, "journalisé");
	lobby.machines[0].game.CITY_STATE_COUNT = 8;
	lobby.machines[0].env.Network.BroadcastGameConfig();
	check(not lobby.estPret(1), "configuration changée : Bob n'est plus prêt");
	-- Le joueur qui se retire lui-même du « Prêt » (bouton) n'est pas retenu.
	lobby.pret(2, true);
	lobby.pret(2, false);
	check(not lobby.estPret(2), "Alice se retire elle-même");
end

-- L'hôte se met prêt : configuration renvoyée, empreinte cachée, les prêts restent, rien en public.
do
	local lobby = NewLobby(3);
	lobby.open();
	lobby.pret(1, true);
	lobby.pret(2, true);
	lobby.diffusions = 0;
	lobby.pret(0, true);
	check(lobby.diffusions == 1, "configuration renvoyée une fois");
	check(lobby.estPret(0) and lobby.estPret(1) and lobby.estPret(2), "personne ne perd son « Prêt »");
	local empreintes = 0;
	for _, c in ipairs(lobby.caches) do
		if c.text:find("^%.bgd_cfg ") and c.from == 0 then empreintes = empreintes + 1; end
	end
	check(empreintes == 2, "empreinte envoyée en privé à chaque client (" .. empreintes .. ")");
	lobby.tic(6);
	check(#Rouges(lobby) == 0, "mêmes réglages : rien dans le chat");
	check((lobby.machines[1].log or ""):find("same settings as the host") ~= nil, "client : comparé");
	-- Déjà prêt, l'hôte ne renvoie rien de plus.
	lobby.pret(0, true);
	check(lobby.diffusions == 1, "pas de second envoi");
end

-- Réglages restés différents chez un client (la configuration de l'hôte ne lui parvient pas) :
-- il le dit à tous, une fois, après le délai.
do
	local lobby = NewLobby(3);
	lobby.open();
	lobby.machines[1].game.CITY_STATE_COUNT = 12;
	lobby.perdue = true;
	lobby.pret(0, true);
	check(#Rouges(lobby) == 0, "pas tout de suite : la configuration peut être en route");
	lobby.tic(6);
	local r = Rouges(lobby);
	check(#r == 1 and r[1].from == 1, "Bob le dit à tous");
	check(r[1] and r[1].text:find("Bob's game settings differ from the host's%. Bob, please leave the lobby and join it again") ~= nil,
		"message : " .. tostring(r[1] and r[1].text));
	check((lobby.machines[1].log or ""):find("CITY_STATE_COUNT=n12") ~= nil, "réglages de Bob au journal");
	lobby.tic(6);
	check(#Rouges(lobby) == 1, "une seule fois");
end

-- Différence rattrapée pendant le délai : rien en public.
do
	local lobby = NewLobby(2);
	lobby.open();
	lobby.machines[1].game.CITY_STATE_COUNT = 12;
	lobby.perdue = true;
	lobby.pret(0, true);
	lobby.tic(1);
	lobby.perdue = false;
	lobby.machines[0].env.Network.BroadcastGameConfig();
	lobby.tic(6);
	check(#Rouges(lobby) == 0, "rattrapé : rien dans le chat");
	check((lobby.machines[1].log or ""):find("after a wait") ~= nil, "rattrapage journalisé");
end

-- Un joueur arrive alors que l'hôte est prêt : configuration et empreinte renvoyées après le délai.
do
	local lobby = NewLobby(2);
	lobby.open();
	lobby.pret(0, true);
	lobby.diffusions = 0;
	lobby.machines[0].events.MultiplayerPlayerConnected.Fire(1);
	check(lobby.diffusions == 0, "pas tout de suite");
	lobby.tic(5);
	check(lobby.diffusions == 1, "renvoyée après l'arrivée");
end

if failures == 0 then
	print("config salon : tous les tests passent");
else
	print(failures .. " échec(s)");
	os.exit(1);
end
