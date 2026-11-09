-- ===========================================================================
-- BetterGameExperience : bouton « Signaler un bug (BGE) » du menu Échap (voir
-- startup_proxy/src/rapport.rs). Ajouté à la fin d'InGameTopOptionsMenu.lua, que le remplaçant de
-- MPH inclut. Le proxy a recopié le bouton Options du XML juste sous lui (BGD_RapportButton).
-- Natives posées par le proxy :
--   Civ6Proxy_RapportEnvoyer(lignes) -> 1, ou nil et la raison ; lignes « clé=valeur » pour meta.json
--   Civ6Proxy_RapportEtat()          -> "" tant que l'envoi tourne ; puis une fois : "ok", numéro,
--                                       zip ; ou "echec", raison, zip
--   Civ6Proxy_Journal(texte)         -> écrit dans RolloverProxy.log
--
-- En multijoueur, le clic met la partie en pause si personne ne l'a déjà fait (SetWantsPause, comme
-- MPH), et prévient tout le monde dans le chat. Chaque étape a son message : formulaire ouvert,
-- rapport annulé, envoi en cours, envoi terminé (ou en échec). Les messages partent en anglais, et
-- chaque machine qui a BetterGameExperience les affiche dans sa langue (BGD_TxtChat). La pause
-- n'est pas levée par le bloc : les messages disent quand elle peut l'être.
-- Le formulaire et le suivi de l'envoi passent par un popup du jeu (PopupDialog) ; l'envoi tourne
-- dans un thread du proxy, que le bloc interroge à chaque image tant que le menu est affiché, et à
-- chaque série d'événements du jeu sinon.
-- ===========================================================================
do
local BGD_RapportOk, BGD_RapportErr = pcall(function()
	if BGD_Rapport ~= nil then return; end

	local function Journal(texte)
		if Civ6Proxy_Journal ~= nil then pcall(Civ6Proxy_Journal, "rapport : " .. texte); end
	end

	local bouton = Controls.BGD_RapportButton;
	if Civ6Proxy_RapportEnvoyer == nil or Civ6Proxy_RapportEtat == nil then
		Journal("fonctions natives absentes, pas de bouton");
		if bouton ~= nil then bouton:SetHide(true); end
		return;
	end
	if bouton == nil then
		Journal("bouton absent du XML (menu remplacé par un mod ?), pas de rapport");
		return;
	end

	-- etat : nil (rien), "formulaire" (popup de confirmation ouvert), "envoi" (thread en cours).
	local R = { etat = nil, popup = nil };
	BGD_Rapport = R;

	local function EnLigne()
		return GameConfiguration.IsNetworkMultiplayer() and not GameConfiguration.IsPlayByCloud();
	end

	local function NomLocal()
		local cfg = PlayerConfigurations[Network.GetLocalPlayerID()];
		if cfg == nil then return "?"; end
		return Locale.Lookup(cfg:GetPlayerName());
	end

	local function Chat(cle, couleur)
		if not EnLigne() then return; end
		local texte = "[" .. couleur .. "]BGE - " .. BGD_TxtAnglais(cle, NomLocal()) .. "[ENDCOLOR]";
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		Journal("message : " .. texte);
	end

	-- Pause demandée comme MPH (DropControl.lua) et la touche de pause du jeu : le joueur local
	-- veut la pause, et le dit aux autres. Rien si la partie est déjà en pause.
	local function Pauser()
		if not EnLigne() then return; end
		if GameConfiguration.IsPaused() then
			Journal("partie déjà en pause");
			return;
		end
		local cfg = PlayerConfigurations[Network.GetLocalPlayerID()];
		if cfg == nil then return; end
		cfg:SetWantsPause(true);
		Network.BroadcastPlayerInfo();
		Journal("pause demandée");
	end

	-- Sauvegarde de l'instant du rapport, comme la sauvegarde rapide du menu : le proxy l'attend
	-- (BGE_rapport.Civ6Save) avant de faire le zip, et s'en passe si elle n'arrive pas.
	local function Sauvegarder()
		local ok, err = pcall(function()
			local fichier = {};
			fichier.Name = "BGE_rapport";
			fichier.Location = SaveLocations.LOCAL_STORAGE;
			fichier.Type = Network.GetGameConfigSaveType();
			fichier.IsAutosave = false;
			fichier.IsQuicksave = false;
			Network.SaveGame(fichier);
		end);
		if not ok then Journal("sauvegarde impossible : " .. tostring(err)); end
	end

	-- Ce que le bloc sait de la partie, pour meta.json : une ligne « clé=valeur » par champ.
	local function Meta()
		local champs = {};
		local function Ajouter(cle, f)
			local ok, v = pcall(f);
			if ok and v ~= nil then
				table.insert(champs, cle .. "=" .. string.gsub(tostring(v), "[\r\n]", " "));
			end
		end
		local id = Network.GetLocalPlayerID();
		local cfg = PlayerConfigurations[id];
		Ajouter("tour", function() return Game.GetCurrentGameTurn(); end);
		Ajouter("joueur", NomLocal);
		Ajouter("joueur_id", function() return id; end);
		Ajouter("civilisation", function() return Locale.Lookup(cfg:GetCivilizationShortDescription()); end);
		Ajouter("dirigeant", function() return cfg:GetLeaderTypeName(); end);
		Ajouter("multijoueur", function() return EnLigne() and 1 or 0; end);
		Ajouter("hote", function() return Network.IsGameHost() and 1 or 0; end);
		Ajouter("en_pause", function() return GameConfiguration.IsPaused() and 1 or 0; end);
		Ajouter("humains", function() return #GameConfiguration.GetMultiplayerPlayerIDs(); end);
		Ajouter("langue", function() return Locale.GetCurrentLanguage().Type; end);
		return table.concat(champs, "\n");
	end

	local function Popup()
		if R.popup == nil then
			if PopupDialog == nil then include("PopupDialog"); end
			if Controls.PopupRoot == nil then
				error("le menu n'a pas de popup (instance PopupDialog absente du XML)");
			end
			R.popup = PopupDialog:new("BGD_RapportPopup");
		end
		return R.popup;
	end

	local function Afficher(titre, texte, boutons)
		local p = Popup();
		p:Reset();
		p:AddTitle(titre);
		p:AddText(texte);
		for _, b in ipairs(boutons) do
			p:AddButton(b[1], b[2], b[3]);
		end
		p:Open();
	end

	local function Protege(f)
		return function(...)
			local ok, err = pcall(f, ...);
			if not ok then Journal("erreur : " .. tostring(err)); end
		end
	end

	local Suivre;

	local function ArreterSuivi()
		if not pcall(function() ContextPtr:ClearUpdate(); end) then
			pcall(function() ContextPtr:SetUpdate(function() end); end);
		end
	end

	local function Fin(statut, texte, archive)
		R.etat = nil;
		ArreterSuivi();
		local fermer = { BGD_Txt("LOC_BGD_RAPPORT_FERMER"), function() end, PopupDialog.COMMAND_CONFIRM };
		if statut == "ok" then
			Journal("rapport envoyé, numéro " .. tostring(texte));
			Chat("LOC_BGD_CHAT_RAPPORT_FIN", "COLOR_GREEN");
			Afficher(BGD_Txt("LOC_BGD_RAPPORT_OK_TITRE"), BGD_Txt("LOC_BGD_RAPPORT_OK_TEXTE", texte), { fermer });
		else
			Journal("rapport non envoyé : " .. tostring(texte) .. ", archive : " .. tostring(archive));
			Chat("LOC_BGD_CHAT_RAPPORT_ECHEC", "COLOR_RED");
			Afficher(BGD_Txt("LOC_BGD_RAPPORT_ECHEC_TITRE"), BGD_Txt("LOC_BGD_RAPPORT_ECHEC_TEXTE", archive), { fermer });
		end
	end

	Suivre = Protege(function()
		if R.etat ~= "envoi" then return; end
		local statut, texte, archive = Civ6Proxy_RapportEtat();
		if statut == nil or statut == "" then return; end
		Fin(statut, texte, archive);
	end);

	local function AfficherEnvoi()
		Afficher(BGD_Txt("LOC_BGD_RAPPORT_TITRE"), BGD_Txt("LOC_BGD_RAPPORT_EN_COURS"),
			{ { BGD_Txt("LOC_BGD_RAPPORT_FERMER"), function() end, PopupDialog.COMMAND_CANCEL } });
	end

	local function Confirmer()
		if R.etat ~= "formulaire" then return; end
		R.etat = "envoi";
		Chat("LOC_BGD_CHAT_RAPPORT_ENVOI", "COLOR_GREEN");
		Sauvegarder();
		local ok, err = Civ6Proxy_RapportEnvoyer(Meta());
		if ok ~= 1 then
			Fin("echec", err, "");
			return;
		end
		AfficherEnvoi();
		ContextPtr:SetUpdate(Suivre);
	end

	local function Annuler()
		if R.etat ~= "formulaire" then return; end
		R.etat = nil;
		Journal("rapport annulé");
		Chat("LOC_BGD_CHAT_RAPPORT_ANNULE", "COLOR_GREEN");
	end

	local function Formulaire()
		Afficher(BGD_Txt("LOC_BGD_RAPPORT_TITRE"), BGD_Txt("LOC_BGD_RAPPORT_TEXTE"), {
			{ BGD_Txt("LOC_BGD_RAPPORT_CONFIRMER"), Protege(Confirmer), PopupDialog.COMMAND_CONFIRM },
			{ BGD_Txt("LOC_BGD_RAPPORT_ANNULER"), Protege(Annuler), PopupDialog.COMMAND_CANCEL },
		});
	end

	local function Ouvrir()
		if R.etat == "envoi" then
			-- Menu rouvert pendant l'envoi : on reprend le suivi.
			AfficherEnvoi();
			ContextPtr:SetUpdate(Suivre);
			return;
		end
		if R.etat == "formulaire" then
			-- Popup fermé sans réponse (menu fermé par un écran diplomatique) : on le rouvre, sans
			-- nouveau message.
			Formulaire();
			return;
		end
		R.etat = "formulaire";
		Journal("formulaire ouvert");
		Pauser();
		Chat("LOC_BGD_CHAT_RAPPORT_DEBUT", "COLOR_RED");
		Formulaire();
	end

	bouton:SetText(BGD_Txt("LOC_BGD_RAPPORT_BOUTON"));
	pcall(function() bouton:SetToolTipString(BGD_Txt("LOC_BGD_RAPPORT_INFOBULLE")); end);
	bouton:RegisterCallback(Mouse.eLClick, Protege(Ouvrir));
	pcall(function()
		bouton:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
	end);
	-- Menu fermé pendant l'envoi : le contexte caché n'a plus d'images, les événements du jeu
	-- prennent le relais.
	Events.GameCoreEventPublishComplete.Add(Suivre);
	Journal("bouton prêt");
end);
if not BGD_RapportOk then
	if Civ6Proxy_Journal ~= nil then pcall(Civ6Proxy_Journal, "rapport : erreur : " .. tostring(BGD_RapportErr)); end
end
end
