-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_vote.lua), ajouté par le proxy à la fin de WorldCongressPopup.lua.
-- Analyse : congres/README.md.
-- Retour au vote : si le joueur annule sa fin de tour pendant le vote (bouton de l'écran
-- d'attente, ui_congres_attente.lua ; permis par le proxy de GameCore, src/congres.rs), la fenêtre
-- se rouvre avec les votes envoyés. GameCore rendra leur coût au nouveau vote : la faveur affichée
-- l'ajoute dès maintenant.
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
