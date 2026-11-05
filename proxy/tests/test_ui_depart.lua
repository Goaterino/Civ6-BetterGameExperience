-- Test hors jeu de src/ui_depart.lua (départ de tour simultané), sur plusieurs machines simulées
-- qui s'échangent les signaux par un chat simulé :
--   lua proxy/tests/test_ui_depart.lua
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end

local dir = (arg and arg[0] or ""):match("^(.*)[/\\]") or ".";
local BLOC = ReadFile(dir .. "/../src/ui_depart.lua");

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

local function NewEvent()
	local e = { handlers = {} };
	function e.Add(f) table.insert(e.handlers, f); end
	setmetatable(e, { __call = function(self, ...) for _, f in ipairs(self.handlers) do f(...); end end });
	return e;
end

-- Une partie : joueurs { id = { version, connecte, spectateur, humain } }, une machine par humain
-- connecté qui a le proxy (version non nulle).
local function NewGame(opts)
	local G = { machines = {}, host = opts.host or 0, clock = 0, sent = {}, options = opts };
	G.config = { CPL_SYNCTURN = opts.syncturn, BGD_DEPART_SIMULTANE = opts.option };
	G.players = opts.players;

	function G.Deliver(from, to, text)
		table.insert(G.sent, { from = from, to = to, text = text });
		local m = G.machines[to];
		if m ~= nil and G.players[to].connecte then
			m.Events.MultiplayerChat(from, to, text, 3);
		end
	end

	function G.NewMachine(id)
		local m = { id = id, natives = { attente = false, attendre = 0, partir = 0, joueur = nil }, locaux = {}, update = nil };
		local P = G.players[id];
		local Events = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
		local LuaEvents = setmetatable({}, { __index = function(t, k)
			local e = NewEvent();
			if k == "BGD_DepartTexte" then e.Add(function(texte) table.insert(m.locaux, texte); end); end
			rawset(t, k, e);
			return e;
		end });
		m.Events = Events;
		local cfg = G.config;
		local env = {
			pcall = pcall, print = function() end, tostring = tostring, tonumber = tonumber, type = type,
			ipairs = ipairs, pairs = pairs, table = table, string = string, os = { clock = function() return G.clock; end },
			Events = Events, LuaEvents = LuaEvents,
			Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end },
			GameConfiguration = {
				GetValue = function(k) return cfg[k]; end,
				SetValue = function(k, v) cfg[k] = v; m.setvalue = (m.setvalue or 0) + 1; end,
				IsNetworkMultiplayer = function() return true; end,
				IsPlayByCloud = function() return false; end,
				IsHotseat = function() return false; end,
				GetMultiplayerPlayerIDs = function()
					local t = {};
					for pid in pairs(G.players) do table.insert(t, pid); end
					table.sort(t);
					return t;
				end,
			},
			PlayerConfigurations = setmetatable({}, { __index = function(_, pid)
				local p = G.players[pid];
				if p == nil then return nil; end
				return {
					IsHuman = function() return p.humain ~= false; end,
					GetLeaderTypeName = function() return p.spectateur and "LEADER_SPECTATOR" or "LEADER_X"; end,
					GetValue = function(_, k) if k == "BGD_VERSION" and p.version then return p.version .. "@h#" .. pid; end end,
					GetPlayerName = function() return "J" .. pid; end,
				};
			end }),
			Players = setmetatable({}, { __index = function(_, pid)
				return { IsAlive = function() return true; end, IsTurnActive = function() return m.tourActif == true; end };
			end }),
			Network = {
				GetLocalPlayerID = function() return id; end,
				IsGameHost = function() return id == G.host; end,
				GetGameHostPlayerID = function() return G.host; end,
				IsPlayerConnected = function(pid) return G.players[pid].connecte; end,
				SendChat = function(text, target, to)
					if target == -1 then
						for pid in pairs(G.machines) do G.Deliver(id, pid, text); end
					else
						G.Deliver(id, to, text);
					end
				end,
			},
			ChatTargetTypes = { CHATTARGET_ALL = -1, CHATTARGET_PLAYER = 3 },
			PlayerOperations = { EXECUTE_SCRIPT = 3735928559 },
			Game = { GetCurrentGameTurn = function() return G.tour; end },
			ContextPtr = {
				SetUpdate = function(_, f) m.update = f; end,
				ClearUpdate = function() m.update = nil; end,
			},
			Civ6RolloverProxy_DepartDispo = function() return opts.dispo == false and 0 or 1, 1; end,
			Civ6RolloverProxy_DepartAttendre = function(joueur, exec)
				m.natives.attente = true;
				m.natives.attendre = m.natives.attendre + 1;
				m.natives.joueur, m.natives.exec = joueur, exec;
			end,
			Civ6RolloverProxy_DepartPartir = function()
				local was = m.natives.attente;
				m.natives.attente = false;
				m.natives.partir = m.natives.partir + 1;
				return was and 1 or 0;
			end,
			Civ6RolloverProxy_DepartEtat = function() return m.natives.attente and 1 or 0, m.natives.rejets or 0, 0; end,
			Civ6RolloverProxy_DepartJournal = function(t) table.insert(m.journal, t); end,
		};
		m.journal = {};
		-- ContextPtr:ClearUpdate() s'appelle avec « : ».
		env.ContextPtr.ClearUpdate = function() m.update = nil; end;
		local chunk = assert(load(BLOC, "@ui_depart.lua", "t", env));
		chunk();
		m.env = env;
		G.machines[id] = m;

		function m.Charger()
			m.Events.LoadScreenClose();
		end
		function m.Debut()
			m.tourActif = true;
			m.Events.LocalPlayerTurnBegin();
		end
		function m.Publie()
			m.Events.GameCoreEventPublishComplete();
		end
		function m.Tic(dt)
			if m.update then m.update(dt or 1); end
		end
		return m;
	end

	for pid, p in pairs(G.players) do
		if p.version ~= nil and p.connecte and p.humain ~= false then
			G.NewMachine(pid);
		end
	end
	G.tour = 10;
	return G;
