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
--   Civ6Proxy_VersionInstallee()       -> version de ce proxy, même s'il est désactivé (BGE Check)
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
-- À chaque ouverture du salon (pas à une migration d'hôte), une machine équipée et active dit à tous,
-- en vert, qu'elle a BetterGameExperience et sa version exacte (« BGE - Bob has BetterGameExperience
-- 0.9.0 enabled. »), une demi-seconde après l'ouverture (premier tic du minuteur, voir plus bas).
-- Le BGE Check : un bouton de l'hôte, placé comme le « MOD CHECK » de MPH mais à gauche (symétrique
-- par rapport au centre de l'écran), demande à chaque humain connecté sa version par un message
-- privé caché « .bgd_check <jeton> ». Le bloc de chaque machine équipée répond, même désactivée, par
-- « .bgd_version <jeton> <version installée> <1|0> » (1 : BetterGameExperience actif). Quand tous ont
-- répondu, ou au bout de BGD_CHECK_DELAI tics, l'hôte écrit le bilan à tous. Les messages « .bgd_ »
-- ne s'affichent pas chez qui a BetterGameExperience (enveloppe d'OnChat). Le proxy a ajouté au XML du
-- salon le bouton (BGD_CheckButton) et un minuteur (BGD_SalonMinuteur, environ 0,5 s) ; sans eux,
-- l'annonce part tout de suite et le bouton n'existe pas.
-- Les messages partent en anglais (BGD_TxtAnglais), lisibles par ceux qui n'ont pas BetterGameDLLs ;
-- chaque machine équipée les affiche dans sa langue (BGD_TxtChat, enveloppe d'OnChat en fin de bloc).
-- On ne s'appuie que sur les événements du jeu et sur StartLaunchCountdown, présents dans
-- StagingRoom.lua et dans ses remplaçants connus. Une erreur ici est journalisée, jamais propagée.
-- ===========================================================================

local BGD_CLE_VERSION = "BGD_VERSION";
local BGD_CLE_VERDICT = "BGD_VERDICT";
local BGD_CLE_MEDIAS = "BGD_MEDIAS";

local BGD = {
	version = nil,     -- notre version, nil si cette machine n'appliquerait pas les correctifs POL
	raison = nil,      -- pourquoi, dans ce cas
	jeton = nil,       -- hôte : son jeton de cette ouverture du salon
	perso = nil,       -- notre jeton de cette ouverture du salon
	instantane = {},   -- annonces de chaque joueur à l'ouverture du salon
	message = nil,     -- dernier message envoyé dans le chat
	ouvert = false,    -- salon ouvert : de l'ouverture au chargement de la partie (ou au départ)
	annonce = false,   -- annonce « BetterGameExperience activé » à envoyer au prochain tic
	check = nil,       -- hôte : BGE Check en cours { jeton, ordre, attendus, reponses, tics }
};

local BGD_CHECK_DEMANDE = ".bgd_check ";
local BGD_CHECK_REPONSE = ".bgd_version ";
local BGD_CHECK_DELAI = 12;  -- tics du minuteur (environ 6 s) avant le bilan sans toutes les réponses

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
	local pings = GameConfiguration.GetValue("BGD_TEAM_PINGS");
	local avecObservateurs = pings == true or pings == 1;
	local localID = Network.GetLocalPlayerID();
	local ids = {};
	for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
		local cfg = PlayerConfigurations[id];
		if id ~= localID and cfg ~= nil
			and ((cfg:IsHuman() and cfg:GetSlotStatus() == SlotStatus.SS_TAKEN)
				or (avecObservateurs and cfg:GetSlotStatus() == SlotStatus.SS_OBSERVER)) and Network.IsPlayerConnected(id) then
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
	-- Images et sons des signaux en place (posés par le proxy de démarrage) : notre jeton de ce
	-- salon, qu'une valeur restée d'une sauvegarde ne peut pas porter. Lu par l'hôte au lancement.
	local medias = (Civ6Proxy_MediasPings ~= nil and Civ6Proxy_MediasPings() == 1) and BGD.perso or "";
	if cfg:GetValue(BGD_CLE_MEDIAS) ~= medias then
		cfg:SetValue(BGD_CLE_MEDIAS, medias);
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
		BGD_Dire("[COLOR_GREEN]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_TOUS", BGD.version) .. "[ENDCOLOR]");
		return;
	elseif #autres == 0 then
		texte = BGD_TxtAnglais("LOC_BGD_CHAT_SANS", table.concat(sans, ", "));
	elseif #sans == 0 then
		texte = BGD_TxtAnglais("LOC_BGD_CHAT_AUTRES", BGD.version, table.concat(autres, ", "));
	else
		texte = BGD_TxtAnglais("LOC_BGD_CHAT_SANS_AUTRES", table.concat(sans, ", "), BGD.version, table.concat(autres, ", "));
	end
	BGD_Dire("[COLOR_RED]BGE - " .. texte .. "[ENDCOLOR]");
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
	BGD_Dire("[COLOR_RED]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_HOTE_SANS") .. "[ENDCOLOR]");
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

