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
local BLOC = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/ui_depart.lua");

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Toutes les machines créées : aucune ne doit avoir journalisé d'erreur (bloc ou gestionnaire).
local MACHINES = {};

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
		-- Événements du jeu seulement : un nom inconnu vaut nil, comme en partie (relevés dans l'UI du jeu).
		local Events = {};
		for _, k in ipairs({ "LocalPlayerTurnBegin", "TurnEnd", "GameCoreEventPublishComplete", "LoadScreenClose",
			"MultiplayerChat", "MultiplayerPostPlayerDisconnected", "MultiplayerHostMigrated", "TurnTimerUpdated" }) do
			Events[k] = NewEvent();
		end
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
			Civ6RolloverProxy_DepartDispo = function()
				return opts.dispo == false and 0 or 1, opts.minuteur == false and 0 or 1, opts.retenue and 1 or 0;
			end,
			Civ6RolloverProxy_DepartArmer = function(joueur, exec)
				m.natives.arme = true;
				m.natives.armer = (m.natives.armer or 0) + 1;
				m.natives.joueur, m.natives.exec = joueur, exec;
			end,
			Civ6RolloverProxy_FinForcer = function() m.natives.forcer = (m.natives.forcer or 0) + 1; return 1; end,
			Civ6RolloverProxy_FinLever = function() m.natives.lever = (m.natives.lever or 0) + 1; end,
			Civ6RolloverProxy_DepartSignaux = function() return opts.signaux and 1 or 0; end,
			Civ6RolloverProxy_DepartAttendre = function(joueur, exec)
				m.natives.attente = true;
				m.natives.attendre = m.natives.attendre + 1;
				m.natives.joueur, m.natives.exec = joueur, exec;
			end,
			Civ6RolloverProxy_DepartPartir = function()
				local was = m.natives.attente or m.natives.retenu;
				m.natives.attente = false;
				m.natives.retenu = false;
				m.natives.partir = m.natives.partir + 1;
				return was and 1 or 0;
			end,
			Civ6RolloverProxy_DepartEtat = function()
				return m.natives.attente and 1 or 0, m.natives.rejets or 0, 0, m.natives.retenu and 1 or 0;
			end,
			Civ6RolloverProxy_DepartJournal = function(t) table.insert(m.journal, t); end,
		};
		m.journal = {};
		-- ContextPtr:ClearUpdate() s'appelle avec « : ».
		env.ContextPtr.ClearUpdate = function() m.update = nil; end;
		local chunk = assert(load(BLOC, "@ui_depart.lua", "t", env));
		chunk();
		m.env = env;
		G.machines[id] = m;
		table.insert(MACHINES, m);

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
		-- Le proxy armé arrête la publication devant le début du tour : ordres retenus, et le Lua
		-- le voit à l'image suivante.
		function m.Retenir()
			if m.natives.arme then
				m.natives.arme = false;
				m.natives.retenu = true;
				m.natives.attente = true;
				m.tourActif = true;
			end
			m.Tic(0);
		end
		-- Départ : la publication reprend, le début du tour arrive à l'interface.
		function m.Relacher()
			if not m.natives.retenu and m.tourActif then
				m.Events.LocalPlayerTurnBegin();
				m.Publie();
			end
		end
		function m.Minuteur(ecoule, limite)
			m.Events.TurnTimerUpdated(ecoule, limite);
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
	h.Events.MultiplayerPostPlayerDisconnected(1);
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