end

local V = "0.2.0";

do
	-- Tous équipés : l'hôte part quand les deux clients sont prêts, eux à réception.
	local G = NewGame({ option = true, syncturn = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true }, [2] = { version = V, connecte = true },
	} });
	local h, a, b = G.machines[0], G.machines[1], G.machines[2];
	Check("CPL_SYNCTURN de MPH coupé au chargement", G.config.CPL_SYNCTURN == false);
	for _, m in pairs(G.machines) do m.Charger(); end
	Check("annonce de l'hôte en vert", #G.sent == 3 and G.sent[1].text:find("COLOR_GREEN") ~= nil);
	G.sent = {};
	G.tour = 11;
	h.Debut(); h.Publie();
	Check("hôte en attente, ordres retenus", h.natives.attente and h.natives.joueur == 0 and h.natives.exec == 3735928559);
	a.Debut();
	Check("pas de signal avant que l'interface ait rattrapé", #G.sent == 0);
	a.Publie();
	Check("client prêt : .bgd_pret 11 à l'hôte", #G.sent == 1 and G.sent[1].text == ".bgd_pret 11" and G.sent[1].to == 0);
	Check("pas de départ tant que B n'est pas prêt", h.natives.attente and a.natives.attente);
	b.Debut(); b.Publie();
	Check("départ envoyé à A et B", #G.sent == 4 and G.sent[3].text == ".bgd_depart 11" and G.sent[4].text == ".bgd_depart 11");
	Check("tous partis", not h.natives.attente and not a.natives.attente and not b.natives.attente);
	Check("un seul départ par machine", h.natives.partir == 1 and a.natives.partir == 1 and b.natives.partir == 1);
	h.Publie(); a.Publie();
	Check("rien de plus envoyé ensuite", #G.sent == 4);

	-- Tour suivant : le client finit avant que l'interface de l'hôte ait rattrapé.
	G.sent = {};
	G.tour = 12;
	a.Debut(); a.Publie();
	b.Debut(); b.Publie();
	Check("signaux reçus avant le début de tour de l'hôte", #G.sent == 2 and a.natives.attente);
	h.Debut(); h.Publie();
	Check("l'hôte prêt part aussitôt", not h.natives.attente and not a.natives.attente and not b.natives.attente);
end

do
	-- Option décochée : rien n'est retenu, la synchro de MPH est quand même coupée.
	local G = NewGame({ option = false, syncturn = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true },
	} });
	for _, m in pairs(G.machines) do m.Charger(); end
	G.tour = 11;
	G.machines[0].Debut(); G.machines[0].Publie();
	Check("option décochée : pas d'attente", G.machines[0].natives.attendre == 0 and #G.sent == 0);
	Check("option décochée : CPL_SYNCTURN coupé", G.config.CPL_SYNCTURN == false);
end

do
	-- Proxy indisponible (verdict POL faux) : pas d'attente, l'hôte le dit en rouge.
	local G = NewGame({ option = true, dispo = false, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true },
	} });
	for _, m in pairs(G.machines) do m.Charger(); end
	Check("verdict faux : annonce en rouge", G.sent[1] and G.sent[1].text:find("COLOR_RED") ~= nil);
	G.tour = 11;
	G.machines[0].Debut();
	Check("verdict faux : pas d'attente", G.machines[0].natives.attendre == 0);
end

do
	-- Un humain sans BetterGameDLLs (arrivé en cours de partie) n'est pas attendu ; un spectateur non
	-- plus, et il n'attend pas.
	local G = NewGame({ option = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = nil, connecte = true },
		[2] = { version = V, connecte = true, spectateur = true },
	} });
	for _, m in pairs(G.machines) do m.Charger(); end
	G.sent = {};
	G.tour = 11;
	G.machines[2].Debut(); G.machines[2].Publie();
	Check("spectateur : pas d'attente", G.machines[2].natives.attendre == 0 and #G.sent == 0);
	G.machines[0].Debut(); G.machines[0].Publie();
		Check("sans proxy et spectateur non attendus : l'hôte part seul", not G.machines[0].natives.attente and #G.sent == 0);
end

do
	-- Écran de chargement : le signal attend sa fermeture.
	local G = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true } } });
	local h, a = G.machines[0], G.machines[1];
	G.tour = 5;
	h.Debut(); h.Publie(); a.Debut(); a.Publie();
	Check("chargement : en attente, aucun signal", h.natives.attente and a.natives.attente and #G.sent == 0);
	h.Charger();
	Check("hôte chargé : il attend le client", h.natives.attente);
	a.Charger();
	Check("client chargé : départ pour les deux", not h.natives.attente and not a.natives.attente);
end

do
	-- Retardataire : après 60 s, l'hôte part sans lui et le nomme ; le client part en arrivant.
	local G = NewGame({ option = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true }, [2] = { version = V, connecte = true },
	} });
	local h, a, b = G.machines[0], G.machines[1], G.machines[2];
	for _, m in pairs(G.machines) do m.Charger(); end
	G.tour = 11;
	h.Debut(); h.Publie(); a.Debut(); a.Publie();
	G.clock = 30; h.Tic();
	Check("30 s : on attend encore B", h.natives.attente and a.natives.attente);
	G.clock = 61; h.Tic();
	Check("61 s : départ sans B", not h.natives.attente and not a.natives.attente);
	local nomme = false;
	for _, t in ipairs(h.locaux) do if t:find("sans J2") then nomme = true; end end
	Check("l'hôte nomme le retardataire", nomme);
	b.Debut(); b.Publie();
	Check("le retardataire, déjà lancé, n'attend pas", not b.natives.attente);
