-- ===========================================================================
-- BetterGameDLLs : onglet de l'écran d'options, ajouté à la fin d'Options.lua par le
-- proxy de démarrage (startup_proxy/src/options.rs). Il édite rollover_proxy.cfg, le fichier de
-- réglages du proxy de GameCore, par deux fonctions natives posées par le proxy :
--   Civ6Proxy_LireReglages()          -> texte, installé (1|0), déjà lu (1|0) ; ou nil, erreur
--   Civ6Proxy_EcrireReglages(lignes)  -> 1, déjà lu (1|0) ; ou nil, erreur  (lignes « clé=valeur »)
--   Civ6Proxy_ReinitialiserReglages(cles) -> idem ; retire du fichier les lignes de ces clés (une
--                                        par ligne), qui reprennent leur valeur par défaut
-- « Déjà lu » : le proxy de GameCore est chargé et a lu le fichier, ce qu'il ne fait qu'à son
-- chargement, au lancement de la première partie. Alors un changement attend le redémarrage du
-- jeu : la confirmation lève _PromptRestartApp, et le jeu affiche son propre message « redémarrage
-- nécessaire ». Sinon, la prochaine partie lira le nouveau fichier. Les réglages `demarrage`, lus
-- par le proxy de démarrage au lancement du jeu, demandent toujours le redémarrage.
-- Textes dans la langue du jeu : BGD_Txt (textes/bgd_texte.lua, placé devant ce bloc). Le nom et
-- l'aide d'un réglage sont LOC_BGD_OPT_<CLÉ>_NOM et LOC_BGD_OPT_<CLÉ>_AIDE.
-- m_tabs, OnSelectTab, OnConfirm, OnShow, PopulateComboBox, _kPopupDialog et _PromptRestartApp
-- sont ceux d'Options.lua.
-- ===========================================================================

