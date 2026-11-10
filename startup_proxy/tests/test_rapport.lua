-- Test hors jeu de src/rapport.lua (menu Échap) et src/rapport_suivi.lua (InGame) :
--   lua startup_proxy/tests/test_rapport.lua
-- Deux contextes simulés qui partagent leurs LuaEvents, comme dans le jeu : le menu Échap (bouton
-- et formulaire ajoutés par le proxy, popup, ContextPtr) et InGame (minuteur du suivi). Natives
-- pilotées par le test. On vérifie l'absence de pause, les messages du chat à chaque étape (couleurs et
-- traduction à l'arrivée), le formulaire et sa description, la sauvegarde, les champs envoyés, et la
-- fin : message à tous et ligne locale du chat, menu ouvert, menu fermé et partie en pause.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local TEXTES = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua");
local FIN = ReadFile(dir .. "/../src/rapport_fin.lua");
local BLOCK = TEXTES .. FIN .. ReadFile(dir .. "/../src/rapport.lua");
local SUIVI = TEXTES .. FIN .. ReadFile(dir .. "/../src/rapport_suivi.lua");

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
	return setmetatable(e, { __call = function(self, ...) self.Fire(...); end });
end

-- LuaEvents : un événement par nom, créé au premier accès.
local function NewLuaEvents()
	return setmetatable({}, { __index = function(t, k) local e = NewEvent(); rawset(t, k, e); return e; end });
end

-- Contrôle générique : texte, visibilité, rappels, taille, animation.
local function Control(hidden)
	local c = { text = "", hidden = hidden or false, callbacks = {}, sizeY = 100 };
	function c:SetText(t) self.text = t; end
	function c:GetText() return self.text; end
	function c:SetHide(h) self.hidden = h; end
	function c:IsHidden() return self.hidden; end
	function c:SetToolTipString(t) self.tip = t; end
	function c:RegisterCallback(ev, f) self.callbacks[ev] = f; end
	function c:TakeFocus() self.focus = true; end
	function c:CalculateSize() end
	function c:GetSizeY() return self.sizeY; end
	function c:SetSizeY(h) self.sizeY = h; end
	function c:SetToBeginning() self.debut = (self.debut or 0) + 1; end
	function c:Play() self.joue = true; end
	function c:RegisterEndCallback(f) self.fin = f; end
	return c;
end

local IDS_FORMULAIRE = { "BGD_RapportFond", "BGD_RapportBord", "BGD_RapportCadre", "BGD_RapportPile", "BGD_RapportTitre",
	"BGD_RapportExplication", "BGD_RapportConsigne", "BGD_RapportTexte", "BGD_RapportConfirmer", "BGD_RapportAnnuler" };

