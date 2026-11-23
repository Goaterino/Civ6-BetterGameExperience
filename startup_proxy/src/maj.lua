-- BetterGameDLLs : notification de mise à jour au menu principal (voir startup_proxy/src/maj.rs).
-- Ajouté à la fin de MainMenu.lua. Natives posées par le proxy : Civ6Proxy_MiseAJour([action]) et
-- Civ6Proxy_Journal(texte). Le bloc enveloppe le gestionnaire d'affichage du menu, et le temps de la
-- vérification, sa mise à jour par image ; une erreur du bloc va au journal, jamais au menu.
-- Popup : celui du menu (g_PopupDialog) ; sinon (MainMenu de MPH), le bloc crée le sien sur l'instance
-- PopupDialog que le proxy ajoute au XML du menu.

-- Textes dans la langue du jeu : BGD_Txt (textes/bgd_texte.lua, placé devant ce bloc). La native
-- dit si la version proposée est une release ou une pré-release (canal choisi dans l'installeur,
-- case « Pré-release »).

-- Indicateur de version, comme celui de MPH : Initialize() du menu (jeu ou MPH) vient d'écrire la
-- version du jeu dans VersionLabel, en bas à gauche (MPH y ajoute « - MPH / <version> ») ; on ajoute
-- la nôtre à la suite. Version réécrite par le proxy (maj.rs) ; sans elle, rien.
local BGD_VERSION = nil;

-- Version du jeu non prise en charge (Civ6Proxy_VersionNonPriseEnCharge) : sa version, et si
-- l'avertissement a déjà été montré dans ce lancement ; nil si elle est prise en charge.
local function BGD_VersionNonPriseEnCharge()
	if Civ6Proxy_VersionNonPriseEnCharge == nil then return nil, true; end
	local version, vu = Civ6Proxy_VersionNonPriseEnCharge();
	return version, vu == 1;
end

local function BGD_Version_Afficher()
	local label = Controls and Controls.VersionLabel;
	if BGD_VERSION == nil or label == nil then return; end
	local texte = tostring(label:GetText() or "");
	local ajout = "[COLOR_LIGHTBLUE]BGE / " .. BGD_VERSION .. "[ENDCOLOR]";
	local inconnue = BGD_VersionNonPriseEnCharge();
	if inconnue ~= nil then
		ajout = "[COLOR_RED]" .. BGD_Txt("LOC_BGD_VERSION_NON_SUPPORTEE_COURT", inconnue) .. "[ENDCOLOR]";
	end
	label:SetText(texte == "" and ajout or (texte .. " - " .. ajout));
end

-- Avertissement d'une version non prise en charge, une fois par lancement du jeu, avant la
-- notification de mise à jour.
local bgd_avertir = false;
do
	local inconnue, vu = BGD_VersionNonPriseEnCharge();
	bgd_avertir = inconnue ~= nil and not vu;
end

-- Plus rien à faire : pas de proxy, vérification finie, ou notification déjà montrée.
local bgd_majFinie = Civ6Proxy_MiseAJour == nil;

local function BGD_Maj_Journal(texte)
	if Civ6Proxy_Journal then pcall(Civ6Proxy_Journal, "maj : " .. texte); end
end

local bgd_majPopup = nil;

-- Le popup du menu, ou le nôtre.
local function BGD_Maj_Popup()
	if g_PopupDialog ~= nil then return g_PopupDialog; end
	if bgd_majPopup == nil then
		if PopupDialog == nil then include("PopupDialog"); end
		if Controls.PopupRoot == nil then
			error("le menu n'a pas de popup (instance PopupDialog absente du XML)");
		end
		bgd_majPopup = PopupDialog:new("BGD_MajPopup");
		BGD_Maj_Journal("popup créé (le menu n'en a pas)");
	end
	return bgd_majPopup;
end

