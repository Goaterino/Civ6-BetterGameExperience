-- ===========================================================================
-- BetterGameDLLs : annonce de version dans le salon multijoueur, ajoutée à la fin de
-- StagingRoom.lua (ou de son remplaçant, celui de MPH par exemple) par le proxy de démarrage
-- (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_Version()                -> version de BetterGameDLLs (proxy/src/version.rs) ; ou nil
--                                         et la raison, si cette machine n'appliquerait pas les
--                                         correctifs POL : elle ne s'annonce pas
--   Civ6Proxy_Jeton()                  -> une chaîne neuve à chaque appel
--   Civ6Proxy_Journal(texte)           -> écrit dans RolloverProxy.log
--   Civ6Proxy_FixerPol(actif, raison)  -> état lu par le proxy de GameCore (1 ou 0) : 0 à chaque
--                                         ouverture du salon, puis notre conclusion du verdict
--
-- Les configurations de joueur sont enregistrées avec la partie : une sauvegarde rechargée rapporte
-- les annonces et le verdict d'une session précédente, peut-être chez des joueurs qui n'ont plus le
-- proxy. D'où deux jetons, tirés à chaque ouverture du salon (et à chaque migration d'hôte) :
-- - chaque joueur écrit dans sa configuration BGD_VERSION = "<version>@<jeton de l'hôte>#<son
--   jeton>". Une annonce qui ne porte pas le jeton actuel de l'hôte ne compte pas ;
-- - l'hôte tient à jour BGD_VERDICT = "<version>@<son jeton>:<1|0>:<jetons des joueurs vus>" : 1 si
--   tous les humains connectés annoncent sa version avec son jeton ; les jetons de tous ceux dont il
--   a reçu une annonce de ce salon, quelle que soit leur version. Un client ne croit le verdict que
--   s'il y trouve son propre jeton du moment : un verdict sorti d'une sauvegarde ne peut pas le
--   contenir.
-- Ce verdict est le seul qui compte, le même pour toutes les machines : il active les correctifs
-- POL (« proxy-only lobbies »), ceux qui changent la simulation et désynchroniseraient un joueur
-- sans le proxy (BGD_PolActifs). Les autres correctifs n'en dépendent pas.
-- Au démarrage du compte à rebours de lancement, un message dans le chat, pour tous, commençant
-- par « BGD - » :
-- - l'hôte : en rouge les joueurs sans le proxy ou d'une autre version, sinon en vert la
--   confirmation ;
-- - si l'hôte n'a pas reçu notre annonce (il n'a pas le proxy, ou pas une version compatible), le
--   premier client équipé, par ordre d'identifiant, le dit en rouge. Un client « équipé » est un
--   client dont l'annonce a changé depuis l'ouverture du salon : une annonce restée d'une
--   sauvegarde ne change pas. Un client arrivé après l'annonce d'un autre ne la voit pas changer :
--   tous deux peuvent alors écrire, le même message. Il n'en manque jamais.
-- Les messages partent en anglais (BGD_TxtAnglais), lisibles par ceux qui n'ont pas BetterGameDLLs ;
-- chaque machine équipée les affiche dans sa langue (BGD_TxtChat, enveloppe d'OnChat en fin de bloc).
-- On ne s'appuie que sur les événements du jeu et sur StartLaunchCountdown, présents dans
-- StagingRoom.lua et dans ses remplaçants connus. Une erreur ici est journalisée, jamais propagée.
-- ===========================================================================

local BGD_CLE_VERSION = "BGD_VERSION";
local BGD_CLE_VERDICT = "BGD_VERDICT";

local BGD = {
	version = nil,     -- notre version, nil si cette machine n'appliquerait pas les correctifs POL
	raison = nil,      -- pourquoi, dans ce cas
	jeton = nil,       -- hôte : son jeton de cette ouverture du salon
	perso = nil,       -- notre jeton de cette ouverture du salon
	instantane = {},   -- annonces de chaque joueur à l'ouverture du salon
	message = nil,     -- dernier message envoyé dans le chat
	ouvert = false,    -- salon ouvert : de l'ouverture au chargement de la partie (ou au départ)
};

local function BGD_Journal(texte)
	if Civ6Proxy_Journal ~= nil then
		Civ6Proxy_Journal("salon : " .. texte);
	end
end

-- Salon ouvert d'une partie en réseau en direct : ni hotseat, ni PlayByCloud, où les joueurs
-- n'ouvrent pas le salon ensemble et ne peuvent donc pas partager un jeton. Une fois la partie
-- chargée, le salon se tait : le contexte reste chargé et reçoit encore les événements, et un
-- bilan en pleine partie couperait les correctifs POL sur une seule machine. QueuePopup affiche le
-- salon en différé : ContextPtr:IsHidden() ne dit pas s'il est ouvert.
local function BGD_Concerne()
	return BGD.ouvert and BGD.version ~= nil
		and GameConfiguration.IsNetworkMultiplayer()
		and not GameConfiguration.IsHotseat()
		and not GameConfiguration.IsPlayByCloud();
end

-- Annonce "<version>@<jeton de l'hôte>#<jeton du joueur>" -> les trois ; nil si la valeur manque
-- ou n'a pas cette forme.
local function BGD_LireAnnonce(valeur)
	if type(valeur) ~= "string" then return nil; end
	return string.match(valeur, "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
end

-- Verdict "<version>@<jeton de l'hôte>:<1|0>:<jetons>" -> version, jeton de l'hôte, actif
-- (booléen), table des jetons ; nil si la valeur manque ou n'a pas cette forme.
local function BGD_LireVerdict(valeur)
	if type(valeur) ~= "string" then return nil; end
	local version, jeton, actif, liste = string.match(valeur, "^([^@#:]*)@([^@#:]*):([01]):(.*)$");
	if version == nil then return nil; end
	local jetons = {};
	for j in string.gmatch(liste, "[^,]+") do jetons[j] = true; end
	return version, jeton, actif == "1", jetons;
end

local function BGD_Valeur(id, cle)
	local cfg = PlayerConfigurations[id];
	return cfg and cfg:GetValue(cle);
end

local function BGD_JetonDeLHote()
	if Network.IsGameHost() then
		if BGD.jeton == nil then
			BGD.jeton = Civ6Proxy_Jeton();
		end
		return BGD.jeton;
	end
	local _, jeton = BGD_LireAnnonce(BGD_Valeur(Network.GetGameHostPlayerID(), BGD_CLE_VERSION));
	return jeton or "";
end

-- Les autres humains connectés, dans l'ordre des identifiants.
local function BGD_AutresHumains()
	local localID = Network.GetLocalPlayerID();
	local ids = {};
	for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
		local cfg = PlayerConfigurations[id];
		if id ~= localID and cfg ~= nil and cfg:IsHuman()
			and cfg:GetSlotStatus() == SlotStatus.SS_TAKEN and Network.IsPlayerConnected(id) then
			table.insert(ids, id);
		end
	end
	return ids;
end

-- Hôte : ceux sans le proxy (ou sans le jeton de ce salon), ceux d'une autre version avec leur
-- version, et les jetons de tous ceux dont l'annonce porte le jeton de ce salon.
local function BGD_Bilan()
	local sans, autres, jetons = {}, {}, {};
	for _, id in ipairs(BGD_AutresHumains()) do
		local nom = Locale.Lookup(PlayerConfigurations[id]:GetPlayerName());
		local version, jeton, perso = BGD_LireAnnonce(BGD_Valeur(id, BGD_CLE_VERSION));
		if version == nil or jeton ~= BGD.jeton then
			table.insert(sans, nom);
		else
			table.insert(jetons, perso);
			if version ~= BGD.version then
				table.insert(autres, nom .. " (" .. version .. ")");
			end
		end
	end
	return sans, autres, jetons;
end

-- L'hôte a-t-il reçu notre annonce de ce salon ? (son verdict porte notre jeton)
local function BGD_HoteNousVoit()
	local hote = Network.GetGameHostPlayerID();
	local _, jetonHote = BGD_LireAnnonce(BGD_Valeur(hote, BGD_CLE_VERSION));
	local _, jeton, _, jetons = BGD_LireVerdict(BGD_Valeur(hote, BGD_CLE_VERDICT));
	return jeton ~= nil and jeton == jetonHote and BGD.perso ~= nil and jetons[BGD.perso] == true;
end

-- Les correctifs POL sont-ils actifs pour cette partie, d'après le verdict de l'hôte ?
local function BGD_PolActifs()
	if not BGD_Concerne() then return false; end
	local version, jeton, actif = BGD_LireVerdict(BGD_Valeur(Network.GetGameHostPlayerID(), BGD_CLE_VERDICT));
	if not actif or version ~= BGD.version then return false; end
	if Network.IsGameHost() then
		return jeton == BGD.jeton;
	end
	return BGD_HoteNousVoit();
end

-- Écrit notre annonce, et pour l'hôte le verdict, dans notre configuration de joueur ; ne la
-- diffuse que si elle a changé, sinon chaque PlayerInfoChanged en relancerait un.
local function BGD_Publier()
	if not BGD_Concerne() then return; end
	local cfg = PlayerConfigurations[Network.GetLocalPlayerID()];
	if cfg == nil then return; end
	if BGD.perso == nil then
		BGD.perso = Civ6Proxy_Jeton();
	end
	local jetonHote = BGD_JetonDeLHote();
	local annonce = BGD.version .. "@" .. jetonHote .. "#" .. BGD.perso;
	local change = false;
	if cfg:GetValue(BGD_CLE_VERSION) ~= annonce then
		cfg:SetValue(BGD_CLE_VERSION, annonce);
		change = true;
	end
	if Network.IsGameHost() then
		local sans, autres, jetons = BGD_Bilan();
		local verdict = BGD.version .. "@" .. jetonHote .. ":"
			.. ((#sans == 0 and #autres == 0) and "1" or "0") .. ":" .. table.concat(jetons, ",");
		if cfg:GetValue(BGD_CLE_VERDICT) ~= verdict then
			cfg:SetValue(BGD_CLE_VERDICT, verdict);
			change = true;
		end
	end
	if change then
		Network.BroadcastPlayerInfo();
		BGD_Journal("annonce " .. annonce .. (Network.IsGameHost() and (", verdict " .. tostring(cfg:GetValue(BGD_CLE_VERDICT))) or ""));
	end
	-- L'état lu par le proxy de GameCore suit le verdict jusqu'au lancement.
	Civ6Proxy_FixerPol(BGD_PolActifs() and 1 or 0, "verdict de l'hôte");
end

-- Un message dans le chat, pour tous, une fois par contenu.
local function BGD_Dire(texte)
	if texte ~= BGD.message then
		BGD.message = texte;
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_Journal("message : " .. texte);
	end
end

-- Hôte : le bilan.
local function BGD_MessageDeLHote()
	local sans, autres = BGD_Bilan();
	local texte;
	if #sans == 0 and #autres == 0 then
		BGD_Dire("[COLOR_GREEN]BGD - " .. BGD_TxtAnglais("LOC_BGD_CHAT_TOUS", BGD.version) .. "[ENDCOLOR]");
		return;
	elseif #autres == 0 then
		texte = BGD_TxtAnglais("LOC_BGD_CHAT_SANS", table.concat(sans, ", "));
	elseif #sans == 0 then
		texte = BGD_TxtAnglais("LOC_BGD_CHAT_AUTRES", BGD.version, table.concat(autres, ", "));
	else
		texte = BGD_TxtAnglais("LOC_BGD_CHAT_SANS_AUTRES", table.concat(sans, ", "), BGD.version, table.concat(autres, ", "));
	end
	BGD_Dire("[COLOR_RED]BGD - " .. texte .. "[ENDCOLOR]");
end

-- Client : si l'hôte ne nous voit pas, le premier client équipé le dit. Équipé : son annonce a
-- changé depuis l'ouverture du salon (une annonce restée d'une sauvegarde ne change pas).
local function BGD_MessageDUnClient()
	if BGD_HoteNousVoit() then return; end
	local moi = Network.GetLocalPlayerID();
	local hote = Network.GetGameHostPlayerID();
	for _, id in ipairs(BGD_AutresHumains()) do
		local valeur = BGD_Valeur(id, BGD_CLE_VERSION);
		if id < moi and id ~= hote and BGD_LireAnnonce(valeur) ~= nil and valeur ~= BGD.instantane[id] then
			return;  -- un client équipé avant nous s'en charge
		end
	end
	BGD_Dire("[COLOR_RED]BGD - " .. BGD_TxtAnglais("LOC_BGD_CHAT_HOTE_SANS") .. "[ENDCOLOR]");
end

-- Au démarrage du compte à rebours, sur chaque machine : annonce et verdict à jour, message, et la
-- conclusion locale au journal.
local function BGD_AvantLancement()
	if not BGD_Concerne() then return; end
	BGD_Publier();
	if Network.IsGameHost() then
		BGD_MessageDeLHote();
	else
		BGD_MessageDUnClient();
	end
	BGD_Journal("lancement : correctifs POL " .. (BGD_PolActifs() and "actifs" or "désactivés"));
end

local function BGD_Protege(f)
	return function(...)
		local ok, err = pcall(f, ...);
		if not ok then
			BGD_Journal("erreur : " .. tostring(err));
		end
	end
end

-- Nouvelle ouverture du salon (partie créée, rejointe ou rechargée), ou nouvel hôte : nouveaux
-- jetons, instantané des annonces présentes, puis la nôtre. Les autres se réannoncent en voyant le
-- nouveau jeton de l'hôte.
local function BGD_Ouverture()
	BGD.ouvert = true;
	-- Correctifs POL coupés jusqu'au verdict ; disponibilité relue, elle change quand le proxy de
	-- GameCore se charge (première partie) ou que ses réglages changent.
	Civ6Proxy_FixerPol(0, "ouverture du salon");
	BGD.version, BGD.raison = Civ6Proxy_Version();
	if BGD.version == nil then
		BGD_Journal("pas d'annonce : " .. tostring(BGD.raison));
	end
	BGD.jeton = nil;
	BGD.perso = nil;
	BGD.message = nil;
	BGD.instantane = {};
	if BGD_Concerne() then
		for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
			BGD.instantane[id] = BGD_Valeur(id, BGD_CLE_VERSION);
		end
	end
	BGD_Publier();
end

-- Correctif : au chargement d'une partie, le salon de MPH affiche tout emplacement sous le maximum
-- de joueurs de la carte (plafonné à 50, contre 12 dans le jeu de base). Ce maximum n'est pas
-- recalculé pour une partie chargée : les cités-États reçoivent alors un menu de dirigeant, leur
-- dirigeant (LEADER_MINOR_CIV_*) n'est pas dans la liste des dirigeants et ne peut plus être
-- changé : MissingDomainValue, « Opt. joueur invalides. », Prêt grisé pour tous jusqu'à la
-- prochaine reconstruction de la liste. Seuls les civilisations majeures ont un dirigeant à choisir :
-- cette erreur-là est ignorée pour les autres.
local BGD_ErreurIgnoree = {};
local function BGD_PasMajeur(id)
	local pc = PlayerConfigurations[id];
	if pc == nil or pc.GetCivilizationLevelTypeID == nil or CivilizationLevelTypes == nil then
		return false;
	end
	local niveau = pc:GetCivilizationLevelTypeID();
	return niveau ~= nil and niveau ~= 0 and niveau ~= CivilizationLevelTypes.CIVILIZATION_LEVEL_FULL_CIV;
end
if type(GetPlayerParameterError) == "function" then
	local BGD_GetPlayerParameterError = GetPlayerParameterError;
	GetPlayerParameterError = function(id, ...)
		local err = BGD_GetPlayerParameterError(id, ...);
		local ok, ignorer = pcall(function()
			return type(err) == "table" and err.Id == "MissingDomainValue" and BGD_PasMajeur(id);
		end);
		if ok and ignorer then
			if not BGD_ErreurIgnoree[id] then
				BGD_ErreurIgnoree[id] = true;
				BGD_Journal("joueur " .. tostring(id) .. " (pas une civilisation majeure) : dirigeant hors liste ignoré");
			end
			return nil;
		end
		return err;
	end
end

-- Diagnostic : erreur d'un paramètre de joueur qui grise le bouton Prêt (« Opt. joueur
-- invalides. », vu au chargement d'une partie, levé en changeant une option de partie). Journalisé
-- à chaque changement d'état, par joueur, avec ce qu'il faut pour en trouver la cause.
local BGD_ErreurVue = {};
local function BGD_DecrireErreur(id, err)
	local t = { "joueur " .. tostring(id) .. " : erreur " .. tostring(err.Id) .. "/" .. tostring(err.Reason) };
	local pc = PlayerConfigurations[id];
	local dirigeant;
	if pc ~= nil then
		dirigeant = pc:GetLeaderTypeName();
		table.insert(t, "statut " .. tostring(pc:GetSlotStatus()) .. ", humain " .. tostring(pc:IsHuman())
			.. ", participant " .. tostring(pc:IsParticipant()) .. ", dirigeant " .. tostring(dirigeant));
	end
	table.insert(t, "état " .. tostring(GameConfiguration.GetGameState())
		.. ", base " .. tostring(DB.ConfigurationChanges and DB.ConfigurationChanges()));
	local pp = GetPlayerParameters and GetPlayerParameters(id);
	local p = pp and pp.Parameters and pp.Parameters["PlayerLeader"];
	if p == nil then
		table.insert(t, "PlayerLeader absent");
	else
		local n, trouve = 0, false;
		for _, v in ipairs(p.Values or {}) do
			n = n + 1;
			if v.Value == dirigeant then
				trouve = true;
			end
		end
		table.insert(t, "domaine " .. tostring(p.Domain) .. ", " .. n .. " valeurs, dirigeant "
			.. (trouve and "présent" or "absent") .. ", valeur " .. tostring(p.Value and p.Value.Value));
	end
	return table.concat(t, " ; ");
end
local function BGD_DiagErreurs()
	if GetPlayerParameterError == nil then
		return;
	end
	local jeu = GetGameParametersError and GetGameParametersError();
	local cle = jeu and tostring(type(jeu) == "table" and (tostring(jeu.Id) .. "/" .. tostring(jeu.Reason)) or jeu);
	if cle ~= BGD_ErreurVue.jeu then
		BGD_ErreurVue.jeu = cle;
		BGD_Journal("diag : options de partie : " .. (cle and ("erreur " .. cle) or "erreur levée"));
	end
	for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
		local err = GetPlayerParameterError(id);
		cle = err and (tostring(err.Id) .. "/" .. tostring(err.Reason));
		if cle ~= BGD_ErreurVue[id] then
			BGD_ErreurVue[id] = cle;
			BGD_Journal("diag : " .. (err and BGD_DecrireErreur(id, err) or ("joueur " .. tostring(id) .. " : erreur levée")));
		end
	end
end
if type(UpdateReadyButton) == "function" then
	local BGD_UpdateReadyButton = UpdateReadyButton;
	local diag = BGD_Protege(BGD_DiagErreurs);
	UpdateReadyButton = function(...)
		local r = BGD_UpdateReadyButton(...);
		diag();
		return r;
	end
end

if Civ6Proxy_FixerPol == nil or Civ6Proxy_Version == nil then
	BGD_Journal("fonctions natives absentes : pas d'annonce");
else
	local BGD_StartLaunchCountdown = StartLaunchCountdown;
	if type(BGD_StartLaunchCountdown) == "function" then
		local avant = BGD_Protege(BGD_AvantLancement);
		StartLaunchCountdown = function(...)
			avant();
			return BGD_StartLaunchCountdown(...);
		end
	else
		BGD_Journal("StartLaunchCountdown introuvable : pas de message avant le lancement");
	end
	LuaEvents.HostGame_ShowStagingRoom.Add(BGD_Protege(BGD_Ouverture));
	LuaEvents.JoiningRoom_ShowStagingRoom.Add(BGD_Protege(BGD_Ouverture));
	Events.MultiplayerHostMigrated.Add(BGD_Protege(function()
		if BGD.ouvert then
			BGD_Ouverture();
		end
	end));
	-- Partie chargée, ou salon quitté : le salon se tait (voir BGD_Concerne).
	local function fermeture()
		BGD.ouvert = false;
	end
	Events.LoadScreenContentReady.Add(fermeture);
	Events.LeaveGameComplete.Add(fermeture);
	Events.MultiplayerPlayerConnected.Add(BGD_Protege(BGD_Publier));
	Events.PlayerInfoChanged.Add(BGD_Protege(function()
		if not ContextPtr:IsHidden() then
			BGD_Publier();
		end
	end));
	BGD_Journal("bloc chargé");
end

-- Messages de BetterGameDLLs reçus en anglais, affichés dans la langue du jeu. OnChat est la globale
-- de StagingRoom.lua (et de ses remplaçants), qu'OnMultiplayerChat appelle par son nom.
if OnChat ~= nil and BGD_TxtChat ~= nil and BGD_SalonChat == nil then
	BGD_SalonChat = true;
	local BGD_OnChatBase = OnChat;
	OnChat = function(fromPlayer, toPlayer, text, ...)
		local ok, traduit = pcall(BGD_TxtChat, text);
		if ok and traduit ~= nil then
			text = traduit;
		end
		return BGD_OnChatBase(fromPlayer, toPlayer, text, ...);
	end
end
