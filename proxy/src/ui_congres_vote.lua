-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_vote.lua), ajouté par le proxy à la fin de WorldCongressPopup.lua.
-- Analyse : congres/README.md.
-- Retour au vote : si le joueur annule sa fin de tour pendant le vote (bouton de l'écran
-- d'attente, ui_congres_attente.lua ; permis par le proxy de GameCore, src/congres.rs), la fenêtre
-- se rouvre avec les votes envoyés. GameCore rendra leur coût au nouveau vote : la faveur affichée
-- l'ajoute dès maintenant. Sans vote envoyé (tour parti au minuteur), elle se rouvre avec des votes
-- vides. Le bouton du congrès, pendant le vote, repeuple la fenêtre avec la session en cours, et
-- le récapitulatif ne s'ouvre plus sur des votes incomplets.
-- Ce bloc partage les locales du fichier (m_CurrentStage, m_kResolutionVotes…), auquel il est ajouté.
-- ===========================================================================
do
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

	-- Retour au vote ---------------------------------------------------------
	local m_BGD_Envoi = nil;		-- votes envoyés au dernier OnAccept
	local m_BGD_Rendu = 0;			-- faveur que GameCore rendra au nouveau vote
	local m_BGD_RenduCle = nil;
	local m_BGD_Rouvert = nil;		-- clé « tour:segment » du dernier retour au vote

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

	-- Le tour du congrès peut aussi être parti sans vote (minuteur de l'exe, BGE-9E21BE) : la fenêtre
	-- se rouvre alors avec des votes vides, rien à rendre.
	local function BGD_OnLocalPlayerTurnUnready()
		local stage = BGD_StageEnVote();
		if stage == nil or BGD_Spectateur() then
			return;
		end
		local e = m_BGD_Envoi;
		m_BGD_Envoi = nil;
		if e ~= nil and e.cle == BGD_Cle() and e.stage == stage then
			m_kResolutionVotes = e.resolutions or {};
			m_kResolutionChoices = e.choix or {};
			m_kProposalVotes = e.propositions or {};
			m_BGD_Rendu = e.depense;
			m_BGD_RenduCle = e.cle;
			print("BGE congrès : fin de tour annulée, retour au vote (session " .. stage .. ", " .. e.depense .. " de faveur à rendre)");
		else
			m_kResolutionVotes = {};
			m_kResolutionChoices = {};
			m_kProposalVotes = {};
			m_BGD_Rendu = 0;
			m_BGD_RenduCle = nil;
			print("BGE congrès : fin de tour annulée sans vote envoyé, retour au vote (session " .. stage .. ")");
		end
		m_CurrentPhase = 0;
		m_BGD_Rouvert = BGD_Cle();
		LuaEvents.WorldCongressPopup_HideWorldCongressBetweenTurns();
		SetupWorldCongress(stage, true);
	end

	-- L'annulation relance WorldCongressStage1 / 2 : l'original y rappelle SetupWorldCongress sans
	-- ouvrir la session, ce qui ferme la fenêtre rouverte. Après un retour au vote, elle reste
	-- ouverte (et repeuplée) ; sinon, comportement d'origine (l'intro ouvre la session).
	local BGD_OnWorldCongressStageChange = OnWorldCongressStageChange;
	function OnWorldCongressStageChange(playerID, stageNum, ...)
		local id = Game.GetLocalPlayer();
		local pPlayer = id ~= nil and id >= 0 and Players[id];
		if playerID == id and m_BGD_Rouvert == BGD_Cle() and BGD_StageEnVote() == stageNum
			and pPlayer and pPlayer:IsTurnActive() then
			SetupWorldCongress(stageNum, true);
			return;
		end
		return BGD_OnWorldCongressStageChange(playerID, stageNum, ...);
	end

	-- Rouvrir le congrès (bouton, notification) pendant le vote, tour actif : la fenêtre est
	-- repeuplée avec la session en cours. L'original remettait en file son contenu tel quel, celui
	-- de la dernière population : un joueur dont le tour était parti sans vote voyait le congrès
	-- précédent (BGE-9E21BE).
	local BGD_OnResumeCongress = OnResumeCongress;
	function OnResumeCongress(...)
		local stage = BGD_StageEnVote();
		local id = Game.GetLocalPlayer();
		local pPlayer = id ~= nil and id >= 0 and Players[id];
		if stage ~= nil and pPlayer and pPlayer:IsTurnActive() and not BGD_Spectateur()
			and not GameConfiguration.IsPaused() then
			if m_CurrentStage ~= stage then
				m_CurrentPhase = 0;
			end
			SetupWorldCongress(stage, true);
			return;
		end
		return BGD_OnResumeCongress(...);
	end

	-- Récapitulatif (phase 2) : chaque résolution doit avoir ses votes et son choix. Sinon (tables
	-- perdues entre la réouverture et « Suivant », T74 de BGE-9E21BE), PopulateSummary indexait nil :
	-- retour à la phase 1, qui recrée les votes manquants, comme le bouton « Précédent ».
	local function BGD_RecapComplet()
		if m_CurrentStage ~= 1 or BGD_Spectateur() then
			return true;
		end
		local votes = m_kResolutionVotes or {};
		local choix = m_kResolutionChoices or {};
		local kResolutions = Game.GetWorldCongress():GetResolutions(Game.GetLocalPlayer()) or {};
		for i, kResolutionData in pairs(kResolutions) do
			if type(i) == "number" then
				local kResolution = GameInfo.Resolutions[kResolutionData.Type];
				local kVoteData = votes[i];
				if kResolution == nil or kVoteData == nil or kVoteData.A == nil or kVoteData.B == nil
					or choix[kResolution.Hash] == nil then
					return false;
				end
			end
		end
		return true;
	end

	local BGD_PopulateSummary = PopulateSummary;
	function PopulateSummary(...)
		if not BGD_RecapComplet() then
			print("BGE congrès : votes incomplets au récapitulatif, retour aux résolutions");
			m_kResolutionVotes = m_kResolutionVotes or {};
			m_kResolutionChoices = m_kResolutionChoices or {};
			m_kProposalVotes = m_kProposalVotes or {};
			SetPhase(1);
			return;
		end
		return BGD_PopulateSummary(...);
	end

	if GameCapabilities.HasCapability("CAPABILITY_WORLD_CONGRESS") then
		Events.LocalPlayerTurnUnready.Add(BGD_OnLocalPlayerTurnUnready);
	end
end
