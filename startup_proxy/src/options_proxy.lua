-- ===========================================================================
-- BetterGameDLLs : onglet de l'écran d'options, ajouté à la fin d'Options.lua par le
-- proxy de démarrage (startup_proxy/src/options.rs). Il édite rollover_proxy.cfg, le fichier de
-- réglages du proxy de GameCore, par deux fonctions natives posées par le proxy :
--   Civ6Proxy_LireReglages()          -> texte, installé (1|0), déjà lu (1|0) ; ou nil, erreur
--   Civ6Proxy_EcrireReglages(lignes)  -> 1, déjà lu (1|0) ; ou nil, erreur  (lignes « clé=valeur »)
-- « Déjà lu » : le proxy de GameCore est chargé et a lu le fichier, ce qu'il ne fait qu'à son
-- chargement, au lancement de la première partie. Alors seulement un changement attend le
-- redémarrage du jeu : la confirmation lève _PromptRestartApp, et le jeu affiche son propre message
-- « redémarrage nécessaire ». Sinon, la prochaine partie lira le nouveau fichier.
-- Textes en français si le jeu l'est, en anglais pour toutes les autres langues.
-- m_tabs, OnSelectTab, OnConfirm, OnShow, PopulateComboBox et _PromptRestartApp sont ceux
-- d'Options.lua.
-- ===========================================================================

local PROXY_FR = false;
do
	local language = Locale.GetCurrentLanguage and Locale.GetCurrentLanguage();
	local code = language and language.Type;
	PROXY_FR = type(code) == "string" and string.sub(string.lower(code), 1, 2) == "fr";
end

-- Le texte dans la langue du jeu : { français, anglais }.
local function Proxy_T(t)
	if PROXY_FR then return t[1]; end
	return t[2];
end

local PROXY_ON_OFF = {
	{ "on", { "Activé", "Enabled" } },
	{ "off", { "Désactivé", "Disabled" } },
};
local PROXY_CACHE = {
	{ "quiet", { "Activé", "Enabled" } },
	{ "off", { "Désactivé", "Disabled" } },
	{ "fast", { "Activé, avec journal", "Enabled, with log" } },
	{ "count", { "Mesure seulement (diagnostic)", "Measure only (diagnostic)" } },
	{ "verify", { "Vérification (diagnostic, plus lent)", "Verification (diagnostic, slower)" } },
};

