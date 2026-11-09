-- Test hors jeu de src/rapport.lua :
--   lua startup_proxy/tests/test_rapport.lua
-- Un menu Échap simulé (bouton recopié par le proxy, popup, ContextPtr) et des natives pilotées par
-- le test. On vérifie la pause, les messages du chat à chaque étape (et leur traduction à
-- l'arrivée), la sauvegarde, les champs envoyés au proxy et le suivi de l'envoi.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local TEXTES = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua");
local BLOCK = TEXTES .. ReadFile(dir .. "/../src/rapport.lua");

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

-- options : multi (défaut vrai), pause (déjà en pause), sansNatives, sansBouton, lang, refus.
local function NewMenu(options)
	options = options or {};
	local multi = options.multi ~= false;
	local m = { chat = {}, journal = {}, envois = {}, saves = {}, broadcasts = 0, wantsPause = false, etat = "",
		paused = options.pause or false };

	local popup = { open = false, buttons = {}, texts = {} };
	function popup:Reset() self.buttons = {}; self.texts = {}; self.title = nil; end
	function popup:AddTitle(t) self.title = t; end
	function popup:AddText(t) table.insert(self.texts, t); end
	function popup:AddButton(label, cb, cmd) table.insert(self.buttons, { label = label, cb = cb, cmd = cmd }); end
	function popup:Open() self.open = true; end
	function popup:Click(label)
		for _, b in ipairs(self.buttons) do
			if b.label == label then self.open = false; b.cb(); return true; end
		end
		return false;
	end
	function popup:Command(cmd)
		for _, b in ipairs(self.buttons) do
			if b.cmd == cmd then self.open = false; b.cb(); return true; end
		end
		return false;
	end
	m.popup = popup;

	local bouton = { callbacks = {} };
	function bouton:SetText(t) self.text = t; end
	function bouton:SetToolTipString(t) self.tip = t; end
	function bouton:SetHide(h) self.hidden = h; end
	function bouton:RegisterCallback(ev, f) self.callbacks[ev] = f; end
	m.bouton = bouton;

	local cfg = {
		GetPlayerName = function() return "Testeur"; end,
		GetCivilizationShortDescription = function() return "LOC_CIV_FRANCE"; end,
		GetLeaderTypeName = function() return "LEADER_CATHERINE_DE_MEDICI"; end,
		SetWantsPause = function(_, v) m.wantsPause = v; end,
	};

	local events = { GameCoreEventPublishComplete = NewEvent() };
	local env = {
		string = string, type = type, pcall = pcall, error = error, tostring = tostring, table = table,
		ipairs = ipairs, pairs = pairs, tonumber = tonumber,
		Controls = { PopupRoot = {}, BGD_RapportButton = (not options.sansBouton) and bouton or nil },
		PopupDialog = { COMMAND_CONFIRM = "confirm", COMMAND_CANCEL = "cancel" },
		Mouse = { eLClick = "clic", eMouseEnter = "survol" },
		UI = { PlaySound = function() end },
		Events = events,
		Locale = {
			GetCurrentLanguage = function() return { Type = options.lang or "fr_FR" }; end,
			Lookup = function(s) return s; end,
		},
		GameConfiguration = {
			IsNetworkMultiplayer = function() return multi; end,
			IsPlayByCloud = function() return false; end,
			IsPaused = function() return m.paused; end,
			GetMultiplayerPlayerIDs = function() return { 0, 1 }; end,
		},
		PlayerConfigurations = { [0] = cfg },
		Network = {
			GetLocalPlayerID = function() return 0; end,
			IsGameHost = function() return true; end,
			BroadcastPlayerInfo = function() m.broadcasts = m.broadcasts + 1; end,
			SendChat = function(text, target, to) table.insert(m.chat, { text = text, target = target, to = to }); end,
			GetGameConfigSaveType = function() return "multi"; end,
			SaveGame = function(f) table.insert(m.saves, f); end,
		},
		ChatTargetTypes = { CHATTARGET_ALL = -1 },
		SaveLocations = { LOCAL_STORAGE = 0 },
		Game = { GetCurrentGameTurn = function() return 87; end },
		ContextPtr = {
			SetUpdate = function(_, f) m.update = f; end,
			ClearUpdate = function() m.update = nil; m.cleared = true; end,
		},
		include = function() error("PopupDialog déjà défini, include inattendu"); end,
		Civ6Proxy_Journal = function(t) table.insert(m.journal, t); end,
	};
	function env.PopupDialog:new(id) m.popupId = id; return popup; end
	if not options.sansNatives then
		env.Civ6Proxy_RapportEnvoyer = function(lignes)
			table.insert(m.envois, lignes);
			if options.refus then return nil, "un envoi est déjà en cours"; end
			return 1;
		end
		env.Civ6Proxy_RapportEtat = function()
			local e = m.etat;
			if type(e) == "table" then m.etat = ""; return e[1], e[2], e[3]; end
			return e;
		end
	end
	env._G = env;
	m.env = env;
	local chunk = assert(load(BLOCK, "rapport", "t", env));
	chunk();
	m.Clic = function() bouton.callbacks.clic(); end
	m.Image = function() if m.update then m.update(0.016); end end
	return m;
end

local function Dernier(m) return m.chat[#m.chat] and m.chat[#m.chat].text or ""; end

-- Traduction à l'arrivée, comme ChatPanel.lua (ui_depart_chat.lua) chez un joueur francophone.
local recepteur = {};
assert(load(TEXTES, "textes", "t", setmetatable(recepteur, { __index = _G })))();
local function Traduit(texte)
	local env = setmetatable({ Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end } }, { __index = _G });
	assert(load(TEXTES, "textes", "t", env))();
	return env.BGD_TxtChat(texte);
end

-- 1. Multijoueur, pas encore en pause : la pause est demandée, le chat prévenu, puis annulation.
do
	local m = NewMenu();
	check(m.bouton.text == "Signaler un bug (BGE)", "texte du bouton : " .. tostring(m.bouton.text));
	check(m.bouton.tip ~= nil, "infobulle");
	m.Clic();
	check(m.wantsPause == true and m.broadcasts == 1, "pause demandée et diffusée");
	check(#m.chat == 1 and m.chat[1].target == -1, "un message au chat de tous");
	check(Dernier(m) == "[COLOR_RED]BGE - Testeur is reporting a bug, please wait.[ENDCOLOR]", "message de début : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_RED]BGE - Testeur signale un bug, merci de patienter.[ENDCOLOR]", "traduit à l'arrivée : " .. tostring(Traduit(Dernier(m))));
	check(m.popup.open and #m.popup.buttons == 2, "formulaire ouvert, deux boutons");
	check(m.popup.title == "Signaler un bug", "titre");
	-- Échap : la commande d'annulation du popup.
	check(m.popup:Command("cancel"), "Annuler sur Échap");
	check(#m.chat == 2 and Dernier(m) == "[COLOR_GREEN]BGE - Testeur cancelled the bug report. The pause can be lifted.[ENDCOLOR]", "message d'annulation : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_GREEN]BGE - Testeur a annulé le rapport de bug. La pause peut être levée.[ENDCOLOR]", "annulation traduite");
	check(#m.envois == 0 and #m.saves == 0, "rien d'envoyé");
	check(m.wantsPause == true, "la pause n'est pas levée par le bloc");
	-- Nouveau clic : nouveau formulaire, nouveaux messages.
	m.Clic();
	check(#m.chat == 3, "deuxième rapport annoncé");
end

-- 2. Déjà en pause : pas de nouvelle demande, mais le message part.
do
	local m = NewMenu({ pause = true });
	m.Clic();
	check(m.wantsPause == false and m.broadcasts == 0, "pas de pause demandée");
	check(#m.chat == 1, "message de début envoyé quand même");
end

-- 3. Envoi réussi : message d'envoi, sauvegarde, champs, suivi, message de fin.
do
	local m = NewMenu();
	m.Clic();
	check(m.popup:Click("Envoyer le rapport"), "bouton Confirmer");
	check(Dernier(m) == "[COLOR_GREEN]BGE - Testeur is sending the bug report...[ENDCOLOR]", "message d'envoi : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_GREEN]BGE - Testeur : envoi du rapport en cours...[ENDCOLOR]", "envoi traduit : " .. tostring(Traduit(Dernier(m))));
	check(#m.saves == 1 and m.saves[1].Name == "BGE_rapport" and m.saves[1].IsQuicksave == false, "sauvegarde du rapport");
	check(#m.envois == 1, "envoi lancé");
	local lignes = m.envois[1] or "";
	for _, attendu in ipairs({ "tour=87", "joueur=Testeur", "multijoueur=1", "hote=1", "en_pause=0", "humains=2", "langue=fr_FR", "dirigeant=LEADER_CATHERINE_DE_MEDICI" }) do
		check(string.find(lignes, attendu, 1, true) ~= nil, "champ " .. attendu .. " dans : " .. lignes);
	end
	check(m.update ~= nil and m.popup.open, "suivi à chaque image, popup d'envoi");
	m.Image();
	check(#m.chat == 2, "rien tant que l'envoi tourne");
	-- Popup fermé, menu rouvert : on reprend le suivi sans nouveau message.
	m.popup:Click("Fermer");
	m.Clic();
	check(#m.chat == 2 and m.popup.open, "menu rouvert pendant l'envoi : suivi seul");
	m.etat = { "ok", "BGE-7F3A", "C:\\x.zip" };
	m.Image();
	check(Dernier(m) == "[COLOR_GREEN]BGE - Testeur: bug report sent. The pause can be lifted.[ENDCOLOR]", "message de fin : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_GREEN]BGE - Testeur : envoi du rapport terminé. La pause peut être levée.[ENDCOLOR]", "fin traduite");
	check(m.cleared and m.update == nil, "suivi arrêté");
	check(m.popup.title == "Rapport envoyé" and string.find(m.popup.texts[1], "BGE-7F3A", 1, true), "numéro affiché");
	check(#m.chat == 3, "un seul message de fin");
	m.env.Events.GameCoreEventPublishComplete.Fire();
	check(#m.chat == 3, "plus rien après la fin");
end

-- 4. Échec : archive montrée, message d'échec ; suivi par les événements du jeu, menu fermé.
do
	local m = NewMenu();
	m.Clic();
	m.popup:Click("Envoyer le rapport");
	m.update = nil;  -- menu fermé : plus d'images
	m.etat = { "echec", "relais non configuré", "C:\\Users\\x\\Desktop\\BGE_rapport_1.zip" };
	m.env.Events.GameCoreEventPublishComplete.Fire();
	check(string.find(Dernier(m), "could not be sent", 1, true) ~= nil, "message d'échec : " .. Dernier(m));
	check(Traduit(Dernier(m)) ~= nil, "échec traduit");
	check(m.popup.title == "Rapport non envoyé" and string.find(m.popup.texts[1], "BGE_rapport_1.zip", 1, true), "archive affichée");
end

-- 5. Envoi refusé par le proxy (un autre en cours) : échec tout de suite.
do
	local m = NewMenu({ refus = true });
	m.Clic();
	m.popup:Click("Envoyer le rapport");
	check(string.find(Dernier(m), "could not be sent", 1, true) ~= nil, "refus : message d'échec");
	check(m.update == nil, "pas de suivi");
end

-- 6. Solo : ni pause, ni chat ; le rapport part quand même.
do
	local m = NewMenu({ multi = false });
	m.Clic();
	check(#m.chat == 0 and m.wantsPause == false, "solo : ni chat ni pause");
	m.popup:Click("Envoyer le rapport");
	check(#m.envois == 1 and string.find(m.envois[1], "multijoueur=0", 1, true), "solo : envoi");
end

-- 7. Anglais, natives absentes, bouton absent.
do
	local m = NewMenu({ lang = "en_US" });
	check(m.bouton.text == "Report a bug (BGE)", "texte anglais");
	local s = NewMenu({ sansNatives = true });
	check(s.bouton.hidden == true and s.bouton.callbacks.clic == nil, "sans natives : bouton caché");
	local b = NewMenu({ sansBouton = true });
	check(string.find(table.concat(b.journal, "\n"), "bouton absent", 1, true) ~= nil, "sans bouton : journal");
end

if failures == 0 then
	print("rapport : tout est bon");
else
	print(failures .. " échec(s)");
	os.exit(1);
end