-- options : multi (défaut vrai), pause (déjà en pause), sansNatives, sansBouton, sansFormulaire,
-- sansMinuteur, sansSuivi (InGame sans le bloc), lang, refus, steam (identifiant rendu par le jeu).
local function NewJeu(options)
	options = options or {};
	local multi = options.multi ~= false;
	local m = { chat = {}, locales = {}, journal = {}, envois = {}, saves = {}, broadcasts = 0, wantsPause = false, etat = "",
		paused = options.pause or false, ouvertures = 0, menuCache = false };
	local luaEvents = NewLuaEvents();

	local popup = { open = false, buttons = {}, texts = {} };
	function popup:Reset() self.buttons = {}; self.texts = {}; self.title = nil; end
	function popup:AddTitle(t) self.title = t; end
	function popup:AddText(t) table.insert(self.texts, t); end
	function popup:AddButton(label, cb, cmd) table.insert(self.buttons, { label = label, cb = cb, cmd = cmd }); end
	function popup:Open() self.open = true; end
	function popup:Close() self.open = false; end
	function popup:Click(label)
		for _, b in ipairs(self.buttons) do
			if b.label == label then self.open = false; b.cb(); return true; end
		end
		return false;
	end
	m.popup = popup;

	local cfg = {
		GetPlayerName = function() return "Testeur"; end,
		GetCivilizationShortDescription = function() return "LOC_CIV_FRANCE"; end,
		GetLeaderTypeName = function() return "LEADER_CATHERINE_DE_MEDICI"; end,
		GetNetworkIdentifer = function() return options.steam; end,
		SetWantsPause = function(_, v) m.wantsPause = v; end,
		IsHuman = function() return true; end,
		GetSlotStatus = function() return "pris"; end,
	};
	-- Emplacement de configuration : humain ou IA, occupé ou non.
	local function Emplacement(humain, statut)
		return { IsHuman = function() return humain; end, GetSlotStatus = function() return statut; end };
	end

	local controls = { PopupRoot = {} };
	if not options.sansBouton then controls.BGD_RapportButton = Control(); end
	if not options.sansFormulaire then
		for _, id in ipairs(IDS_FORMULAIRE) do controls[id] = Control(id == "BGD_RapportFond"); end
	end
	m.controls = controls;
	m.bouton = controls.BGD_RapportButton;

	-- Le menu se rouvre par l'événement que la touche Échap emploie : il ne doit plus s'en servir.
	luaEvents.InGame_OpenInGameOptionsMenu.Add(function() m.ouvertures = m.ouvertures + 1; m.menuCache = false; end);
	-- Lignes locales du chat (ChatPanel, ui_depart_chat.lua du proxy de GameCore).
	luaEvents.BGD_DepartTexte.Add(function(t) table.insert(m.locales, t); end);

	local function Env(extra)
		local env = {
			string = string, type = type, pcall = pcall, error = error, tostring = tostring, table = table,
			ipairs = ipairs, pairs = pairs, tonumber = tonumber,
			LuaEvents = luaEvents,
			Locale = {
				GetCurrentLanguage = function() return { Type = options.lang or "fr_FR" }; end,
				Lookup = function(s) return s; end,
			},
			Civ6Proxy_Journal = function(t) table.insert(m.journal, t); end,
			Events = { GameCoreEventPublishComplete = NewEvent() },
			GameConfiguration = {
				IsNetworkMultiplayer = function() return multi; end,
				IsPlayByCloud = function() return false; end,
				IsPaused = function() return m.paused; end,
				-- Tous les emplacements : deux humains, une IA, un emplacement humain vide.
				GetMultiplayerPlayerIDs = function() return { 0, 1, 2, 3 }; end,
			},
			PlayerConfigurations = { [0] = cfg, [1] = Emplacement(true, "pris"), [2] = Emplacement(false, "pris"),
				[3] = Emplacement(true, "ouvert") },
			SlotStatus = { SS_TAKEN = "pris", SS_OPEN = "ouvert" },
			Network = {
				GetLocalPlayerID = function() return 0; end,
				IsGameHost = function() return true; end,
				BroadcastPlayerInfo = function() m.broadcasts = m.broadcasts + 1; end,
				SendChat = function(text, target, to) table.insert(m.chat, { text = text, target = target, to = to }); end,
				GetGameConfigurationSaveType = function() return "multi"; end,
				SaveGame = function(f) table.insert(m.saves, f); end,
			},
			ChatTargetTypes = { CHATTARGET_ALL = -1 },
		};
		if not options.sansNatives then
			env.Civ6Proxy_RapportEtat = function()
				m.interrogations = (m.interrogations or 0) + 1;
				local e = m.etat;
				if type(e) == "table" then m.etat = ""; return e[1], e[2], e[3]; end
				return e;
			end
		end
		for k, v in pairs(extra) do env[k] = v; end
		env._G = env;
		return env;
	end

	-- Le menu Échap.
	local menu = Env({
		Controls = controls,
		PopupDialog = { COMMAND_CONFIRM = "confirm", COMMAND_CANCEL = "cancel" },
		Mouse = { eLClick = "clic", eMouseEnter = "survol" },
		UI = { PlaySound = function() end },
		SaveLocations = { LOCAL_STORAGE = 0 },
		Game = { GetCurrentGameTurn = function() return 87; end },
		ContextPtr = {
			SetUpdate = function(_, f) m.update = f; end,
			ClearUpdate = function() m.update = nil; m.cleared = true; end,
			IsHidden = function() return m.menuCache; end,
		},
		include = function() error("PopupDialog déjà défini, include inattendu"); end,
	});
	function menu.PopupDialog:new(id) m.popupId = id; return popup; end
	if not options.sansNatives then
		menu.Civ6Proxy_RapportEnvoyer = function(lignes)
			table.insert(m.envois, lignes);
			if options.refus then return nil, "un envoi est déjà en cours"; end
			return 1;
		end
	end
	m.menu = menu;
	assert(load(BLOCK, "rapport", "t", menu))();

	-- InGame, avec le minuteur du suivi.
	local minuteur = (not options.sansMinuteur) and Control() or nil;
	m.minuteur = minuteur;
	m.ingame = Env({ Controls = { BGD_RapportMinuteur = minuteur } });
	if not options.sansSuivi then assert(load(SUIVI, "rapport_suivi", "t", m.ingame))(); end

	m.Clic = function() m.bouton.callbacks.clic(); end
	m.Ecrire = function(t) controls.BGD_RapportTexte.text = t; end
	m.Confirmer = function() controls.BGD_RapportConfirmer.callbacks.clic(); end
	m.Annuler = function() controls.BGD_RapportAnnuler.callbacks.clic(); end
	m.Formulaire = function() return controls.BGD_RapportFond ~= nil and not controls.BGD_RapportFond.hidden; end
	-- Une image du menu, s'il est affiché ; une fin d'animation du minuteur, s'il tourne.
	m.Image = function() if m.update and not m.menuCache then m.update(0.016); end end
	m.Tic = function()
		if minuteur and minuteur.joue then minuteur.joue = false; minuteur.fin(); end
	end
	return m;