-- Une entrée par réglage lu par le proxy de GameCore (read_config, proxy/src/lib.rs) ou par celui-ci
-- (verif_maj, src/maj.rs) : un test (cargo test, startup_proxy) vérifie que les listes concordent.
-- `defaut` est la valeur que prend le proxy quand la clé manque au fichier. `libre` : jamais grisé
-- par l'interrupteur général, qui ne concerne que le proxy de GameCore.
-- `avance` (sur un titre) : la section et ses réglages sont des options avancées, cachées par défaut
-- derrière le bouton « Afficher les options avancées », comme les options vidéo avancées du jeu.
-- Restent visibles le confort et l'interface.
local PROXY_REGLAGES = {
	{ titre = { "Général", "General" } },
	-- Interrupteur général : off, les autres réglages sont grisés (et gardés dans le fichier).
	{ cle = "actif", defaut = "on", general = true,
		nom = { "Proxys BetterGameDLLs", "BetterGameDLLs proxies" },
		aide = {
			"Désactive d'un coup tous les changements du proxy de GameCore, sans le désinstaller : le jeu se comporte comme sans proxy. Les autres réglages sont gardés, et cet onglet reste là pour le réactiver.",
			"Turns off all GameCore proxy changes at once, without uninstalling it: the game behaves as without the proxy. The other settings are kept, and this tab stays available to turn it back on.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "verif_maj", defaut = "on", libre = true,
		nom = { "Recherche des mises à jour", "Check for updates" },
		aide = {
			"Au lancement du jeu, cherche sur GitHub une nouvelle version de BetterGameDLLs, et la propose au menu principal. Prend effet au prochain lancement du jeu.",
			"When the game starts, looks on GitHub for a new version of BetterGameDLLs, and offers it in the main menu. Takes effect the next time the game starts.",
		},
		choix = PROXY_ON_OFF },

	{ titre = { "Performances du passage de tour", "Turn processing performance" }, avance = true },
	{ cle = "mode", defaut = "coalesce",
		nom = { "Regroupement des mises à jour de villes", "Batched city updates" },
		aide = {
			"Pendant une cascade de déclarations de guerre, les données de villes et de diplomatie ne sont rafraîchies qu'une fois, à la fin : plus de 2 s gagnées sur un gros passage de tour. Ne change rien à la partie : sans risque en multijoueur, même face à des joueurs sans proxy.",
			"During a cascade of war declarations, city and diplomacy data are refreshed only once, at the end: over 2 s saved on a heavy turn. Does not change the game: safe in multiplayer, even against players without the proxy.",
		},
		choix = {
			{ "coalesce", { "Activé", "Enabled" } },
			{ "off", { "Désactivé", "Disabled" } },
			{ "count", { "Mesure seulement (diagnostic)", "Measure only (diagnostic)" } },
		} },
	{ cle = "portee", defaut = "cascade",
		nom = { "Portée du regroupement", "Batching scope" },
		aide = {
			"« Chaque message du jeu » regroupe aussi les rafraîchissements hors guerre (gouverneurs, ressources, accords). L'interface peut alors montrer une donnée de ville en retard d'un instant.",
			"\"Every game message\" also batches refreshes outside of wars (governors, resources, deals). The interface may then show city data a moment late.",
		},
		choix = {
			{ "cascade", { "Cascades de guerre seulement", "War cascades only" } },
			{ "message", { "Chaque message du jeu", "Every game message" } },
		} },
	{ cle = "ameliorations", defaut = "off",
		nom = { "Cache des améliorations constructibles", "Buildable improvements cache" },
		aide = {
			"Accélère la question « ce joueur peut-il construire cette amélioration ? », posée des millions de fois par passage de tour par l'IA. Même résultat que le jeu, vérifié sans aucun écart.",
			"Speeds up the question \"can this player build this improvement?\", asked millions of times per turn by the AI. Same result as the game, verified with no difference.",
		},
		choix = PROXY_CACHE },
	{ cle = "rendements", defaut = "off",
		nom = { "Mémoire des rendements de cases pour l'IA", "Tile yield memory for the AI" },
		aide = {
			"Mémorise le rendement des cases pendant les calculs parallèles de l'IA, où la carte ne peut pas changer.",
			"Remembers tile yields during the AI's parallel computations, while the map cannot change.",
		},
		choix = PROXY_CACHE },

	{ titre = { "Interface", "Interface" } },
	{ cle = "wmd_accel", defaut = "20",
		nom = { "Accélération de l'animation des armes nucléaires", "Nuclear strike animation speed-up" },
		aide = {
			"Seulement si le combat rapide est activé (onglet Jeu). Accélère l'affichage pendant la frappe, sans effet sur la partie.",
			"Only with quick combat enabled (Game tab). Speeds up the display during the strike, with no effect on the game.",
		},
		choix = {
			{ "off", { "Désactivée", "Disabled" } },
			{ "5", { "x5", "x5" } },
			{ "10", { "x10", "x10" } },
			{ "20", { "x20", "x20" } },
			{ "40", { "x40", "x40" } },
		} },
	{ cle = "ui_boutons", defaut = "on",
		nom = { "Boutons d'attaque au-dessus des drapeaux d'unités", "Attack buttons above unit flags" },
		aide = {
			"Les boutons d'attaque des villes et des campements, et ceux de frappe nucléaire des silos, passent au-dessus des drapeaux d'unités, où ils restent cliquables.",
			"City and encampment attack buttons, and silo nuclear strike buttons, are drawn above unit flags, where they stay clickable.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_recentrage", defaut = "off",
		nom = { "Recentrer la caméra au clic sur ces boutons", "Recenter the camera when clicking these buttons" },
		aide = {
			"Comportement d'origine du jeu : la caméra se recentre sur la ville au clic sur un bouton d'attaque.",
			"Original game behavior: the camera recenters on the city when an attack button is clicked.",
		},
		choix = {
			{ "on", { "Oui", "Yes" } },
			{ "off", { "Non", "No" } },
		} },
	{ cle = "ui_epingles", defaut = "on",
		nom = { "Épingles de carte allégées", "Lighter map pins" },
		aide = {
			"Les épingles et leur liste ne sont plus reconstruites quand rien n'a changé : moins de ralentissements en multijoueur.",
			"Map pins and their list are no longer rebuilt when nothing changed: fewer slowdowns in multiplayer.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "tours_production", defaut = "on",
		nom = { "Compteur de tours de production corrigé", "Fixed production turn counter" },
		aide = {
			"Le nombre de tours affiché tient compte des bonus de gouvernement (fascisme, autocratie, république marchande), comme la production réelle. Affichage seulement.",
			"The displayed number of turns includes government bonuses (Fascism, Autocracy, Merchant Republic), like the actual production. Display only.",
		},
		choix = PROXY_ON_OFF },

	{ titre = { "Correctifs", "Fixes" } },
	{ cle = "ui_merveilles", defaut = "on",
		nom = { "Merveille gardée dans la file après un échange", "Wonder kept in the queue after a swap" },
		aide = {
			"Une merveille retirée de la file de production par un échange y est remise, par des commandes normales du jeu : pas de désynchronisation, même face à des joueurs sans proxy.",
			"A wonder removed from the production queue by a swap is put back, through normal game commands: no desync, even against players without the proxy.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_rendements", defaut = "on",
		nom = { "Rendements d'empire à jour après un gouverneur ou un personnage illustre", "Empire yields updated after a governor or great person" },
		aide = {
			"Le bandeau du haut se met à jour après un titre de gouverneur, un déplacement de gouverneur ou un personnage illustre, sans changer un citoyen. Affichage seulement : pas de désynchronisation.",
			"The top bar updates after a governor title, a governor move or a great person, without changing a citizen. Display only: no desync.",
		},
		choix = PROXY_ON_OFF },

	{ titre = { "Mods d'interface intégrés", "Built-in interface mods" } },
	{ cle = "ui_charges", defaut = "off",
		nom = { "Charges des bâtisseurs sur leur drapeau", "Builder charges on their flag" },
		aide = {
			"D'après Better Builder Charges Tracking (wltk). Le drapeau d'une unité qui a des charges (bâtisseur, ingénieur militaire, légion romaine…) les montre à la place des promotions. Remplace le mod s'il est actif.",
			"Based on Better Builder Charges Tracking (wltk). The flag of a unit with charges (builder, military engineer, Roman legion…) shows them instead of promotions. Replaces the mod if it is active.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_routes", defaut = "off",
		nom = { "Tours restants des routes commerciales", "Trade route turns remaining" },
		aide = {
			"Le drapeau de chacun de vos négociants en route montre le nombre de tours avant la fin de sa route commerciale. Le négociant la termine ensuite en rentrant à sa ville d'origine. Affichage seulement.",
			"The flag of each of your traders on a route shows the number of turns until its trade route ends. The trader then finishes it on returning to its origin city. Display only.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_notifications", defaut = "off",
		nom = { "Bouton d'effacement des notifications", "Clear notifications button" },
		aide = {
			"D'après NotificationClear (336tarot). Un bouton du panneau des notifications ferme d'un clic celles qu'on peut fermer soi-même. Remplace le mod s'il est actif.",
			"Based on NotificationClear (336tarot). A button in the notification panel dismisses in one click those you can dismiss yourself. Replaces the mod if it is active.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_bandeau", defaut = "off",
		nom = { "Bandeau du haut étendu", "Extended top panel" },
		aide = {
			"D'après TopPanel Extension Pro (号码菌 / Azukii). Nourriture, production, population et ressources de luxe dans le bandeau, avec le détail par ville ; ressources des coéquipiers. Remplace le mod s'il est actif. Sans le mod, le bandeau de Better Spectator Mod est gardé.",
			"Based on TopPanel Extension Pro (号码菌 / Azukii). Food, production, population and luxury resources in the top panel, with per-city details; teammates' resources. Replaces the mod if it is active. Without the mod, Better Spectator Mod's top panel is kept.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_politiques", defaut = "off",
		nom = { "Effet des cartes de politique", "Policy card effects" },
		aide = {
			"D'après Extended Policy Cards (Aristos). Chaque carte montre son gain chiffré pour votre empire, calculé par Real Modifier Analysis (Infixo), corrigé (Raj, Médina, Monachisme, Prestige civil, cartes d'entretien, nourriture des cartes de personnages illustres). Ne demande plus Better Report Screen. Remplace le mod s'il est actif.",
			"Based on Extended Policy Cards (Aristos). Each card shows its numeric gain for your empire, computed by Real Modifier Analysis (Infixo), fixed (Raj, Medina Quarter, Monasticism, Civil Prestige, maintenance cards, food on great person cards). No longer needs Better Report Screen. Replaces the mod if it is active.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_rapports", defaut = "off",
		nom = { "Écran des rapports amélioré", "Better report screen" },
		aide = {
			"D'après Better Report Screen (Infixo). Écran des rapports refait, avec les onglets Accords, Unités, Politiques, Cités-États et Villes, et l'effet de chaque politique et cité-État, corrigé comme les cartes. Remplace le mod s'il est actif.",
			"Based on Better Report Screen (Infixo). Reworked report screen, with Deals, Units, Policies, City-States and Cities tabs, and the effect of each policy and city-state, fixed like the cards. Replaces the mod if it is active.",
		},
		choix = PROXY_ON_OFF },

	{ titre = { "Diagnostic", "Diagnostics" }, avance = true },
	{ cle = "ui_journal", defaut = "off",
		nom = { "Journal des scripts et XML chargés", "Log loaded scripts and XML" },
		aide = {
			"Note chaque fichier d'interface lu dans le journal du proxy.",
			"Writes each interface file read to the proxy log.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_mesure_epingles", defaut = "off",
		nom = { "Mesure du coût des épingles", "Measure map pin cost" },
		aide = {
			"Écrit dans Lua.log la durée des reconstructions d'épingles et du bandeau des dirigeants.",
			"Writes to Lua.log how long map pin and leader ribbon rebuilds take.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "ui_diag_accords", defaut = "off",
		nom = { "Diagnostic des accords entre joueurs", "Diagnose deals between players" },
		aide = {
			"Écrit dans Lua.log les propositions d'accord reçues et envoyées, et ce que montre l'écran d'accord, pour comprendre les accords croisés entre humains.",
			"Writes to Lua.log the deal proposals sent and received, and what the deal screen shows, to understand crossed deals between humans.",
		},
		choix = PROXY_ON_OFF },
	{ cle = "depart_signaux", defaut = "off",
		nom = { "Afficher les signaux du départ simultané", "Show simultaneous turn start signals" },
		aide = {
			"Avec l'option de partie « No first moves (BGD) », les machines s'échangent des messages de chat cachés (.bgd_pret, .bgd_depart). Activé, ils restent visibles dans votre chat, pour suivre le départ de chaque tour. Affichage seulement, sur cette machine.",
			"With the \"No first moves (BGD)\" game option, machines exchange hidden chat messages (.bgd_pret, .bgd_depart). When on, they stay visible in your chat, to follow each turn start. Display only, on this machine.",
		},
		choix = PROXY_ON_OFF },
};

local PROXY_TEXTES = {
	avertissement_partie = {
		"Ces réglages sont ceux des proxys BetterGameDLLs (rollover_proxy.cfg). Ils prendront effet au lancement de la prochaine partie.",
		"These are the settings of the BetterGameDLLs proxies (rollover_proxy.cfg). They will take effect when the next game starts.",
	},
	avertissement_redemarrage = {
		"Ces réglages sont ceux des proxys BetterGameDLLs (rollover_proxy.cfg). Une partie a déjà été lancée : ils ne prendront effet qu'au prochain redémarrage du jeu.",
		"These are the settings of the BetterGameDLLs proxies (rollover_proxy.cfg). A game has already been started: they will only take effect after restarting the game.",
	},
	sans_natives = {
		"Fonctions du proxy de démarrage absentes : les réglages ne peuvent être ni lus ni enregistrés.",
		"Startup proxy functions missing: settings can be neither read nor saved.",
	},
	lecture_impossible = { "Lecture des réglages impossible : ", "Cannot read the settings: " },
	installe = { "Proxy de GameCore installé.", "GameCore proxy installed." },
	non_installe = {
		"Proxy de GameCore non installé : ces réglages n'auront d'effet qu'une fois installé.",
		"GameCore proxy not installed: these settings will only take effect once it is installed.",
	},
	non_enregistres = { "Réglages non enregistrés : ", "Settings not saved: " },
	valeur_du_fichier = { " (valeur du fichier)", " (value from file)" },
	afficher_avancees = { "Afficher les options avancées", "Show advanced options" },
	masquer_avancees = { "Masquer les options avancées", "Hide advanced options" },
};

-- Auteurs des mods repris dans le proxy (mods_ui/README.md) : affichés en bas de l'onglet, jamais
-- grisés. { mod, Workshop, auteurs }.
local PROXY_CREDITS = {
	{ "Better Report Screen, Real Modifier Analysis, ReportsList Loader", "1312585482",
		{ "Infixo ; onglets Unités et Accords d'origine : Greg Miller ; remerciements : nightfearer, Tyolan, copysoul, RohBang, Gorlod, Zergus, Rac, Aristos",
		  "Infixo; original Units and Deals tabs: Greg Miller; thanks: nightfearer, Tyolan, copysoul, RohBang, Gorlod, Zergus, Rac, Aristos" } },
	{ "Extended Policy Cards", "2266952591", { "Aristos", "Aristos" } },
	{ "TopPanel Extension Pro", "2997927787", { "号码菌 (Azukii) ; remerciements : JNNR", "号码菌 (Azukii); thanks: JNNR" } },
	{ "NotificationClear", "3063966792", { "336tarot", "336tarot" } },
	{ "Better Builder Charges Tracking", "2409116842", { "wltk ; remerciements : DeepLogic, 小小雨10101", "wltk; thanks: DeepLogic, 小小雨10101" } },
};

local function Proxy_CreditsText()
	local lines = { Proxy_T({
		"Les mods d'interface intégrés reprennent le travail de leurs auteurs, que BetterGameDLLs remercie :",
		"The built-in interface mods build on their authors' work, with BetterGameDLLs' thanks:",
	}) };
	for _, c in ipairs(PROXY_CREDITS) do
		table.insert(lines, "[ICON_Bullet] " .. c[1] .. " (Workshop " .. c[2] .. ") : " .. Proxy_T(c[3]) .. ".");
	end
	return table.concat(lines, "[NEWLINE]");
end

local m_proxySaved = {};     -- valeurs du fichier
local m_proxyPending = {};   -- choix pas encore confirmés
local m_proxyPullDowns = {};
local m_proxyAdvancedShown = false;  -- gardé d'une ouverture à l'autre, comme dans l'onglet Vidéo

-- Tailles des piles, de l'intérieur vers l'extérieur, puis du défilement.
local function Proxy_Resize()
	Controls.ProxyBasicStack:CalculateSize();
	Controls.ProxyAdvancedStack:CalculateSize();
	Controls.ProxyOptionsStack:CalculateSize();
	Controls.ProxyOptionsScrollPanel:CalculateInternalSize();
end

local function Proxy_ShowAdvanced(shown)
	m_proxyAdvancedShown = shown;
	Controls.ProxyAdvancedStack:SetHide(not shown);
	Controls.ProxyAdvancedToggle:SetSelected(shown);
	Controls.ProxyAdvancedToggle:SetText(Proxy_T(shown and PROXY_TEXTES.masquer_avancees or PROXY_TEXTES.afficher_avancees));
	Proxy_Resize();
end

local function Proxy_Current(r)
	local v = m_proxyPending[r.cle];
	if v == nil then v = m_proxySaved[r.cle]; end
	if v == nil then v = r.defaut; end
	return v;
end

-- Relit le fichier ; rend le texte d'état et l'avertissement à afficher.
local function Proxy_Read()
	m_proxySaved = {};
	if Civ6Proxy_LireReglages == nil then
		return Proxy_T(PROXY_TEXTES.sans_natives), Proxy_T(PROXY_TEXTES.avertissement_redemarrage);
	end
	local text, installed, loaded = Civ6Proxy_LireReglages();
	if text == nil then
		return Proxy_T(PROXY_TEXTES.lecture_impossible) .. tostring(installed), Proxy_T(PROXY_TEXTES.avertissement_redemarrage);
	end
	-- Comme read_config : la dernière ligne d'une clé l'emporte.
	for line in string.gmatch(text, "[^\r\n]+") do
		local k, v = string.match(line, "^%s*([%w_]+)%s*=%s*(.-)%s*$");
		if k ~= nil then m_proxySaved[k] = v; end
	end
	local warning = Proxy_T(loaded == 1 and PROXY_TEXTES.avertissement_redemarrage or PROXY_TEXTES.avertissement_partie);
	if installed == 1 then
		return Proxy_T(PROXY_TEXTES.installe), warning;
	end
	return Proxy_T(PROXY_TEXTES.non_installe), warning;
end

-- Remplit les listes avec les valeurs du fichier et les choix en attente. Interrupteur général
-- sur off : les autres listes sont grisées.
local function Proxy_Populate()
	local locked = false;
	for _, r in ipairs(PROXY_REGLAGES) do
		if r.general then locked = Proxy_Current(r) == "off"; end
	end
	for _, r in ipairs(PROXY_REGLAGES) do
		if r.cle ~= nil then
			local current = Proxy_Current(r);
			local values = {};
			local known = false;
			for _, c in ipairs(r.choix) do
				table.insert(values, { Proxy_T(c[2]), c[1] });
				if c[1] == current then known = true; end
			end
			-- Une valeur écrite à la main, hors de la liste : on la montre telle quelle.
			if not known then
				table.insert(values, { current .. Proxy_T(PROXY_TEXTES.valeur_du_fichier), current });
			end
			local saved = m_proxySaved[r.cle] or r.defaut;
			PopulateComboBox(m_proxyPullDowns[r.cle], values, current, function(value)
				if value == saved then
					m_proxyPending[r.cle] = nil;
				else
					m_proxyPending[r.cle] = value;
				end
				Controls.ConfirmButton:SetDisabled(false);
				if r.general then Proxy_Populate(); end
			end, locked and not (r.general or r.libre));
		end
	end
	Proxy_Resize();
end

local function Proxy_Refresh()
	local status, warning = Proxy_Read();
	Controls.ProxyStatus:SetText(status);
	Controls.ProxyWarning:SetText(warning);
	Proxy_Populate();
end

-- Enregistre les choix en attente ; vrai s'ils sont écrits et qu'ils attendent le redémarrage du
-- jeu (le proxy de GameCore a déjà lu le fichier).
local function Proxy_Save()
	local lines = {};
	for k, v in pairs(m_proxyPending) do
		table.insert(lines, k .. "=" .. v);
	end
	if #lines == 0 then return false; end
	if Civ6Proxy_EcrireReglages == nil then
		Controls.ProxyStatus:SetText(Proxy_T(PROXY_TEXTES.non_enregistres) .. Proxy_T(PROXY_TEXTES.sans_natives));
		return false;
	end
	local ok, loaded = Civ6Proxy_EcrireReglages(table.concat(lines, "\n"));
	if ok == nil then
		Controls.ProxyStatus:SetText(Proxy_T(PROXY_TEXTES.non_enregistres) .. tostring(loaded));
		return false;
	end
	for k, v in pairs(m_proxyPending) do
		m_proxySaved[k] = v;
	end
	m_proxyPending = {};
	return loaded == 1;
end

local function Proxy_Build()
	-- Chaque section va dans la pile visible ou dans celle des options avancées, selon son titre.
	local stack = Controls.ProxyBasicStack;
	for _, r in ipairs(PROXY_REGLAGES) do
		local inst = {};
		if r.titre ~= nil then
			stack = r.avance and Controls.ProxyAdvancedStack or Controls.ProxyBasicStack;
			ContextPtr:BuildInstanceForControl("KeyBindingCategory", inst, stack);
			inst.CategoryName:SetText(Proxy_T(r.titre));
		else
			ContextPtr:BuildInstanceForControl("PullDownParameterInstance", inst, stack);
			inst.StringName:SetText(Proxy_T(r.nom));
			inst.StringName:SetToolTipString(Proxy_T(r.aide));
			inst.PullDown:GetButton():SetToolTipString(Proxy_T(r.aide));
			m_proxyPullDowns[r.cle] = inst.PullDown;
		end
	end
	-- Crédits, à la fin : le label du panneau (options_proxy_panel.xml) passe après les réglages.
	if Controls.ProxyCredits ~= nil then
		local inst = {};
		ContextPtr:BuildInstanceForControl("KeyBindingCategory", inst, Controls.ProxyOptionsStack);
		inst.CategoryName:SetText(Proxy_T({ "Crédits", "Credits" }));
		Controls.ProxyCredits:ChangeParent(Controls.ProxyOptionsStack);
		Controls.ProxyCredits:SetText(Proxy_CreditsText());
		Controls.ProxyCredits:SetHide(false);
	end
	Controls.ProxyAdvancedToggle:RegisterCallback(Mouse.eLClick, function() Proxy_ShowAdvanced(not m_proxyAdvancedShown); end);
	Controls.ProxyAdvancedToggle:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
	Proxy_ShowAdvanced(false);
end

-- Si le XML n'a pas été modifié (Options.xml remplacé par un mod, par exemple), rien n'est ajouté.
if Controls.ProxyTab ~= nil and Controls.ProxyOptions ~= nil and Controls.ProxyAdvancedStack ~= nil and m_tabs ~= nil then
	-- 1 : pas de bouton « Restaurer les valeurs par défaut » sur cet onglet, comme pour les raccourcis.
	local tab = { Controls.ProxyTab, Controls.ProxyOptions, "BetterGameDLLs", 1 };
	table.insert(m_tabs, tab);
	Controls.ProxyTab:RegisterCallback(Mouse.eLClick, function() OnSelectTab(tab); end);
	Controls.ProxyTab:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
	Controls.ProxyTab:SetHide(false);
	Controls.TabStack:CalculateSize();
	Proxy_Build();
	Proxy_Refresh();

	-- À chaque ouverture : choix en attente abandonnés (Annuler ferme l'écran sans rien écrire),
	-- fichier relu (il a pu changer depuis l'autre écran d'options, menu principal ou partie), et
	-- avertissement mis à jour (une partie a pu être lancée entre-temps).
	ContextPtr:SetShowHandler(function()
		OnShow();
		m_proxyPending = {};
		Proxy_Refresh();
	end);

	-- Confirmer : le fichier d'abord, puis la confirmation du jeu, qui affiche le message de
	-- redémarrage si _PromptRestartApp est levé.
	local Proxy_BaseOnConfirm = OnConfirm;
	OnConfirm = function()
		if Proxy_Save() then
			_PromptRestartApp = true;
		end
		Proxy_BaseOnConfirm();
	end
	Controls.ConfirmButton:RegisterCallback(Mouse.eLClick, OnConfirm);

	print("BetterGameDLLs : onglet des proxys ajouté aux options");
end