end

do
	-- Un joueur se déconnecte pendant l'attente : il n'est plus attendu.
	local G = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true } } });
	local h = G.machines[0];
	for _, m in pairs(G.machines) do m.Charger(); end
	G.tour = 11;
	h.Debut(); h.Publie();
	G.players[1].connecte = false;
	h.Events.MultiplayerPlayerDisconnected(1);
	Check("déconnexion : l'hôte part", not h.natives.attente);
end

do
	-- Reconnexion en plein tour : le revenant se déclare prêt, l'hôte lui renvoie le départ.
	local G = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = false } } });
	local h = G.machines[0];
	h.Charger();
	G.tour = 11;
	h.Debut(); h.Publie();
	Check("seul : l'hôte part aussitôt", not h.natives.attente);
	G.players[1].connecte = true;
	local a = G.NewMachine(1);
	a.tourActif = true;
	a.Charger();
	Check("revenant : départ renvoyé, attente levée", not a.natives.attente and a.natives.partir == 1);
end

do
	-- Signal envoyé par un autre que l'hôte : ignoré.
	local G = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true }, [2] = { version = V, connecte = true } } });
	local a = G.machines[1];
	for _, m in pairs(G.machines) do m.Charger(); end
	G.tour = 11;
	a.Debut(); a.Publie();
	a.Events.MultiplayerChat(2, 1, ".bgd_depart 11", 3);
	Check("départ d'un non-hôte ignoré", a.natives.attente);
