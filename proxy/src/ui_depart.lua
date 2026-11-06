-- ===========================================================================
-- Civ6RolloverProxy (src/ui_depart.lua) : départ de tour simultané (depart_tour/README.md), ajouté
-- par le proxy de GameCore à la fin de InGame.lua (ou de son remplaçant du même nom, celui de MPH).
--
-- 1. Synchro de tour de MPH (CPL_SYNCTURN) coupée sur cette machine, toujours : elle ne
--    synchronise rien (chaque machine se libère à son propre début de tour) et masque l'écran.
-- 2. Si l'hôte a coché l'option de partie « No First/Last moves (BGD) » (BGD_DEPART_SIMULTANE) et
--    que tous les humains ont BetterGameDLLs à la même version (correctifs POL actifs) :
--    - à chaque fin de tour, le proxy est armé : il arrête la publication des événements GameCore
--      juste avant l'activation du tour local (LocalPlayerTurnBegin, PlayerTurnActivated).
--      L'interface reste dans le passage de tour, les ordres du joueur sont retenus et son
--      minuteur gelé. Sans ce détour, l'attente commence à LocalPlayerTurnBegin, interface ouverte ;
--    - dès que l'interface a rattrapé la simulation (publication arrêtée, ou premier
--      GameCoreEventPublishComplete qui suit le début du tour, ou fermeture de l'écran de
--      chargement), la machine envoie `.bgd_pret <tour>` à l'hôte ;
--    - l'hôte, quand tous les humains équipés et connectés sont prêts (lui compris), envoie
--      `.bgd_depart <tour>` à chacun et part ; chacun part à réception. Après DELAI_HOTE secondes,
--      l'hôte part sans les retardataires ;
--    - un joueur qui se reconnecte en plein tour se déclare prêt et reçoit aussitôt le départ.
--    Les signaux passent par le chat, en message privé, et ChatPanel.lua les cache
--    (ui_depart_chat.lua). Le proxy lève lui-même l'attente au bout de 120 s.
-- 3. Même option, mêmes conditions, et minuteur de l'exe détourné : quand le
--    minuteur de l'hôte arrive à zéro, il envoie `.bgd_fin <tour>` à chacun ; à réception, le proxy
--    pousse le minuteur local au-delà de sa limite et l'exe termine le tour du joueur à l'image
--    suivante, même s'il lui restait du temps. Chacun garde aussi son propre minuteur.
-- Fonctions natives : Civ6RolloverProxy_Depart* (proxy/src/depart.rs). Tout passe par pcall.
-- ===========================================================================
do
local BGD_DepartOk, BGD_DepartErr = pcall(function()
	if BGD_Depart ~= nil then
		return;
	end
	BGD_Depart = {};
	local D = BGD_Depart;

	local CLE_OPTION = "BGD_DEPART_SIMULTANE";
	local PRET, DEPART, FIN = ".bgd_pret ", ".bgd_depart ", ".bgd_fin ";
	local DELAI_HOTE = 60;

	local function J(texte)
		if Civ6RolloverProxy_DepartJournal ~= nil then
			Civ6RolloverProxy_DepartJournal(texte);
		end
	end

	-- Ligne locale dans le chat, que ChatPanel.lua affiche sans l'envoyer.
	local function Afficher(texte)
		LuaEvents.BGD_DepartTexte("[COLOR_Civ6Yellow]BGE - " .. texte .. "[ENDCOLOR]");
	end

	-- Réglage depart_signaux : signaux `.bgd_` et lignes de diagnostic visibles dans le chat.
	local function Signaux()
		return Civ6RolloverProxy_DepartSignaux ~= nil and Civ6RolloverProxy_DepartSignaux() == 1;
	end

	local function Protege(f)
		return function(...)
			local ok, err = pcall(f, ...);
			if not ok then
				J("erreur : " .. tostring(err));
			end
		end
	end

	-- 1. Synchro de tour de MPH ------------------------------------------------------------------
	local function CouperSynchroMPH()
		local v = GameConfiguration.GetValue("CPL_SYNCTURN");
		if v == true or v == 1 then
			GameConfiguration.SetValue("CPL_SYNCTURN", false);
			LuaEvents.InGame_CloseTurnProcessing();
			J("synchro de tour de MPH (CPL_SYNCTURN) désactivée");
		end
	end
	CouperSynchroMPH();
	Events.LoadScreenClose.Add(Protege(CouperSynchroMPH));

	-- 2. Départ simultané ------------------------------------------------------------------------
	local function OptionCochee()
		local v = GameConfiguration.GetValue(CLE_OPTION);
		return v == true or v == 1;
	end

	local function Reseau()
		return GameConfiguration.IsNetworkMultiplayer() and not GameConfiguration.IsPlayByCloud()
			and not GameConfiguration.IsHotseat();
	end

	local function Dispo()
		return Civ6RolloverProxy_DepartDispo ~= nil and Civ6RolloverProxy_DepartDispo() == 1;
	end

	local function Actif()
		return Reseau() and OptionCochee() and Dispo();
	end

	-- Le début du tour peut-il être retenu avant l'interface (publication des événements détournée) ?
	local function Retenue()
		if Civ6RolloverProxy_DepartDispo == nil or Civ6RolloverProxy_DepartArmer == nil then
			return false;
		end
		local _, _, publication = Civ6RolloverProxy_DepartDispo();
		return publication == 1;
	end

	-- La fin forcée passe par le minuteur de l'exe : il faut aussi son détour.
	local function FinActif()
		if not (Reseau() and OptionCochee()) or Civ6RolloverProxy_DepartDispo == nil then
			return false;
		end
		local dispo, minuteur = Civ6RolloverProxy_DepartDispo();
		return dispo == 1 and minuteur == 1;
	end

	local function Moi()
		return Network.GetLocalPlayerID();
	end

	local function Spectateur(id)
		local pc = PlayerConfigurations[id];
		return pc ~= nil and pc:GetLeaderTypeName() == "LEADER_SPECTATOR";
	end

	-- Version annoncée dans le salon : "<version>@<jeton de l'hôte>#<jeton>".
	local function Version(id)
		local pc = PlayerConfigurations[id];
		if pc == nil then return nil; end
		local v = pc:GetValue("BGD_VERSION");
		return string.match(tostring(v), "^([^@#:]+)@");
	end

	local function Nom(id)
		local pc = PlayerConfigurations[id];
		return pc and Locale.Lookup(pc:GetPlayerName()) or ("#" .. tostring(id));
	end

	-- Les humains attendus : vivants, pas spectateurs, connectés, et qui annoncent notre version.
	-- Un joueur sans BetterGameDLLs n'enverrait jamais son signal : on ne l'attend pas.
	local function Attendus()
		local moi, version = Moi(), Version(Moi());
		local t = {};
		for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
			local pc, p = PlayerConfigurations[id], Players[id];
			if pc ~= nil and pc:IsHuman() and not Spectateur(id) and p ~= nil and p:IsAlive()
				and (id == moi or Network.IsPlayerConnected(id)) and version ~= nil and Version(id) == version then
				table.insert(t, id);
			end
		end
		return t;
	end

	local function Envoyer(texte, id)
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_PLAYER, id);
	end

	D.tour = -1;          -- tour de la barrière en cours sur cette machine
	D.attente = false;    -- ordres retenus
	D.pret = false;       -- signal envoyé pour D.tour
	D.aEnvoyer = false;   -- signal à envoyer au prochain GameCoreEventPublishComplete
	D.chargement = true;  -- écran de chargement encore ouvert
	D.recu = -1;          -- dernier départ reçu de l'hôte
	D.prets = {};         -- hôte : D.prets[tour][id] = true
	D.parti = -1;         -- hôte : dernier tour lancé
	D.pretDepuis = nil;   -- hôte : os.clock() quand il a été prêt
	D.avisRejets = 0;     -- ordres retenus déjà signalés au joueur pour ce tour
	D.annonce = false;
	D.arme = false;       -- la prochaine activation du tour local sera retenue avant l'interface
	D.retenu = false;     -- activation retenue par le proxy, déjà prise en compte ici

	local Partir, Evaluer, SurRetenue;

	-- Armé, à chaque image : la publication s'est-elle arrêtée devant le début du tour ?
	-- En attente, deux fois par seconde : ordres retenus à signaler et délai de l'hôte.
	D.cumul = 0;
	local function Tic(dt)
		local okR, errR = pcall(function()
			if (D.arme or D.retenu) and Civ6RolloverProxy_DepartEtat ~= nil then
				local _, _, _, retenu = Civ6RolloverProxy_DepartEtat();
				if retenu == 1 and not D.retenu then
					D.retenu = true;
					D.arme = false;
					SurRetenue();
				elseif retenu ~= 1 then
					D.retenu = false;
				end
			end
		end);
		if not okR then
			J("erreur : " .. tostring(errR));
		end
		D.cumul = D.cumul + (tonumber(dt) or 0);
		if D.cumul < 0.5 then
			return;
		end
		D.cumul = 0;
		local ok, err = pcall(function()
			if D.attente and Civ6RolloverProxy_DepartEtat ~= nil then
				local enAttente, rejets = Civ6RolloverProxy_DepartEtat();
				if enAttente ~= 1 then
					-- Le proxy a levé l'attente de lui-même (délai de sécurité).
					D.attente = false;
				elseif rejets > D.avisRejets then
					if D.avisRejets == 0 then
						Afficher(BGD_Txt("LOC_BGD_DEPART_ORDRES_RETENUS"));
					end
					D.avisRejets = rejets;
				end
			end
			if Network.IsGameHost() and D.pret and D.parti < D.tour then
				Evaluer(D.tour);
			end
			if not D.attente and not D.arme and not D.retenu
				and not (Network.IsGameHost() and D.pret and D.parti < D.tour) then
				ContextPtr:ClearUpdate();
			end
		end);
		if not ok then
			J("erreur : " .. tostring(err));
			ContextPtr:ClearUpdate();
		end
	end

	Partir = function(raison)
		if D.attente then
			D.attente = false;
			if Civ6RolloverProxy_DepartPartir ~= nil then
				Civ6RolloverProxy_DepartPartir(raison);
			end
			if D.avisRejets > 0 then
				Afficher(BGD_Txt("LOC_BGD_DEPART_PARTI"));
			end
		end
	end

	local function Lancer(tour, absents)
		D.parti = tour;
		for _, id in ipairs(Attendus()) do
			if id ~= Moi() then
				Envoyer(DEPART .. tour, id);
			end
		end
		if absents ~= nil and #absents > 0 then
			local noms = {};
			for _, id in ipairs(absents) do table.insert(noms, Nom(id)); end
			Afficher(BGD_Txt("LOC_BGD_DEPART_SANS", table.concat(noms, ", "), DELAI_HOTE));
		end
		J("tour " .. tour .. " lancé" .. ((absents and #absents > 0) and (", sans " .. #absents .. " joueur(s)") or ""));
		Partir("départ de l'hôte, tour " .. tour);
	end

	-- Hôte : tous les attendus sont-ils prêts pour ce tour ?
	Evaluer = function(tour)
		if not Network.IsGameHost() or D.tour ~= tour or not D.pret or D.parti >= tour then
			return;
		end
		local prets = D.prets[tour] or {};
		local absents = {};
		for _, id in ipairs(Attendus()) do
			if id ~= Moi() and not prets[id] then
				table.insert(absents, id);
			end
		end
		if #absents == 0 then
			Lancer(tour);
		elseif D.pretDepuis ~= nil and os.clock() - D.pretDepuis >= DELAI_HOTE then
			Lancer(tour, absents);
		end
	end

	local function EnvoyerPret()
		if D.pret or D.chargement then
			return;
		end
		D.pret = true;
		if Network.IsGameHost() then
			D.pretDepuis = os.clock();
			Evaluer(D.tour);
		else
			Envoyer(PRET .. D.tour, Network.GetGameHostPlayerID());
		end
		J("prêt pour le tour " .. D.tour);
	end

	local function Entrer(tour)
		if D.tour == tour or not Actif() or Spectateur(Moi()) then
			return;
		end
		D.tour = tour;
		D.pret = false;
		D.pretDepuis = nil;
		D.avisRejets = 0;
		if D.recu >= tour or D.parti >= tour then
			return;  -- départ déjà donné (reconnexion après le départ, ou retard rattrapé)
		end
		local exec = PlayerOperations and PlayerOperations.EXECUTE_SCRIPT or 0;
		Civ6RolloverProxy_DepartAttendre(Moi(), exec);
		D.attente = true;
		D.aEnvoyer = true;
		ContextPtr:SetUpdate(Tic);
	end

	-- Arme la retenue du prochain début de tour (à chaque fin de tour, et au chargement).
	local function Armer()
		if not Actif() or not Retenue() or Spectateur(Moi()) then
			return;
		end
		local exec = PlayerOperations and PlayerOperations.EXECUTE_SCRIPT or 0;
		Civ6RolloverProxy_DepartArmer(Moi(), exec);
		D.arme = true;
		ContextPtr:SetUpdate(Tic);
	end

	-- La publication s'est arrêtée devant le début du tour local : l'interface a rattrapé la
	-- simulation, le proxy retient déjà les ordres et le minuteur. Prêt tout de suite.
	SurRetenue = function()
		local tour = Game.GetCurrentGameTurn();
		if D.recu >= tour or D.parti >= tour or not Actif() then
			Civ6RolloverProxy_DepartPartir("départ déjà donné, tour " .. tour);
			return;
		end
		if D.tour ~= tour then
			D.tour = tour;
			D.pret = false;
			D.pretDepuis = nil;
			D.avisRejets = 0;
		end
		D.attente = true;
		D.aEnvoyer = false;
		J("tour " .. tour .. " : début du tour retenu avant l'interface");
		EnvoyerPret();
	end

	local function OnLocalPlayerTurnBegin()
		if Civ6RolloverProxy_FinLever ~= nil then
			Civ6RolloverProxy_FinLever();
		end
		Entrer(Game.GetCurrentGameTurn());
	end

	-- 3. Fin simultanée ---------------------------------------------------------------------------
	-- Hôte : le minuteur n'est « armé » qu'après avoir été vu sous sa limite pendant le tour, pour
	-- qu'une mise à jour encore écoulée du tour précédent ne termine pas le nouveau. Un nouveau
	-- segment (congrès) remet le minuteur à zéro et le réarme.
	D.finArme = -1;      -- tour pour lequel le minuteur de l'hôte a été vu sous sa limite
	D.finRecue = -1;     -- dernier tour terminé à réception de la fin

	local function OnTurnTimerUpdated(ecoule, limite)
		if not Network.IsGameHost() or type(ecoule) ~= "number" or type(limite) ~= "number" or limite <= 0 then
			return;
		end
		local tour = Game.GetCurrentGameTurn();
		if ecoule < limite then
			D.finArme = tour;
			return;
		end
		if D.finArme ~= tour or not FinActif() then
			return;
		end
		D.finArme = -1;
		for _, id in ipairs(Attendus()) do
			if id ~= Moi() then
				Envoyer(FIN .. tour, id);
			end
		end
		J("tour " .. tour .. " : minuteur de l'hôte écoulé, fin envoyée");
	end

	local function RecevoirFin(tour)
		if tour ~= Game.GetCurrentGameTurn() or not FinActif() or Spectateur(Moi()) then
			return;
		end
		local p = Players[Moi()];
		if p == nil or not p:IsTurnActive() then
			return;
		end
		D.attente = false;
		if Civ6RolloverProxy_FinForcer() == 1 then
			D.finRecue = tour;
			if Signaux() then
				Afficher(BGD_Txt("LOC_BGD_DEPART_FIN_HOTE"));
			end
			J("tour " .. tour .. " : fin reçue, tour terminé");
		else
			J("tour " .. tour .. " : fin reçue, minuteur introuvable");
		end
	end

	local function OnPublishComplete()
		if D.aEnvoyer and not D.chargement then
			D.aEnvoyer = false;
			EnvoyerPret();
		end
	end

	local function Annoncer()
		if D.annonce or not Network.IsGameHost() or not Reseau() or not OptionCochee() then
			return;
		end
		D.annonce = true;
		local texte;
		if FinActif() then
			texte = "[COLOR_GREEN]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_NFLM_ACTIF") .. "[ENDCOLOR]";
		elseif Dispo() then
			texte = "[COLOR_GREEN]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_NFLM_DEPART_SEUL") .. "[ENDCOLOR]";
		else
			texte = "[COLOR_RED]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_NFLM_DESACTIVE") .. "[ENDCOLOR]";
		end
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		J("annonce : " .. texte);
	end

	local function OnLoadScreenClose()
		D.chargement = false;
		Annoncer();
		local p = Players[Moi()];
		if p ~= nil and p:IsTurnActive() then
			Entrer(Game.GetCurrentGameTurn());
		end
		if D.attente and not D.pret then
			D.aEnvoyer = false;
			EnvoyerPret();
		end
		Armer();
	end

	local function OnChat(fromPlayer, toPlayer, text, eTargetType)
		if type(text) ~= "string" or string.sub(text, 1, 5) ~= ".bgd_" then
			return;
		end
		if string.sub(text, 1, #PRET) == PRET then
			local tour = tonumber(string.sub(text, #PRET + 1));
			if tour == nil or not Network.IsGameHost() then return; end
			D.prets[tour] = D.prets[tour] or {};
			D.prets[tour][fromPlayer] = true;
			if D.parti >= tour then
				Envoyer(DEPART .. tour, fromPlayer);  -- reconnexion en plein tour, ou retardataire
			else
				Evaluer(tour);
			end
		elseif string.sub(text, 1, #DEPART) == DEPART then
			local tour = tonumber(string.sub(text, #DEPART + 1));
			if tour == nil or fromPlayer ~= Network.GetGameHostPlayerID() then return; end
			if tour > D.recu then D.recu = tour; end
			if D.tour == tour then
				Partir("départ reçu, tour " .. tour);
			end
		elseif string.sub(text, 1, #FIN) == FIN then
			local tour = tonumber(string.sub(text, #FIN + 1));
			if tour == nil or fromPlayer ~= Network.GetGameHostPlayerID() then return; end
			RecevoirFin(tour);
		end
	end

	-- Un joueur part : il n'est plus attendu.
	local function OnDepartJoueur()
		if Network.IsGameHost() then
			Evaluer(D.tour);
		end
	end

	-- Migration d'hôte : le nouvel hôte n'a rien reçu. Chacun lui renvoie son signal.
	local function OnHoteMigre()
		if not D.pret or D.recu >= D.tour then
			return;
		end
		if Network.IsGameHost() then
			D.pretDepuis = os.clock();
			D.prets[D.tour] = D.prets[D.tour] or {};
			ContextPtr:SetUpdate(Tic);
			Evaluer(D.tour);
		else
			Envoyer(PRET .. D.tour, Network.GetGameHostPlayerID());
		end
	end

	if Reseau() then
		Events.LocalPlayerTurnBegin.Add(Protege(OnLocalPlayerTurnBegin));
		-- Distribué par la publication détournée elle-même : armé à temps même si le début du tour
		-- suit dans la même série d'événements.
		Events.TurnEnd.Add(Protege(Armer));
		Events.GameCoreEventPublishComplete.Add(Protege(OnPublishComplete));
		Events.LoadScreenClose.Add(Protege(OnLoadScreenClose));
		Events.MultiplayerChat.Add(Protege(OnChat));
		Events.MultiplayerPostPlayerDisconnected.Add(Protege(OnDepartJoueur));
		Events.MultiplayerHostMigrated.Add(Protege(OnHoteMigre));
		Events.TurnTimerUpdated.Add(Protege(OnTurnTimerUpdated));
	end
end);
if not BGD_DepartOk then
	print("BGD depart echec " .. tostring(BGD_DepartErr));
	if Civ6RolloverProxy_DepartJournal ~= nil then
		Civ6RolloverProxy_DepartJournal("erreur au chargement : " .. tostring(BGD_DepartErr));
	end
end
end