-- Un message, tel quel, pour tous (annonce, bilan du BGE Check : pas de déduplication).
local function BGD_Envoyer(texte)
	Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
	BGD_Journal("message : " .. texte);
end

local function BGD_Prive(texte, id)
	Network.SendChat(texte, ChatTargetTypes.CHATTARGET_PLAYER or ChatTargetTypes.CHATTARGET_ALL, id);
end

local function BGD_Nom(id)
	local cfg = PlayerConfigurations[id];
	return cfg and Locale.Lookup(cfg:GetPlayerName()) or ("#" .. tostring(id));
end

-- Annonce d'arrivée : cette machine a BetterGameExperience actif, à telle version.
local function BGD_Annoncer()
	BGD.annonce = false;
	if not BGD_Concerne() then return; end
	BGD_Envoyer("[COLOR_GREEN]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_ANNONCE", BGD_Nom(Network.GetLocalPlayerID()), BGD.version) .. "[ENDCOLOR]");
end

-- Salon d'une partie en réseau en direct, que BetterGameExperience soit actif ou non (réponse au BGE Check).
local function BGD_SalonEnReseau()
	return BGD.ouvert and GameConfiguration.IsNetworkMultiplayer()
		and not GameConfiguration.IsHotseat() and not GameConfiguration.IsPlayByCloud();
end

local function BGD_Liste(t)
	return #t > 0 and table.concat(t, ", ") or "-";
end

-- Hôte : bilan du BGE Check, pour tous. L'hôte est compté avec sa propre version.
local function BGD_CheckBilan()
	local c = BGD.check;
	BGD.check = nil;
	if c == nil then return; end
	local memes, autres, inactifs, muets = { BGD_Nom(Network.GetLocalPlayerID()) }, {}, {}, {};
	for _, id in ipairs(c.ordre) do
		local r = c.reponses[id];
		local nom = BGD_Nom(id);
		if r == nil then
			if Network.IsPlayerConnected(id) then table.insert(muets, nom); end
		elseif not r.actif then
			table.insert(inactifs, nom .. " (" .. r.version .. ")");
		elseif r.version ~= BGD.version then
			table.insert(autres, nom .. " (" .. r.version .. ")");
		else
			table.insert(memes, nom);
		end
	end
	if #autres == 0 and #inactifs == 0 and #muets == 0 then
		BGD_Envoyer("[COLOR_GREEN]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_CHECK_OK", BGD.version, BGD_Liste(memes)) .. "[ENDCOLOR]");
	else
		BGD_Envoyer("[COLOR_RED]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_CHECK_ECARTS", BGD.version,
			BGD_Liste(memes), BGD_Liste(autres), BGD_Liste(inactifs), BGD_Liste(muets)) .. "[ENDCOLOR]");
	end
end

-- Hôte : le bouton ne sert que dans un salon où BetterGameExperience est actif chez lui, minuteur
-- présent (sans lui, pas de délai pour ceux qui ne répondent pas). Caché pendant le draft de MPH :
-- son bouton RESET prend la même place.
local function BGD_CheckBouton()
	local b = Controls ~= nil and Controls.BGD_CheckButton;
	if not b then return; end
	local montrer = BGD_Concerne() and Network.IsGameHost() and Controls.BGD_SalonMinuteur ~= nil;
	if montrer and Controls.ResetButton ~= nil and not Controls.ResetButton:IsHidden() then
		montrer = false;
	end
	b:SetHide(not montrer);
	b:SetDisabled(BGD.check ~= nil);
end