end

do
	-- Migration d'hôte pendant l'attente : les signaux repartent vers le nouvel hôte.
	local G = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true }, [2] = { version = V, connecte = true } } });
	local a, b = G.machines[1], G.machines[2];
	for _, m in pairs(G.machines) do m.Charger(); end
	G.tour = 11;
	a.Debut(); a.Publie(); b.Debut(); b.Publie();
	G.players[0].connecte = false;
	G.machines[0] = nil;
	G.host = 1;
	a.Events.MultiplayerHostMigrated(1);
	b.Events.MultiplayerHostMigrated(1);
	Check("nouvel hôte : départ pour tous", not a.natives.attente and not b.natives.attente);
end

do
	-- Ordres retenus : le joueur est prévenu une fois, puis du départ.
	local G = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true } } });
	local h, a = G.machines[0], G.machines[1];
	for _, m in pairs(G.machines) do m.Charger(); end
	G.tour = 11;
	a.Debut(); a.Publie();
	a.natives.rejets = 2; a.Tic();
	a.natives.rejets = 3; a.Tic();
	Check("une seule ligne d'avertissement", #a.locaux == 1 and a.locaux[1]:find("ordres retenus") ~= nil);
	h.Debut(); h.Publie();
	Check("puis « c'est parti »", #a.locaux == 2 and a.locaux[2]:find("parti") ~= nil);
end

do
	-- ChatPanel.lua suivi de ui_depart_chat.lua : signaux cachés, sauf réglage ; lignes locales.
	local CHAT = ReadFile(dir .. "/../src/ui_depart_chat.lua");
	local function Panel(signaux)
		local shown, entries = {}, {};
		local LuaEvents = setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
		local env = { pcall = pcall, print = function() end, tostring = tostring, type = type, string = string, LuaEvents = LuaEvents,
			Controls = {}, Civ6RolloverProxy_DepartSignaux = function() return signaux; end,
			AddChatEntry = function(texte) table.insert(entries, texte); end };
		env.OnChat = function(from, to, text) table.insert(shown, text); end;
		assert(load(CHAT, "@ui_depart_chat.lua", "t", env))();
		return env, shown, entries, LuaEvents;
	end
	local env, shown, entries, LuaEvents = Panel(0);
	env.OnChat(1, 0, ".bgd_pret 11", 3);
	env.OnChat(1, 0, "bonjour", 3);
	Check("signal caché, message normal affiché", #shown == 1 and shown[1] == "bonjour");
	LuaEvents.BGD_DepartTexte("BGD - x");
	Check("ligne locale ajoutée", #entries == 1);
	local env2, shown2 = Panel(1);
	env2.OnChat(1, 0, ".bgd_depart 11", 3);
	Check("depart_signaux=on : signal affiché", #shown2 == 1);
end

print(failures == 0 and "tout est bon" or (failures .. " échec(s)"));
os.exit(failures == 0 and 0 or 1);