-- Début de tour retenu avant l'interface (publication détournée) ---------------------------------
do
	local G = NewGame({ option = true, retenue = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true }, [2] = { version = V, connecte = true },
	} });
	local h, a, b = G.machines[0], G.machines[1], G.machines[2];
	for _, m in pairs(G.machines) do m.Charger(); end
	Check("retenue : armé au chargement", h.natives.armer == 1 and a.natives.joueur == 1 and a.natives.exec == 3735928559);
	G.sent = {};
	G.tour = 11;
	for _, m in pairs(G.machines) do m.Events.TurnEnd(); end
	Check("retenue : réarmé à la fin du tour", a.natives.armer == 2 and a.update ~= nil);
	a.Retenir();
	Check("retenue : client prêt dès l'arrêt de la publication", #G.sent == 1 and G.sent[1].text == ".bgd_pret 11");
	Check("retenue : pas d'attente posée par le Lua", a.natives.attendre == 0 and a.natives.retenu);
	h.Retenir();
	Check("retenue : l'hôte attend B", h.natives.retenu and #G.sent == 1);
	b.Retenir();
	Check("retenue : départ envoyé à A et B", #G.sent == 4 and G.sent[3].text == ".bgd_depart 11" and G.sent[4].text == ".bgd_depart 11");
	Check("retenue : publication rendue partout", not h.natives.retenu and not a.natives.retenu and not b.natives.retenu);
	Check("retenue : un seul départ par machine", h.natives.partir == 1 and a.natives.partir == 1 and b.natives.partir == 1);
	for _, m in pairs(G.machines) do m.Relacher(); end
	Check("retenue : le début du tour, arrivé après, ne remet pas en attente",
		h.natives.attendre == 0 and a.natives.attendre == 0 and not a.natives.attente and #G.sent == 4);
	for _, m in pairs(G.machines) do m.Tic(1); end
	Check("retenue : mise à jour coupée une fois tout fini", a.update == nil and h.update == nil);

	-- Tour suivant : un client retenu alors que l'hôte est déjà parti (retardataire) : relâché aussitôt.
	G.tour = 12;
	for _, m in pairs(G.machines) do m.Events.TurnEnd(); end
	h.Retenir(); a.Retenir();
	G.clock = 100; h.Tic(1);
	Check("retenue : l'hôte part sans B après le délai", not h.natives.retenu and not a.natives.retenu);
	b.Retenir();
	Check("retenue : B, déjà lancé, relâché aussitôt", not b.natives.retenu and b.natives.partir == 2);
end

do
	-- Spectateur, ou publication non détournée : pas armé ; l'attente garde l'ancien chemin.
	local G = NewGame({ option = true, retenue = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true, spectateur = true },
	} });
	for _, m in pairs(G.machines) do m.Charger(); end
	Check("retenue : spectateur pas armé", G.machines[1].natives.armer == nil and G.machines[0].natives.armer == 1);
	local G2 = NewGame({ option = true, players = { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true } } });
	for _, m in pairs(G2.machines) do m.Charger(); m.Events.TurnEnd(); end
	Check("sans détour de la publication : pas armé", G2.machines[0].natives.armer == nil);
end

-- Fin simultanée (même option, « No First/Last moves (BGD) ») ------------------------------------
-- `fin` : l'option est cochée (elle donne aussi le départ simultané : les machines attendent).
local function FinGame(opts)
	opts.option = opts.fin;
	opts.players = opts.players or { [0] = { version = V, connecte = true }, [1] = { version = V, connecte = true },
		[2] = { version = V, connecte = true } };
	local G = NewGame(opts);
	for _, m in pairs(G.machines) do m.Charger(); end
	G.sent = {};
	G.tour = 11;
	for _, m in pairs(G.machines) do m.Debut(); end
	return G, G.machines[0], G.machines[1], G.machines[2];
end

local function Envoyes(G, texte)
	local n = 0;
	for _, e in ipairs(G.sent) do if e.text == texte then n = n + 1; end end
	return n;
end

do
	-- Minuteur de l'hôte écoulé : fin envoyée à chacun, tour terminé chez eux.
	local G, h, a, b = FinGame({ fin = true });
	h.Minuteur(10, 60);
	Check("fin : rien avant l'échéance", Envoyes(G, ".bgd_fin 11") == 0);
	h.Minuteur(60, 60);
	Check("fin : .bgd_fin 11 envoyé à A et B", Envoyes(G, ".bgd_fin 11") == 2);
	Check("fin : tour terminé chez A et B", a.natives.forcer == 1 and b.natives.forcer == 1 and h.natives.forcer == nil);
	local ligne = false;
	for _, t in ipairs(a.locaux) do if t:find("minuteur de l'hôte") then ligne = true; end end
	Check("fin : pas de ligne locale sans depart_signaux", not ligne);
	h.Minuteur(61, 60);
	Check("fin : une seule fois par échéance", Envoyes(G, ".bgd_fin 11") == 2);
	-- Nouveau segment (congrès) : minuteur remis à zéro, réarmé.
	h.Minuteur(2, 60); h.Minuteur(60, 60);
	Check("fin : nouveau segment, nouvelle fin", Envoyes(G, ".bgd_fin 11") == 4);
	-- Tour suivant : minuteur rendu à son état, puis mise à jour encore écoulée du tour 11.
	G.sent = {};
	G.tour = 12;
	a.Debut();
	Check("fin : levée au début du tour", a.natives.lever ~= nil and a.natives.lever >= 2);
	h.Debut();
	h.Minuteur(60, 60);
	Check("fin : mise à jour périmée au début du tour ignorée", Envoyes(G, ".bgd_fin 12") == 0);
end

do
	-- Option décochée, ou minuteur de l'exe non détourné : rien n'est envoyé.
	local G, h = FinGame({ fin = false });
	h.Minuteur(10, 60); h.Minuteur(60, 60);
	Check("fin décochée : rien envoyé", #G.sent == 0);
	local G2, h2 = FinGame({ fin = true, minuteur = false });
	h2.Minuteur(10, 60); h2.Minuteur(60, 60);
	Check("fin sans minuteur : rien envoyé", Envoyes(G2, ".bgd_fin 11") == 0);
	local G3, h3 = FinGame({ fin = true });
	h3.Minuteur(0, 0);
	Check("fin sans minuteur de tour : rien envoyé", Envoyes(G3, ".bgd_fin 11") == 0);
end

do
	-- Côté client : signal d'un non-hôte, d'un autre tour, ou tour déjà fini : ignoré.
	local G, h, a, b = FinGame({ fin = true });
	a.Events.MultiplayerChat(2, 1, ".bgd_fin 11", 3);
	Check("fin d'un non-hôte ignorée", a.natives.forcer == nil);
	a.Events.MultiplayerChat(0, 1, ".bgd_fin 10", 3);
	Check("fin d'un autre tour ignorée", a.natives.forcer == nil);
	a.tourActif = false;
	a.Events.MultiplayerChat(0, 1, ".bgd_fin 11", 3);
	Check("fin quand le tour est déjà fini : ignorée", a.natives.forcer == nil);
end

do
	-- Départ donné, puis fin ; avec depart_signaux, la ligne locale de la fin s'affiche.
	local G, h, a = FinGame({ fin = true, signaux = true, players = {
		[0] = { version = V, connecte = true }, [1] = { version = V, connecte = true } } });
	h.Publie(); a.Publie();
	Check("départ puis fin : départ donné", not a.natives.attente);
	h.Minuteur(5, 60); h.Minuteur(60, 60);
	Check("départ puis fin : fin reçue", a.natives.forcer == 1);
	Check("depart_signaux=on : ligne locale de la fin", a.locaux[#a.locaux]:find("minuteur de l'hôte") ~= nil);
end

do
	-- ChatPanel.lua suivi de ui_depart_chat.lua : signaux cachés, sauf réglage ; lignes locales.
	local CHAT = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/ui_depart_chat.lua");
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

do
	local erreurs = {};
	for _, m in ipairs(MACHINES) do
		for _, t in ipairs(m.journal) do
			if t:find("erreur") then table.insert(erreurs, t); end
		end
	end
	Check("aucune erreur journalisée" .. (#erreurs > 0 and (" (" .. erreurs[1] .. ")") or ""), #erreurs == 0);
end

print(failures == 0 and "tout est bon" or (failures .. " échec(s)"));
os.exit(failures == 0 and 0 or 1);