-- Réécrite avec la version de BetterGameDLLs par le proxy de démarrage (src/options.rs), affichée
-- dans le titre de l'écran quand l'onglet est ouvert : « BGE 0.5.0 » (le bouton de l'onglet garde « BetterGameExperience », sans version).
local PROXY_VERSION = nil;

local function Proxy_T(cle, ...)
	return BGD_Txt(cle, ...);
end

local function Proxy_Nom(r)
	return Proxy_T("LOC_BGD_OPT_" .. string.upper(r.cle) .. "_NOM");
end

local function Proxy_Aide(r)
	return Proxy_T("LOC_BGD_OPT_" .. string.upper(r.cle) .. "_AIDE");
end

local PROXY_ON_OFF = {
	{ "on", "LOC_BGD_OPT_CHOIX_ACTIVE" },
	{ "off", "LOC_BGD_OPT_CHOIX_DESACTIVE" },
};
local PROXY_CACHE = {
	{ "quiet", "LOC_BGD_OPT_CHOIX_ACTIVE" },
	{ "off", "LOC_BGD_OPT_CHOIX_DESACTIVE" },
	{ "fast", "LOC_BGD_OPT_CHOIX_JOURNAL" },
	{ "count", "LOC_BGD_OPT_CHOIX_MESURE" },
	{ "verify", "LOC_BGD_OPT_CHOIX_VERIFICATION" },
};

-- Une entrée par réglage lu par le proxy de GameCore (read_config, proxy/src/lib.rs) ou par celui-ci
-- (verif_maj, src/maj.rs) : un test (cargo test, startup_proxy) vérifie que les listes concordent.
-- `defaut` est la valeur que prend le proxy quand la clé manque au fichier. `libre` : jamais grisé
-- par l'interrupteur général, qui ne concerne que le proxy de GameCore. `demarrage` : lu par le
-- proxy de démarrage au lancement du jeu, son changement demande donc toujours le redémarrage.
-- `avance` (sur un titre) : la section et ses réglages sont des options avancées, cachées par défaut
-- derrière le bouton « Afficher les options avancées », comme les options vidéo avancées du jeu.
-- Restent visibles le confort et l'interface. `diag` : outil d'analyse, seulement dans les builds
-- locaux (feature `diag` des proxys) ; la release ne le compile pas et l'onglet ne le montre pas.
local PROXY_REGLAGES = {
	{ titre = "LOC_BGD_OPT_TITRE_GENERAL" },
	-- Interrupteur général : off, les autres réglages sont grisés (et gardés dans le fichier).
	{ cle = "actif", defaut = "on", general = true,
		choix = PROXY_ON_OFF },
	{ cle = "verif_maj", defaut = "on", libre = true, demarrage = true,
		choix = PROXY_ON_OFF },

	{ titre = "LOC_BGD_OPT_TITRE_PERFORMANCES", avance = true },
	{ cle = "mode", defaut = "coalesce",
		choix = {
			{ "coalesce", "LOC_BGD_OPT_CHOIX_ACTIVE" },
			{ "off", "LOC_BGD_OPT_CHOIX_DESACTIVE" },
			{ "count", "LOC_BGD_OPT_CHOIX_MESURE" },
		} },
	{ cle = "portee", defaut = "message",
		choix = {
			{ "cascade", "LOC_BGD_OPT_CHOIX_CASCADES" },
			{ "message", "LOC_BGD_OPT_CHOIX_MESSAGES" },
		} },
	{ cle = "ameliorations", defaut = "quiet",
		choix = PROXY_CACHE },
	{ cle = "rendements", defaut = "quiet",
		choix = PROXY_CACHE },

	{ titre = "LOC_BGD_OPT_TITRE_INTERFACE" },
	{ cle = "wmd_accel", defaut = "20",
		choix = {
			{ "off", "LOC_BGD_OPT_CHOIX_DESACTIVE" },
			{ "5", "x5" },
			{ "10", "x10" },
			{ "20", "x20" },
			{ "40", "x40" },
		} },
	{ cle = "ui_boutons", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_recentrage", defaut = "off",
		choix = {
			{ "on", "LOC_BGD_OPT_CHOIX_OUI" },
			{ "off", "LOC_BGD_OPT_CHOIX_NON" },
		} },
	{ cle = "ui_epingles", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_carte_entiere", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "tours_production", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_banniere_annuler", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_reduction_quartiers", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_debordement", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_avancement", defaut = "on",
		choix = PROXY_ON_OFF },

	{ titre = "LOC_BGD_OPT_TITRE_CORRECTIFS" },
	{ cle = "ui_merveilles", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_rendements", defaut = "on",
		choix = PROXY_ON_OFF },

	{ titre = "LOC_BGD_OPT_TITRE_MODS" },
	{ cle = "ui_charges", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_routes", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_notifications", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_bandeau", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_politiques", defaut = "on",
		choix = PROXY_ON_OFF },
	{ cle = "ui_rapports", defaut = "on",
		choix = PROXY_ON_OFF },

	{ titre = "LOC_BGD_OPT_TITRE_DIAGNOSTIC", avance = true },
	{ cle = "ui_journal", defaut = "off", diag = true,
		choix = PROXY_ON_OFF },
	{ cle = "ui_mesure_epingles", defaut = "off", diag = true,
		choix = PROXY_ON_OFF },
	{ cle = "ui_diag_accords", defaut = "off", diag = true,
		choix = PROXY_ON_OFF },
	{ cle = "ui_mesure_mouvements", defaut = "off",
		choix = PROXY_ON_OFF },
	{ cle = "depart_signaux", defaut = "off",
		choix = PROXY_ON_OFF },
	{ cle = "profil_zones", defaut = "off", libre = true, demarrage = true, diag = true,
		choix = PROXY_ON_OFF },
};

-- Toutes les clés, réglages de diagnostic compris : la réinitialisation les retire toutes du fichier.
local PROXY_CLES = {};
for _, r in ipairs(PROXY_REGLAGES) do
	if r.cle ~= nil then table.insert(PROXY_CLES, r.cle); end
end

-- Réécrite en `true` par le proxy de démarrage compilé avec la feature `diag` (src/options.rs).
local PROXY_DIAG = false;
if not PROXY_DIAG then
	for i = #PROXY_REGLAGES, 1, -1 do
		if PROXY_REGLAGES[i].diag then table.remove(PROXY_REGLAGES, i); end
	end
end

-- Auteurs des mods repris dans le proxy (mods_ui/README.md) : affichés en bas de l'onglet, jamais
-- grisés. { mod, Workshop, auteurs }.
local function Proxy_Credits()
	return {
		{ "Better Report Screen, Real Modifier Analysis, ReportsList Loader", "1312585482",
			Proxy_T("LOC_BGD_OPT_CREDITS_AUTEURS_BRS", "Infixo", "Greg Miller", "nightfearer, Tyolan, copysoul, RohBang, Gorlod, Zergus, Rac, Aristos") },
		{ "Extended Policy Cards", "2266952591", "Aristos" },
		{ "TopPanel Extension Pro", "2997927787", Proxy_T("LOC_BGD_OPT_CREDITS_AUTEURS_MERCI", "号码菌 (Azukii)", "JNNR") },
		{ "NotificationClear", "3063966792", "336tarot" },
		{ "Better Builder Charges Tracking", "2409116842", Proxy_T("LOC_BGD_OPT_CREDITS_AUTEURS_MERCI", "wltk", "DeepLogic, 小小雨10101") },
	};
end

local function Proxy_CreditsText()
	local lines = { Proxy_T("LOC_BGD_OPT_CREDITS_INTRO") };
	for _, c in ipairs(Proxy_Credits()) do
		table.insert(lines, "[ICON_Bullet] " .. Proxy_T("LOC_BGD_OPT_CREDITS_LIGNE", c[1], c[2], c[3]));
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
	Controls.ProxyAdvancedToggle:SetText(Proxy_T(shown and "LOC_BGD_OPT_MASQUER_AVANCEES" or "LOC_BGD_OPT_AFFICHER_AVANCEES"));
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
		return Proxy_T("LOC_BGD_OPT_SANS_NATIVES"), Proxy_T("LOC_BGD_OPT_AVERT_REDEMARRAGE");
	end
	local text, installed, loaded = Civ6Proxy_LireReglages();
	if text == nil then
		return Proxy_T("LOC_BGD_OPT_LECTURE_IMPOSSIBLE", tostring(installed)), Proxy_T("LOC_BGD_OPT_AVERT_REDEMARRAGE");
	end
	-- Comme read_config : la dernière ligne d'une clé l'emporte.
	for line in string.gmatch(text, "[^\r\n]+") do
		local k, v = string.match(line, "^%s*([%w_]+)%s*=%s*(.-)%s*$");
		if k ~= nil then m_proxySaved[k] = v; end
	end
	local warning = Proxy_T(loaded == 1 and "LOC_BGD_OPT_AVERT_REDEMARRAGE" or "LOC_BGD_OPT_AVERT_PARTIE");
	if installed == 1 then
		return Proxy_T("LOC_BGD_OPT_INSTALLE"), warning;
	end
	return Proxy_T("LOC_BGD_OPT_NON_INSTALLE"), warning;
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
				table.insert(values, { Proxy_T("LOC_BGD_OPT_VALEUR_FICHIER", current), current });
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
-- jeu : le proxy de GameCore a déjà lu le fichier, ou l'un d'eux n'est lu qu'au lancement du jeu.
local function Proxy_Save()
	local lines = {};
	local demarrage = false;
	for k, v in pairs(m_proxyPending) do
		table.insert(lines, k .. "=" .. v);
		for _, r in ipairs(PROXY_REGLAGES) do
			if r.cle == k and r.demarrage then demarrage = true; end
		end
	end
	if #lines == 0 then return false; end
	if Civ6Proxy_EcrireReglages == nil then
		Controls.ProxyStatus:SetText(Proxy_T("LOC_BGD_OPT_NON_ENREGISTRES", Proxy_T("LOC_BGD_OPT_SANS_NATIVES")));
		return false;
	end
	local ok, loaded = Civ6Proxy_EcrireReglages(table.concat(lines, "\n"));
	if ok == nil then
		Controls.ProxyStatus:SetText(Proxy_T("LOC_BGD_OPT_NON_ENREGISTRES", tostring(loaded)));
		return false;
	end
	for k, v in pairs(m_proxyPending) do
		m_proxySaved[k] = v;
	end
	m_proxyPending = {};
	return loaded == 1 or demarrage;
end

-- Réinitialisation, après confirmation : les clés de l'onglet sont retirées du fichier, et les
-- choix en attente abandonnés. Le message « redémarrage nécessaire » du jeu suit si un réglage
-- change et que le proxy de GameCore a déjà lu le fichier, ou qu'il n'est lu qu'au lancement du jeu.
local function Proxy_Reset()
	local function Rendre()
		Controls.ProxyResetButton:SetDisabled(false);
		Controls.WindowCloseButton:SetDisabled(false);
	end
	local function Reinitialiser()
		Rendre();
		local change, demarrage = false, false;
		for _, r in ipairs(PROXY_REGLAGES) do
			local v = m_proxySaved[r.cle];
			if r.cle ~= nil and v ~= nil and v ~= r.defaut then
				change = true;
				if r.demarrage then demarrage = true; end
			end
		end
		if Civ6Proxy_ReinitialiserReglages == nil then
			Controls.ProxyStatus:SetText(Proxy_T("LOC_BGD_OPT_NON_ENREGISTRES", Proxy_T("LOC_BGD_OPT_SANS_NATIVES")));
			return;
		end
		local ok, loaded = Civ6Proxy_ReinitialiserReglages(table.concat(PROXY_CLES, "\n"));
		if ok == nil then
			Controls.ProxyStatus:SetText(Proxy_T("LOC_BGD_OPT_NON_ENREGISTRES", tostring(loaded)));
			return;
		end
		m_proxyPending = {};
		Proxy_Refresh();
		Controls.ProxyStatus:SetText(Proxy_T("LOC_BGD_OPT_REINIT_FAIT"));
		if change and (loaded == 1 or demarrage) then
			_kPopupDialog:AddText(Locale.Lookup("LOC_OPTIONS_CHANGES_REQUIRE_APP_RESTART"));
			_kPopupDialog:AddButton(Locale.Lookup("LOC_OPTIONS_RESET_OPTIONS_POPUP_OK"), function() end);
			_kPopupDialog:Open();
		end
	end
	_kPopupDialog:AddText(Proxy_T("LOC_BGD_OPT_REINIT_TEXTE"));
	_kPopupDialog:AddButton(Locale.Lookup("LOC_OPTIONS_RESET_OPTIONS_POPUP_YES"), Reinitialiser, nil, nil, "PopupButtonInstanceRed");
	_kPopupDialog:AddButton(Locale.Lookup("LOC_OPTIONS_RESET_OPTIONS_POPUP_NO"), Rendre);
	_kPopupDialog:Open();
	Controls.ProxyResetButton:SetDisabled(true);
	Controls.WindowCloseButton:SetDisabled(true);
end

-- Sur l'onglet BGE, « Réinitialiser BGE » à gauche et Confirmer à droite, côte à côte sous les
-- panneaux ; sur les autres, Confirmer seul et centré, comme dans le jeu.
local function Proxy_PlaceButtons(onTab)
	if Controls.ProxyResetButton == nil then return; end
	Controls.ProxyResetButton:SetHide(not onTab);
	Controls.ProxyResetButton:SetOffsetX(onTab and -155 or 0);
	Controls.ConfirmButton:SetOffsetX(onTab and 155 or 0);
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
			inst.StringName:SetText(Proxy_Nom(r));
			inst.StringName:SetToolTipString(Proxy_Aide(r));
			inst.PullDown:GetButton():SetToolTipString(Proxy_Aide(r));
			m_proxyPullDowns[r.cle] = inst.PullDown;
		end
	end
	-- Crédits, à la fin : le label du panneau (options_proxy_panel.xml) passe après les réglages.
	if Controls.ProxyCredits ~= nil then
		local inst = {};
		ContextPtr:BuildInstanceForControl("KeyBindingCategory", inst, Controls.ProxyOptionsStack);
		inst.CategoryName:SetText(Proxy_T("LOC_BGD_OPT_TITRE_CREDITS"));
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
	local tab = { Controls.ProxyTab, Controls.ProxyOptions, PROXY_VERSION and ("BGE " .. PROXY_VERSION) or "BGE", 1 };
	table.insert(m_tabs, tab);
	Controls.ProxyTab:RegisterCallback(Mouse.eLClick, function() OnSelectTab(tab); end);
	Controls.ProxyTab:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
	Controls.ProxyTab:SetHide(false);
	Controls.TabStack:CalculateSize();
	Proxy_Build();
	Proxy_Refresh();

	-- Bouton de réinitialisation (ajouté au XML après Confirmer) : visible sur cet onglet seul.
	-- Les onglets du jeu appellent OnSelectTab par son nom global, au clic et à l'ouverture.
	if Controls.ProxyResetButton ~= nil then
		Controls.ProxyResetButton:SetText(Proxy_T("LOC_BGD_OPT_REINIT_BOUTON"));
		Controls.ProxyResetButton:SetToolTipString(Proxy_T("LOC_BGD_OPT_REINIT_AIDE"));
		Controls.ProxyResetButton:RegisterCallback(Mouse.eLClick, Proxy_Reset);
		Controls.ProxyResetButton:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
		local Proxy_BaseOnSelectTab = OnSelectTab;
		OnSelectTab = function(t)
			Proxy_BaseOnSelectTab(t);
			if type(t) == "number" then t = m_tabs[t]; end
			Proxy_PlaceButtons(t == tab);
		end
		Proxy_PlaceButtons(not Controls.ProxyOptions:IsHidden());
	end

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

	print("BetterGameExperience : onglet des proxys ajouté aux options");
end