local function BGD_Maj_Installer()
	if Civ6Proxy_MiseAJour("installer") == 1 then
		BGD_Maj_Journal("fermeture du jeu pour l'installeur");
		-- Comme « Quitter vers le bureau » du menu (FrontEndPopup.lua, ExitOK) : UI.ExitGame() ne
		-- ferme le jeu qu'en partie.
		local friends = Network and Network.GetFriends and Network.GetFriends();
		if friends ~= nil then friends:ClearRichPresence(); end
		Events.UserConfirmedClose();
	else
		BGD_Maj_Popup():ShowOkDialog(BGD_Txt("LOC_BGD_MAJ_PAGE"));
	end
end

local function BGD_Maj_Avertir()
	local popup = BGD_Maj_Popup();
	if popup:IsOpen() then return; end
	local inconnue = BGD_VersionNonPriseEnCharge();
	bgd_avertir = false;
	if inconnue == nil then return; end
	pcall(Civ6Proxy_VersionNonPriseEnCharge, "vu");
	BGD_Maj_Journal("avertissement : jeu " .. tostring(inconnue) .. " non pris en charge");
	popup:Reset();
	popup:AddTitle(BGD_Txt("LOC_BGD_VERSION_NON_SUPPORTEE_TITRE"));
	popup:AddText(BGD_Txt("LOC_BGD_VERSION_NON_SUPPORTEE_TEXTE", inconnue));
	popup:AddButton(BGD_Txt("LOC_BGD_RAPPORT_FERMER"), function() end, PopupDialog.COMMAND_CANCEL);
	popup:Open();
end

local function BGD_Maj_Verifier()
	-- Pas par-dessus un autre message du menu (défis, erreur de lancement) : on attendra sa fermeture.
	if g_PopupDialog ~= nil and g_PopupDialog:IsOpen() then return; end
	if bgd_avertir then
		BGD_Maj_Avertir();
		return;
	end
	if bgd_majFinie then return; end
	local latest, current, kind = Civ6Proxy_MiseAJour();
	if latest == "" then return; end
	bgd_majFinie = true;
	if latest == nil then return; end
	local popup = BGD_Maj_Popup();
	local pre = kind == "prerelease";
	BGD_Maj_Journal("notification de la " .. (pre and "pré-release " or "release ") .. latest);
	popup:Reset();
	popup:AddTitle(BGD_Txt(pre and "LOC_BGD_MAJ_TITRE_PRERELEASE" or "LOC_BGD_MAJ_TITRE_RELEASE"));
	popup:AddText(BGD_Txt(pre and "LOC_BGD_MAJ_TEXTE_PRERELEASE" or "LOC_BGD_MAJ_TEXTE_RELEASE", latest, current));
	popup:AddButton(BGD_Txt("LOC_BGD_MAJ_BOUTON_MAJ"), BGD_Maj_Installer, PopupDialog.COMMAND_CONFIRM);
	popup:AddButton(BGD_Txt("LOC_BGD_MAJ_BOUTON_PLUS_TARD"), function() Civ6Proxy_MiseAJour("plus_tard"); end, PopupDialog.COMMAND_CANCEL);
	popup:Open();
end

local function BGD_Maj_Proteger(f, ...)
	local ok, err = pcall(f, ...);
	if not ok then
		bgd_majFinie = true;
		BGD_Maj_Journal("erreur : " .. tostring(err));
	end
end

do
	local ok, err = pcall(BGD_Version_Afficher);
	if not ok then BGD_Maj_Journal("indicateur de version : " .. tostring(err)); end
end

if (not bgd_majFinie or bgd_avertir) and ContextPtr ~= nil then
	local onShow = OnShow;
	local onUpdate = OnUpdate;
	ContextPtr:SetShowHandler(function(...)
		if onShow then onShow(...); end
		-- OnShow vient de poser OnUpdate : on l'enveloppe tant que la notification reste à montrer.
		if not bgd_majFinie or bgd_avertir then
			ContextPtr:SetUpdate(function(dt)
				if onUpdate then onUpdate(dt); end
				BGD_Maj_Proteger(BGD_Maj_Verifier);
			end);
		end
	end);
	BGD_Maj_Journal("bloc chargé");
end