-- Hôte : clic sur le bouton BGE Check.
local function BGD_CheckLancer()
	if BGD.check ~= nil or not BGD_Concerne() or not Network.IsGameHost() then return; end
	local c = { jeton = Civ6Proxy_Jeton(), ordre = BGD_AutresHumains(), attendus = {}, reponses = {}, tics = 0 };
	BGD.check = c;
	BGD_Journal("BGE Check : demande " .. c.jeton .. " à " .. #c.ordre .. " joueur(s)");
	for _, id in ipairs(c.ordre) do
		c.attendus[id] = true;
		BGD_Prive(BGD_CHECK_DEMANDE .. c.jeton, id);
	end
	if #c.ordre == 0 then
		BGD_CheckBilan();
	end
	BGD_CheckBouton();
end

-- Hôte : tous ceux qu'on attend ont-ils répondu (ou sont-ils partis) ?
local function BGD_CheckComplet(c)
	for _, id in ipairs(c.ordre) do
		if c.reponses[id] == nil and Network.IsPlayerConnected(id) then return false; end
	end
	return true;
end

-- Messages cachés du BGE Check, sur chaque machine (Events.MultiplayerChat).
local function BGD_CheckChat(fromPlayer, toPlayer, text)
	if type(text) ~= "string" or string.sub(text, 1, 5) ~= ".bgd_" then return; end
	if fromPlayer == Network.GetLocalPlayerID() or not BGD_SalonEnReseau() then return; end
	local hote = Network.GetGameHostPlayerID();
	if string.sub(text, 1, #BGD_CHECK_DEMANDE) == BGD_CHECK_DEMANDE then
		local jeton = string.match(string.sub(text, #BGD_CHECK_DEMANDE + 1), "^(%w+)$");
		if jeton == nil or fromPlayer ~= hote then return; end
		local version = (Civ6Proxy_VersionInstallee ~= nil and Civ6Proxy_VersionInstallee()) or "?";
		local actif = Civ6Proxy_Version() ~= nil;
		BGD_Prive(BGD_CHECK_REPONSE .. jeton .. " " .. version .. " " .. (actif and "1" or "0"), hote);
		BGD_Journal("BGE Check : réponse " .. version .. (actif and " (actif)" or " (inactif)"));
	elseif string.sub(text, 1, #BGD_CHECK_REPONSE) == BGD_CHECK_REPONSE then
		local c = BGD.check;
		if c == nil or not Network.IsGameHost() or not c.attendus[fromPlayer] then return; end
		local jeton, version, actif = string.match(string.sub(text, #BGD_CHECK_REPONSE + 1), "^(%w+) ([%w%.%+%-_%?]+) ([01])$");
		if jeton ~= c.jeton then return; end
		c.reponses[fromPlayer] = { version = version, actif = actif == "1" };
		if BGD_CheckComplet(c) then
			BGD_CheckBilan();
			BGD_CheckBouton();
		end
	end
end

-- Salon de MPH : MPH descend le tableau des joueurs de 66 px (PrimaryPanelStack à 180 au lieu de
-- 114 dans le jeu de base) pour y loger son draft (phase, consignes, bans, votes de carte), et une
-- partie rechargée montre son libellé de phase sans texte, dont la valeur du XML est « . ». Hors
-- draft, on vide ce libellé et on remonte le tableau de BGD.ESPACE px, agrandi d'autant (son bas ne
-- bouge pas) ; le bandeau des colonnes (HeaderStack) et la version de MPH (PhaseLabel_Version)
-- remontent avec lui. Dès qu'un contrôle du draft paraît, la mise en page de MPH revient.
BGD.ESPACE = 52;
BGD.DRAFT = { "PhaseButton", "ResetButton", "PhaseLabel_hint", "BanLabel", "BanPullDown",
	"BanMadeLabel", "PickMadeLabel", "PickedMapLabel", "BanMapLabel", "BanMapPullDown", "MapPoolLabel",
	"clockLabel", "RefreshLabel", "VoteMapScriptPullDown", "VoteMapAgePullDown", "VoteMapTempPullDown",
	"VoteMapScriptLabel", "VoteMapAgeLabel", "VoteMapTempLabel" };
BGD.mise = nil;  -- { y, version, gain } : positions de MPH et place rendue

function BGD.DraftVisible()
	for _, nom in ipairs(BGD.DRAFT) do
		local c = Controls[nom];
		if c ~= nil and not c:IsHidden() then return true; end
	end
	local phase = Controls.PhaseLabel;
	if not phase:IsHidden() then
		local t = phase:GetText();
		if t ~= nil and t ~= "" and t ~= "." then return true; end
	end
	return false;
end

function BGD.MiseEnPage()
	if Controls == nil or Controls.PhaseLabel == nil or Controls.PrimaryPanelStack == nil or Controls.PrimaryStackGrid == nil then
		return;  -- pas le salon de MPH
	end
	local pile, grille = Controls.PrimaryPanelStack, Controls.PrimaryStackGrid;
	if BGD.mise == nil then
		-- Remontent ensemble : le tableau, le bandeau de ses colonnes (un contrôle à part) et la
		-- version de MPH. Positions de MPH notées une fois.
		BGD.mise = { y = pile:GetOffsetY(), gain = 0, origines = {} };
		for _, nom in ipairs({ "PrimaryPanelStack", "HeaderStack", "PhaseLabel_Version" }) do
			if Controls[nom] ~= nil then BGD.mise.origines[nom] = Controls[nom]:GetOffsetY(); end
		end
		if BGD.mise.y < 114 + BGD.ESPACE then
			BGD_Journal("mise en page de MPH inattendue (tableau à " .. tostring(BGD.mise.y) .. ") : laissée telle quelle");
		end
	end
	local m = BGD.mise;
	if m.y < 114 + BGD.ESPACE then return; end
	if not Controls.PhaseLabel:IsHidden() and Controls.PhaseLabel:GetText() == "." then
		Controls.PhaseLabel:SetText("");
	end
	local gain = BGD.DraftVisible() and 0 or BGD.ESPACE;
	if gain == m.gain then return; end
	for nom, y in pairs(m.origines) do
		Controls[nom]:SetOffsetY(y - gain);
	end
	grille:SetSizeY(grille:GetSizeY() + gain - m.gain);
	m.gain = gain;
	pile:CalculateSize();
	if RealizeGridSize ~= nil then RealizeGridSize(); end
end

-- Un tic du minuteur (environ 0,5 s), tant que le salon est ouvert : annonce différée, bouton,
-- délai du BGE Check.
local function BGD_Tic()
	if BGD.annonce then
		BGD_Annoncer();
	end
	local c = BGD.check;
	if c ~= nil then
		c.tics = c.tics + 1;
		if c.tics >= BGD_CHECK_DELAI or BGD_CheckComplet(c) then
			BGD_CheckBilan();
		end
	end
	BGD_CheckBouton();
	BGD.MiseEnPage();
end

-- (Re)lance le minuteur ; sans lui, l'annonce part tout de suite.
local function BGD_Armer()
	local m = Controls ~= nil and Controls.BGD_SalonMinuteur;
	if not m then
		if BGD.annonce then BGD_Annoncer(); end
		return;
	end
	m:SetToBeginning();
	m:Play();
	BGD_CheckBouton();
	BGD.MiseEnPage();
end

-- Nouvelle ouverture du salon (partie créée, rejointe ou rechargée), ou nouvel hôte : nouveaux
-- jetons, instantané des annonces présentes, puis la nôtre. Les autres se réannoncent en voyant le
-- nouveau jeton de l'hôte.
local function BGD_Ouverture(migration)
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
	BGD.check = nil;
	BGD.annonce = (migration ~= true);
	if BGD_Concerne() then
		for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
			BGD.instantane[id] = BGD_Valeur(id, BGD_CLE_VERSION);
		end
	end
	BGD_Publier();
	BGD_Armer();
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
			BGD_Ouverture(true);
		end
	end));
	Events.MultiplayerChat.Add(BGD_Protege(BGD_CheckChat));
	if Controls ~= nil and Controls.BGD_SalonMinuteur ~= nil then
		local tic = BGD_Protege(BGD_Tic);
		Controls.BGD_SalonMinuteur:RegisterEndCallback(function()
			if BGD.ouvert then
				tic();
				Controls.BGD_SalonMinuteur:SetToBeginning();
				Controls.BGD_SalonMinuteur:Play();
			end
		end);
	else
		BGD_Journal("minuteur absent du XML du salon : annonce immédiate, pas de bouton BGE Check");
	end
	if Controls ~= nil and Controls.BGD_CheckButton ~= nil then
		local b = Controls.BGD_CheckButton;
		b:SetText("BGE CHECK");
		b:SetToolTipString(BGD_Txt("LOC_BGD_SALON_CHECK_AIDE"));
		b:RegisterCallback(Mouse.eLClick, BGD_Protege(BGD_CheckLancer));
		b:RegisterCallback(Mouse.eMouseEnter, function() UI.PlaySound("Main_Menu_Mouse_Over"); end);
		b:SetHide(true);
	end
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
	-- Redimensionnement de MPH (changement de résolution) : il recalcule la hauteur du tableau
	-- sans la place rendue, on la rajoute.
	if type(Resize) == "function" then
		BGD.ResizeBase = Resize;
		Resize = function(...)
			BGD.ResizeBase(...);
			if BGD.mise ~= nil and BGD.mise.gain > 0 then
				BGD_Protege(function()
					Controls.PrimaryStackGrid:SetSizeY(Controls.PrimaryStackGrid:GetSizeY() + BGD.mise.gain);
					Controls.PrimaryPanelStack:CalculateSize();
					if RealizeGridSize ~= nil then RealizeGridSize(); end
				end)();
			end
		end
	end
	BGD_Journal("bloc chargé");
end

-- Messages de BetterGameDLLs reçus en anglais, affichés dans la langue du jeu ; messages « .bgd_ »
-- cachés. OnChat est la globale
-- de StagingRoom.lua (et de ses remplaçants), qu'OnMultiplayerChat appelle par son nom.
if OnChat ~= nil and BGD_TxtChat ~= nil and BGD_SalonChat == nil then
	BGD_SalonChat = true;
	local BGD_OnChatBase = OnChat;
	OnChat = function(fromPlayer, toPlayer, text, ...)
		-- Messages cachés du BGE Check (.bgd_check, .bgd_version).
		if type(text) == "string" and string.sub(text, 1, 5) == ".bgd_" then
			return;
		end
		local ok, traduit = pcall(BGD_TxtChat, text);
		if ok and traduit ~= nil then
			text = traduit;
		end
		return BGD_OnChatBase(fromPlayer, toPlayer, text, ...);
	end
end

-- ===========================================================================
-- Images et sons des signaux d'équipe : mod local « BGE - Ping assets » (DLC\BGEPings, installé
-- avec BGE, sans identifiant Workshop). Présent dans la configuration de la partie, il ferait
-- expulser tout joueur qui ne l'a pas dès son arrivée dans le salon (KICK_MOD_MISSING) : il n'y est
-- donc jamais pendant le salon. Le menu principal le laisse décoché (src/ping_assets.lua) et
-- l'hôte le retire d'une configuration qui l'a déjà (sauvegarde d'une partie jouée avec les
-- signaux) à l'ouverture du salon.
-- L'hôte ne l'ajoute qu'au lancement, juste avant Network.LaunchGame (par où passent tous les
-- lancements de StagingRoom.lua et de MPH : fin du compte à rebours, départ forcé, phases de
-- MPH), et seulement si les signaux d'équipe sont cochés et que les correctifs POL sont actifs :
-- tous les humains ont alors cette version de BGE. Chacun dit en plus dans sa configuration de
-- joueur s'il a le mod (BGD_MEDIAS, posé par le proxy de démarrage, src/medias.rs) : s'il manque
-- chez l'un d'eux, il n'est pas ajouté. La configuration part avant l'ordre de lancement.
-- StagingRoom.lua (et celui de MPH) retire le « Prêt » du joueur local à chaque GameConfigChanged :
-- quand le seul changement est l'arrivée ou le départ de ce mod, SetLocalReady(false) est ignoré.
-- ===========================================================================
do
	local BGD_MEDIA_PINGS = "863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd";

	local function DansConfig()
		if type(GameConfiguration.GetEnabledMods) ~= "function" then return false; end
		for _, mod in ipairs(GameConfiguration.GetEnabledMods() or {}) do
			if mod.Id == BGD_MEDIA_PINGS then return true; end
		end
		return false;
	end

	local vu = nil;  -- présence du mod dans la configuration au dernier GameConfigChanged

	-- Hôte, à l'ouverture du salon : retire le mod (il n'y a pas de RemoveEnabledMods : la liste est
	-- refaite sans lui).
	local function Retirer()
		vu = DansConfig();
		if not Network.IsGameHost() or not vu or type(GameConfiguration.ClearEnabledMods) ~= "function" then
			return;
		end
		local autres = {};
		for _, mod in ipairs(GameConfiguration.GetEnabledMods()) do
			if mod.Id ~= BGD_MEDIA_PINGS then table.insert(autres, mod.Id); end
		end
		GameConfiguration.ClearEnabledMods();
		for _, id in ipairs(autres) do
			GameConfiguration.AddEnabledMods(id);
		end
		local reste = #(GameConfiguration.GetEnabledMods() or {});
		Network.BroadcastGameConfig();
		BGD_Journal("signaux : images et sons retirés de la configuration du salon ("
			.. reste .. " mods sur " .. #autres .. " attendus)");
	end

	-- Hôte, juste avant le lancement.
	local function Ajouter()
		if not Network.IsGameHost() or DansConfig() then return; end
		local pings = GameConfiguration.GetValue("BGD_TEAM_PINGS");
		if not (pings == true or pings == 1) then return; end
		if not BGD_PolActifs() then
			BGD_Journal("signaux : images et sons non ajoutés, correctifs POL inactifs");
			return;
		end
		-- Chacun doit avoir le mod sur sa machine (BGD_MEDIAS, écrit par BGD_Publier), sinon il
		-- serait expulsé au lancement : sans lui, les signaux restent sans images ni sons.
		local manquants = {};
		if Civ6Proxy_MediasPings == nil or Civ6Proxy_MediasPings() ~= 1 then
			table.insert(manquants, "hôte");
		end
		for _, id in ipairs(BGD_AutresHumains()) do
			local _, _, perso = BGD_LireAnnonce(BGD_Valeur(id, BGD_CLE_VERSION));
			if perso == nil or BGD_Valeur(id, BGD_CLE_MEDIAS) ~= perso then
				table.insert(manquants, Locale.Lookup(PlayerConfigurations[id]:GetPlayerName()));
			end
		end
		if #manquants > 0 then
			BGD_Journal("signaux : images et sons non ajoutés, absents chez " .. table.concat(manquants, ", "));
			return;
		end
		if Modding == nil or Modding.GetModHandle(BGD_MEDIA_PINGS) == nil then
			BGD_Journal("signaux : mod des images et sons introuvable, non ajouté");
			return;
		end
		GameConfiguration.AddEnabledMods(BGD_MEDIA_PINGS);
		if not DansConfig() then
			BGD_Journal("signaux : AddEnabledMods sans effet, images et sons absents");
			return;
		end
		Network.BroadcastGameConfig();
		BGD_Journal("signaux : images et sons ajoutés à la configuration, lancement");
	end

	-- Network est en lecture seule (« Attempt to modify read-only table ») : la globale Network de
	-- ce script est remplacée par une table qui renvoie tout au vrai Network, sauf LaunchGame. Les
	-- fonctions du salon la lisent à chaque appel. Un bloc rechargé repart du vrai Network.
	if type(Network) == "table" and type(Network.LaunchGame) == "function" then
		local ok, err = pcall(function()
			local vrai = Network.BGD_Vrai or Network;
			local avant = BGD_Protege(Ajouter);
			Network = setmetatable({
				BGD_Vrai = vrai,
				LaunchGame = function(...)
					avant();
					return vrai.LaunchGame(...);
				end,
			}, { __index = vrai });
		end);
		if not ok then BGD_Journal("signaux : Network.LaunchGame non enveloppé : " .. tostring(err)); end
	else
		BGD_Journal("signaux : Network.LaunchGame introuvable, images et sons jamais ajoutés");
	end

	-- Le « Prêt » gardé quand le seul changement de configuration est ce mod.
	if type(SetLocalReady) == "function" then
		local base = SetLocalReady;
		SetLocalReady = function(pret, ...)
			if pret == false and vu ~= nil then
				local maintenant = DansConfig();
				if maintenant ~= vu then
					vu = maintenant;
					BGD_Journal("signaux : « Prêt » gardé (images et sons " .. (maintenant and "ajoutés" or "retirés") .. ")");
					return;
				end
			end
			return base(pret, ...);
		end
	end
	-- Après le gestionnaire de StagingRoom.lua (ajouté avant ce bloc) : l'état vu suit la configuration
	-- même quand le salon est caché et n'appelle pas SetLocalReady.
	if Events ~= nil and LuaEvents ~= nil then
		Events.GameConfigChanged.Add(BGD_Protege(function() vu = DansConfig(); end));
		LuaEvents.HostGame_ShowStagingRoom.Add(BGD_Protege(Retirer));
		LuaEvents.JoiningRoom_ShowStagingRoom.Add(BGD_Protege(function() vu = DansConfig(); end));
	end
end