end

local function Dernier(m) return m.chat[#m.chat] and m.chat[#m.chat].text or ""; end
local function Locale(m) return m.locales[#m.locales] or ""; end

-- Traduction à l'arrivée, comme ChatPanel.lua (ui_depart_chat.lua) chez un joueur francophone.
local function Traduit(texte)
	local env = setmetatable({ Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end } }, { __index = _G });
	assert(load(TEXTES, "textes", "t", env))();
	return env.BGD_TxtChat(texte);
end

-- 1. Multijoueur : pas de pause, le chat prévenu, puis annulation.
do
	local m = NewJeu();
	check(m.bouton.text == "Signaler un bug (BGE)", "texte du bouton : " .. tostring(m.bouton.text));
	check(m.bouton.tip ~= nil, "infobulle");
	check(not m.Formulaire(), "formulaire caché au départ");
	m.Clic();
	check(m.wantsPause == false and m.broadcasts == 0, "pas de pause demandée");
	check(#m.chat == 1 and m.chat[1].target == -1, "un message au chat de tous");
	check(Dernier(m) == "[COLOR_Civ6Red]BGE - Testeur is reporting a bug.[ENDCOLOR]", "message de début : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_Civ6Red]BGE - Testeur signale un bug.[ENDCOLOR]", "traduit à l'arrivée : " .. tostring(Traduit(Dernier(m))));
	check(m.Formulaire(), "formulaire ouvert");
	check(m.controls.BGD_RapportTitre.text == "Signaler un bug", "titre");
	check(string.find(m.controls.BGD_RapportConsigne.text, "Décrivez le problème", 1, true) ~= nil, "consigne du champ");
	check(m.controls.BGD_RapportConfirmer.text == "Envoyer le rapport" and m.controls.BGD_RapportAnnuler.text == "Annuler", "boutons");
	check(m.controls.BGD_RapportTexte.focus == true, "le champ prend le focus");
	check(m.controls.BGD_RapportCadre.sizeY == 136 and m.controls.BGD_RapportBord.sizeY == 140, "cadre ajusté au contenu");
	check(not string.find(table.concat({ m.controls.BGD_RapportExplication.text, m.controls.BGD_RapportConsigne.text }), "Discord", 1, true), "pas de Discord dans le formulaire");
	m.Ecrire("brouillon");
	m.Annuler();
	check(not m.Formulaire(), "formulaire fermé");
	check(#m.chat == 2 and Dernier(m) == "[COLOR_Civ6Green]BGE - Testeur cancelled the bug report.[ENDCOLOR]", "message d'annulation : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_Civ6Green]BGE - Testeur a annulé le rapport de bug.[ENDCOLOR]", "annulation traduite");
	check(#m.envois == 0 and #m.saves == 0, "rien d'envoyé");
	-- Nouveau rapport : nouveaux messages, champ vidé.
	m.Clic();
	check(#m.chat == 3 and m.controls.BGD_RapportTexte.text == "", "deuxième rapport annoncé, champ vide");
end

-- 2. Partie déjà en pause (par un joueur) : le bloc n'y touche pas, le message part.
do
	local m = NewJeu({ pause = true });
	m.Clic();
	check(m.wantsPause == false and m.broadcasts == 0, "pause laissée telle quelle");
	check(#m.chat == 1, "message de début envoyé");
end

-- 3. Menu fermé puis rouvert pendant le formulaire : même formulaire, texte gardé, pas de message.
do
	local m = NewJeu();
	m.Clic();
	m.Ecrire("ma description");
	m.Clic();
	check(#m.chat == 1 and m.Formulaire() and m.controls.BGD_RapportTexte.text == "ma description", "formulaire repris tel quel");
end

-- 4. Envoi réussi, menu ouvert : description et champs envoyés, suivi par image, message de fin.
do
	local m = NewJeu({ steam = "76561198000000042" });
	m.Clic();
	m.Ecrire("La partie se désynchronise\na chaque tour");
	m.Confirmer();
	check(not m.Formulaire(), "formulaire fermé à l'envoi");
	check(Dernier(m) == "[COLOR_Civ6Green]BGE - Testeur is sending the bug report...[ENDCOLOR]", "message d'envoi : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_Civ6Green]BGE - Testeur : envoi du rapport en cours...[ENDCOLOR]", "envoi traduit : " .. tostring(Traduit(Dernier(m))));
	check(#m.saves == 1 and m.saves[1].Name == "BGE_rapport" and m.saves[1].IsQuicksave == false
		and m.saves[1].Type == "multi", "sauvegarde du rapport");
	check(#m.envois == 1, "envoi lancé");
	local lignes = m.envois[1] or "";
	for _, attendu in ipairs({ "tour=87", "joueur=Testeur", "steam_id=76561198000000042", "multijoueur=1", "hote=1", "en_pause=0",
		"humains=2", "langue=fr_FR", "dirigeant=LEADER_CATHERINE_DE_MEDICI", "description=La partie se désynchronise a chaque tour" }) do
		check(string.find(lignes, attendu, 1, true) ~= nil, "champ " .. attendu .. " dans : " .. lignes);
	end
	check(m.update ~= nil and m.popup.open, "suivi à chaque image, popup d'envoi");
	check(m.minuteur.joue == true, "suivi d'InGame lancé aussi");
	m.Image();
	check(#m.chat == 2, "rien tant que l'envoi tourne");
	m.etat = { "ok", "BGE-7F3A2C", "C:\\x.zip" };
	m.Image();
	check(Dernier(m) == "[COLOR_Civ6Green]BGE - Testeur: bug report sent.[ENDCOLOR]", "message de fin : " .. Dernier(m));
	check(Traduit(Dernier(m)) == "[COLOR_Civ6Green]BGE - Testeur : envoi du rapport terminé.[ENDCOLOR]", "fin traduite");
	check(#m.locales == 1 and Locale(m) == "[COLOR_Civ6Green]BGE - Rapport de bug envoyé, numéro BGE-7F3A2C. Pour ajouter des détails, envoyez ce numéro à @goaterino en message privé sur Discord.[ENDCOLOR]", "ligne locale : " .. Locale(m));
	check(m.cleared and m.update == nil, "suivi par image arrêté");
	check(not m.popup.open, "popup d'envoi fermé, pas de popup de résultat");
	check(m.ouvertures == 0, "menu pas rouvert");
	check(string.find(table.concat(m.journal, "\n"), "fin par le menu", 1, true) == nil, "fin passée par le suivi d'InGame");
	local avant = m.interrogations;
	m.Tic();
	check(m.interrogations == avant and not m.minuteur.joue, "suivi d'InGame arrêté aussi");
	check(#m.chat == 3 and #m.locales == 1, "un seul message de fin");
	-- Nouveau rapport possible.
	m.Clic();
	check(m.Formulaire() and #m.chat == 4, "nouveau rapport après la fin");
end

-- 5. Le cas du joueur : menu fermé juste après Envoyer, partie en pause (ni images ni événements).
--    Le suivi d'InGame trouve le résultat aussitôt : message à tous et ligne locale, sans le menu.
do
	local m = NewJeu({ steam = "" });
	m.Clic();
	m.Confirmer();
	check(string.find(m.envois[1], "steam_id", 1, true) == nil, "pas de Steam ID vide envoyé");
	m.paused = true;
	m.menuCache = true;
	m.Image(); m.Tic(); m.Tic();
	check(#m.chat == 2 and #m.locales == 0, "rien tant que l'envoi tourne");
	m.etat = { "echec", "relais non configuré", "C:\\Users\\x\\Desktop\\BGE_rapport_1.zip" };
	m.Tic();
	check(Dernier(m) == "[COLOR_Civ6Red]BGE - Testeur: the bug report could not be sent, it was kept on their computer.[ENDCOLOR]", "message d'échec : " .. Dernier(m));
	check(Traduit(Dernier(m)) ~= nil, "échec traduit");
	check(#m.locales == 1 and string.find(Locale(m), "BGE_rapport_1.zip", 1, true) and string.find(Locale(m), "@goaterino", 1, true)
		and string.find(Locale(m), "[COLOR_Civ6Red]", 1, true) == 1, "ligne locale d'échec : " .. Locale(m));
	check(m.ouvertures == 0 and m.menuCache, "menu laissé fermé");
	check(string.find(table.concat(m.journal, "\n"), "minuteur du suivi actif", 1, true) ~= nil, "journal : minuteur actif");
	m.Tic(); m.Image();
	check(#m.chat == 3 and #m.locales == 1, "une seule fois");
	-- Le menu rouvert ne montre plus « envoi en cours ».
	m.menuCache = false;
	check(not m.popup.open, "popup d'envoi fermé");
end

-- 6. Sans minuteur (InGame remplacé) : les événements du jeu suivent encore, partie pas en pause.
do
	local m = NewJeu({ sansMinuteur = true });
	m.Clic();
	m.Confirmer();
	m.menuCache = true;
	m.etat = { "ok", "BGE-000001", "C:\\x.zip" };
	m.ingame.Events.GameCoreEventPublishComplete.Fire();
	check(#m.locales == 1 and string.find(Locale(m), "BGE-000001", 1, true), "résultat par les événements du jeu");
	check(string.find(table.concat(m.journal, "\n"), "minuteur absent", 1, true) ~= nil, "journal : minuteur absent");
end

-- 6 bis. Sans le bloc du suivi dans InGame : le menu, affiché, termine lui-même.
do
	local m = NewJeu({ sansSuivi = true });
	m.Clic();
	m.Confirmer();
	m.etat = { "ok", "BGE-000002", "C:\\x.zip" };
	m.Image();
	check(#m.locales == 1 and string.find(Locale(m), "BGE-000002", 1, true) and #m.chat == 3, "fin par le menu");
	check(string.find(table.concat(m.journal, "\n"), "fin par le menu", 1, true) ~= nil, "journal : fin par le menu");
	check(m.update == nil and not m.popup.open, "menu remis à zéro");
end

-- 7. Envoi refusé par le proxy (un autre en cours) : échec tout de suite.
do
	local m = NewJeu({ refus = true });
	m.Clic();
	m.Confirmer();
	check(string.find(Dernier(m), "could not be sent", 1, true) ~= nil, "refus : message d'échec");
	check(#m.locales == 1, "refus : ligne locale");
	check(m.update == nil and not m.minuteur.joue, "pas de suivi");
end

-- 8. Solo : ni pause, ni chat ; le rapport part quand même.
do
	local m = NewJeu({ multi = false });
	m.Clic();
	check(#m.chat == 0 and m.wantsPause == false, "solo : pas de chat");
	m.Confirmer();
	check(#m.envois == 1 and string.find(m.envois[1], "multijoueur=0", 1, true), "solo : envoi");
	m.etat = { "ok", "BGE-000003", "C:\\x.zip" };
	m.Tic();
	check(#m.chat == 0 and #m.locales == 1, "solo : ligne locale seule");
end

-- 9. Anglais, natives, bouton ou formulaire absents.
do
	local m = NewJeu({ lang = "en_US" });
	check(m.bouton.text == "Report a bug (BGE)", "texte anglais");
	local s = NewJeu({ sansNatives = true });
	check(s.bouton.hidden == true and s.bouton.callbacks.clic == nil, "sans natives : bouton caché");
	local f = NewJeu({ sansFormulaire = true });
	check(f.bouton.hidden == true and f.bouton.callbacks.clic == nil, "sans formulaire : bouton caché");
	local b = NewJeu({ sansBouton = true });
	check(string.find(table.concat(b.journal, "\n"), "bouton absent", 1, true) ~= nil, "sans bouton : journal");
end

if failures == 0 then
	print("rapport : tout est bon");
else
	print(failures .. " échec(s)");
	os.exit(1);
end
