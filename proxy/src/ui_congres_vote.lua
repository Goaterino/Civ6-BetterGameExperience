-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_vote.lua), ajouté par le proxy à la fin de WorldCongressPopup.lua.
-- Analyse : congres/README.md.
-- 1. Envoi automatique : SetPhase(1) termine le tour du joueur quand la fenêtre ne trouve aucun choix.
--    En session ordinaire, il y a toujours des résolutions : une liste vide vient d'un cache pas encore
--    à jour. Le tour n'est alors pas terminé ; la fenêtre se remplit dès que les résolutions arrivent
--    (au plus BGD_ATTENTE_MAX secondes, puis comportement du jeu).
-- 2. Retour au vote : si le joueur annule sa fin de tour pendant le vote (bouton de l'écran
--    d'attente, ui_congres_attente.lua ; permis par le proxy de GameCore, src/congres.rs), la fenêtre
--    se rouvre avec les votes envoyés. GameCore rendra leur coût au nouveau vote : la faveur affichée
--    l'ajoute dès maintenant.
-- Ce bloc partage les locales du fichier (m_CurrentStage, m_kResolutionVotes…), auquel il est ajouté.
-- ===========================================================================
do
	local BGD_ATTENTE_MAX = 10;	-- secondes

	local function BGD_Cle()
		return tostring(Game.GetCurrentGameTurn()) .. ":" .. tostring(Game.GetCurrentTurnSegment());
	end

	local function BGD_StageEnVote()
		local segment = Game.GetCurrentTurnSegment();
		if segment == WORLD_CONGRESS_STAGE_1 then return 1; end
		if segment == WORLD_CONGRESS_STAGE_2 then return 2; end
		return nil;
	end

	local function BGD_Spectateur()
		local id = Game.GetLocalPlayer();
		local pConfig = id ~= nil and id >= 0 and PlayerConfigurations[id];
		return pConfig ~= nil and pConfig ~= false and pConfig:GetLeaderTypeName() == "LEADER_SPECTATOR";
	end

	-- 1. Envoi automatique ---------------------------------------------------
	local m_BGD_DansPhase1 = false;
	local m_BGD_Attente = nil;		-- clé du segment dont on attend les résolutions
	local m_BGD_Ecoule = 0;
	local m_BGD_Abandon = nil;		-- clé du segment pour lequel on a cessé d'attendre

	local function BGD_ResolutionsDuCache()
		local ok, t = pcall(function() return Game.GetWorldCongress():GetResolutions(Game.GetLocalPlayer()); end);
		if not ok or type(t) ~= "table" then return 0; end
		local n = 0;
		for i, _ in pairs(t) do
			if type(i) == "number" then n = n + 1; end
		end
		return n;
	end

	local function BGD_FinAttente()
		m_BGD_Attente = nil;
		ContextPtr:ClearUpdate();
	end

	local function BGD_Attendre(fDTime)
		if m_BGD_Attente ~= BGD_Cle() or m_CurrentStage ~= 1 or m_CurrentPhase ~= 1 or m_HasAccepted then
			BGD_FinAttente();
			return;
		end
		m_BGD_Ecoule = m_BGD_Ecoule + (fDTime or 0);
		local n = BGD_ResolutionsDuCache();
		if n == 0 and m_BGD_Ecoule < BGD_ATTENTE_MAX then
			return;
		end
		BGD_FinAttente();
		if n == 0 then
			m_BGD_Abandon = BGD_Cle();
			print("BGE congrès : aucune résolution après " .. BGD_ATTENTE_MAX .. " s, envoi automatique du jeu");
		else
			print("BGE congrès : " .. n .. " résolutions arrivées après " .. string.format("%.1f", m_BGD_Ecoule) .. " s, fenêtre remplie");
		end
		SetPhase(1);
	end

	local BGD_SetPhase = SetPhase;
	function SetPhase(phaseNum)
		m_BGD_DansPhase1 = phaseNum == 1;
		local ok, r = pcall(BGD_SetPhase, phaseNum);
		m_BGD_DansPhase1 = false;
		if not ok then error(r, 0); end
		return r;
	end

	local BGD_HasChoices = HasChoices;
	function HasChoices()
		local r = BGD_HasChoices();
		if r or not m_BGD_DansPhase1 or m_CurrentStage ~= 1 or not m_IsInSession or BGD_Spectateur()
			or m_BGD_Abandon == BGD_Cle() then
			return r;
		end
		-- Session ordinaire sans résolution affichée : pas de fin de tour, on attend le cache.
		if m_BGD_Attente ~= BGD_Cle() then
			m_BGD_Attente = BGD_Cle();
			m_BGD_Ecoule = 0;
			print("BGE congrès : aucune résolution dans le cache à l'ouverture (" .. BGD_ResolutionsDuCache() .. "), tour non terminé, attente");
			ContextPtr:SetUpdate(BGD_Attendre);
		end
		return true;
	end

	-- 2. Retour au vote --------------------------------------------------------
	local m_BGD_Envoi = nil;		-- votes envoyés au dernier OnAccept
	local m_BGD_Rendu = 0;			-- faveur que GameCore rendra au nouveau vote
	local m_BGD_RenduCle = nil;

	local BGD_UpdateWorkingFavor = UpdateWorkingFavor;
	function UpdateWorkingFavor(...)
		if m_BGD_Rendu > 0 and m_BGD_RenduCle == BGD_Cle() and m_IsInSession then
			local pPlayer = Players[Game.GetLocalPlayer()];
			if pPlayer ~= nil then
				m_StartingFavor = pPlayer:GetFavor() + m_BGD_Rendu;
			end
		end
		return BGD_UpdateWorkingFavor(...);
	end

	local BGD_OnAccept = OnAccept;
	function OnAccept()
		local stage = m_CurrentStage;
		if (stage == 1 or stage == 2) and not GameConfiguration.IsPaused() and not BGD_Spectateur() then
			m_BGD_Envoi = {
				cle = BGD_Cle(),
				stage = stage,
				resolutions = m_kResolutionVotes,
				choix = m_kResolutionChoices,
				propositions = m_kProposalVotes,
				depense = math.max(0, (m_StartingFavor or 0) - (m_WorkingFavor or 0)),
			};
			m_BGD_Rendu = 0;
		end
		BGD_OnAccept();
	end

	local function BGD_OnLocalPlayerTurnUnready()
		local stage = BGD_StageEnVote();
		local e = m_BGD_Envoi;
		if stage == nil or e == nil or e.cle ~= BGD_Cle() or e.stage ~= stage then
			return;
		end
		m_BGD_Envoi = nil;
		m_kResolutionVotes = e.resolutions or {};
		m_kResolutionChoices = e.choix or {};
		m_kProposalVotes = e.propositions or {};
		m_BGD_Rendu = e.depense;
		m_BGD_RenduCle = e.cle;
		m_CurrentPhase = 0;
		print("BGE congrès : fin de tour annulée, retour au vote (session " .. stage .. ", " .. e.depense .. " de faveur à rendre)");
		LuaEvents.WorldCongressPopup_HideWorldCongressBetweenTurns();
		SetupWorldCongress(stage, true);
	end

	if GameCapabilities.HasCapability("CAPABILITY_WORLD_CONGRESS") then
		Events.LocalPlayerTurnUnready.Add(BGD_OnLocalPlayerTurnUnready);
	end
end
