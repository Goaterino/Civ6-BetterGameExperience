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
-- En multijoueur, chaque étape a son message dans le chat de tous : formulaire ouvert, rapport
-- annulé, envoi en cours, envoi terminé (ou en échec). Les messages partent en anglais, et chaque
-- machine qui a BetterGameExperience les affiche dans sa langue (BGD_TxtChat). La partie n'est pas
-- mise en pause.
-- Couleurs atténuées de la palette du jeu (COLOR_Civ6Green, COLOR_Civ6Red), propres à ce module.
-- Le formulaire (BGD_RapportFond, ajouté au XML par le proxy) a un champ de texte pour décrire le
-- problème, envoyé dans meta.json. L'envoi tourne dans un thread du proxy ; un popup du jeu
-- (PopupDialog) dit qu'il est en cours. Le menu l'interroge à chaque image tant qu'il est affiché ;
-- fermé, il n'a plus d'images (et la partie en pause n'a plus d'événements) : le bloc du suivi, dans
-- InGame (rapport_suivi.lua), toujours affiché, prend le relais. La fin (rapport_fin.lua, placé
-- devant ce bloc) ne passe pas par un popup, que le joueur ne verrait qu'en rouvrant le menu : un
-- message à tous, et une ligne du chat que seul le joueur local voit (numéro ou chemin du zip).
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

	-- Sauvegarde de l'instant du rapport, comme la sauvegarde rapide du menu : le proxy l'attend
	-- (BGE_rapport.Civ6Save) avant de faire le zip, et s'en passe si elle n'arrive pas.
	local function Sauvegarder()
		local ok, err = pcall(function()
			local fichier = {};
			fichier.Name = "BGE_rapport";
			fichier.Location = SaveLocations.LOCAL_STORAGE;
			fichier.Type = Network.GetGameConfigurationSaveType();
			fichier.IsAutosave = false;
			fichier.IsQuicksave = false;
			Network.SaveGame(fichier);
		end);
		if not ok then Journal("sauvegarde impossible : " .. tostring(err)); end
	end

	-- Ce que le bloc sait de la partie, pour meta.json : une ligne « clé=valeur » par champ.
	local function Meta(description)
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
		-- Identifiant Steam 64 bits du joueur (celui que le salon ouvre en profil Steam) ; vide en
		-- solo selon les cas, et le proxy lit alors le compte Steam connecté (steam_id_windows).
		Ajouter("steam_id", function()
			local sid = tostring(cfg:GetNetworkIdentifer() or "");
			if string.match(sid, "^%d+$") and sid ~= "0" then return sid; end
			return nil;
		end);
		Ajouter("civilisation", function() return Locale.Lookup(cfg:GetCivilizationShortDescription()); end);
		Ajouter("dirigeant", function() return cfg:GetLeaderTypeName(); end);
		Ajouter("multijoueur", function() return EnLigne() and 1 or 0; end);
		Ajouter("hote", function() return Network.IsGameHost() and 1 or 0; end);
		Ajouter("en_pause", function() return GameConfiguration.IsPaused() and 1 or 0; end);
		-- GetMultiplayerPlayerIDs rend tous les emplacements (IA et vides compris) : on ne garde que
		-- les emplacements occupés par un humain.
		Ajouter("humains", function()
			local n = 0;
			for _, pid in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
				local pc = PlayerConfigurations[pid];
				if pc ~= nil and pc:IsHuman() and pc:GetSlotStatus() == SlotStatus.SS_TAKEN then
					n = n + 1;
				end
			end
			return n;
		end);
		Ajouter("langue", function() return Locale.GetCurrentLanguage().Type; end);
		Ajouter("description", function() return description; end);
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

	-- Fin du rapport (rapport_fin.lua), par le suivi d'InGame ou, à défaut, par ce menu : il remet
	-- son état à zéro et ferme le popup « envoi en cours » s'il est resté ouvert.
	LuaEvents.BGD_RapportFini.Add(Protege(function()
		if R.etat ~= "envoi" then return; end
		R.etat = nil;
		ArreterSuivi();
		if R.popup ~= nil then pcall(function() R.popup:Close(); end); end
	end));

	local function Terminer(statut, texte, archive)
		-- Le suivi d'InGame termine le rapport (il est toujours chargé en partie) ; sinon, ce menu.
		LuaEvents.BGD_RapportResultat(statut, texte, archive);
		if R.etat == "envoi" then
			Journal("pas de suivi dans InGame : fin par le menu");
			BGD_RapportTerminer(statut, texte, archive, Journal);
		end
	end

	Suivre = Protege(function()
		if R.etat ~= "envoi" then return; end
		local statut, texte, archive = Civ6Proxy_RapportEtat();
		if statut == nil or statut == "" then return; end
		Terminer(statut, texte, archive);
	end);

	local function AfficherEnvoi()
		Afficher(BGD_Txt("LOC_BGD_RAPPORT_TITRE"), BGD_Txt("LOC_BGD_RAPPORT_EN_COURS"),
			{ { BGD_Txt("LOC_BGD_RAPPORT_FERMER"), function() end, PopupDialog.COMMAND_CANCEL } });
	end

	-- Le cadre suit la hauteur de son contenu (explication plus ou moins longue selon la langue).
	local function Ajuster()
		pcall(function()
			Controls.BGD_RapportPile:CalculateSize();
			local h = Controls.BGD_RapportPile:GetSizeY() + 36;
			Controls.BGD_RapportCadre:SetSizeY(h);
			Controls.BGD_RapportBord:SetSizeY(h + 4);
		end);
	end

	local function Formulaire(afficher)
		if not afficher then
			Controls.BGD_RapportFond:SetHide(true);
			return;
		end
		Controls.BGD_RapportTitre:SetText(BGD_Txt("LOC_BGD_RAPPORT_TITRE"));
		Controls.BGD_RapportExplication:SetText(BGD_Txt("LOC_BGD_RAPPORT_TEXTE"));
		Controls.BGD_RapportConsigne:SetText(BGD_Txt("LOC_BGD_RAPPORT_DESCRIPTION"));
		Controls.BGD_RapportConfirmer:SetText(BGD_Txt("LOC_BGD_RAPPORT_CONFIRMER"));
		Controls.BGD_RapportAnnuler:SetText(BGD_Txt("LOC_BGD_RAPPORT_ANNULER"));
		Controls.BGD_RapportFond:SetHide(false);
		Ajuster();
		pcall(function() Controls.BGD_RapportTexte:TakeFocus(); end);
	end

	local function Confirmer()
		if R.etat ~= "formulaire" then return; end
		local description = Controls.BGD_RapportTexte:GetText() or "";
		Formulaire(false);
		R.etat = "envoi";
		Journal("description : " .. description);
		Chat("LOC_BGD_CHAT_RAPPORT_ENVOI", "COLOR_Civ6Green");
		Sauvegarder();
		local ok, err = Civ6Proxy_RapportEnvoyer(Meta(description));
		if ok ~= 1 then
			Journal("envoi refusé par le proxy : " .. tostring(err));
			BGD_RapportTerminer("echec", err, "", Journal);
			return;
		end
		AfficherEnvoi();
		ContextPtr:SetUpdate(Suivre);
		LuaEvents.BGD_RapportSuivre();
	end

	local function Annuler()
		if R.etat ~= "formulaire" then return; end
		Formulaire(false);
		R.etat = nil;
		Journal("rapport annulé");
		Chat("LOC_BGD_CHAT_RAPPORT_ANNULE", "COLOR_Civ6Green");
	end

	local function Ouvrir()
		if R.etat == "envoi" then
			-- Menu rouvert pendant l'envoi : on reprend le suivi.
			AfficherEnvoi();
			ContextPtr:SetUpdate(Suivre);
			return;
		end
		if R.etat == "formulaire" then
			-- Formulaire resté ouvert (menu fermé par Échap ou un écran diplomatique) : on le rouvre,
			-- texte compris, sans nouveau message.
			Formulaire(true);
			return;
		end
		R.etat = "formulaire";
		Journal("formulaire ouvert");
		Chat("LOC_BGD_CHAT_RAPPORT_DEBUT", "COLOR_Civ6Red");
		Controls.BGD_RapportTexte:SetText("");
		Formulaire(true);
	end

	if Controls.BGD_RapportFond == nil or Controls.BGD_RapportTexte == nil then
		Journal("formulaire absent du XML, pas de rapport");
		bouton:SetHide(true);
		return;
	end
	Controls.BGD_RapportConfirmer:RegisterCallback(Mouse.eLClick, Protege(Confirmer));
	Controls.BGD_RapportAnnuler:RegisterCallback(Mouse.eLClick, Protege(Annuler));
	bouton:SetText(BGD_Txt("LOC_BGD_RAPPORT_BOUTON"));
	pcall(function() bouton:SetToolTipString(BGD_Txt("LOC_BGD_RAPPORT_INFOBULLE")); end);
	bouton:RegisterCallback(Mouse.eLClick, Protege(Ouvrir));
	pcall(function()
		bouton:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
	end);
	Journal("bouton prêt");
end);
if not BGD_RapportOk then
	if Civ6Proxy_Journal ~= nil then pcall(Civ6Proxy_Journal, "rapport : erreur : " .. tostring(BGD_RapportErr)); end
end
end
