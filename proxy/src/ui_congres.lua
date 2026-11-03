-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres.lua), ajouté par le proxy à la fin de WorldCongressPopup.lua.
-- Spectateurs de BSM (leader LEADER_SPECTATOR), si les correctifs POL sont actifs
-- (Civ6RolloverProxy_Pol, posée par le proxy) : GameCore refuse leurs votes (src/spectateurs.rs).
-- Ici, la fenêtre du congrès reste consultable pendant toute la session, en lecture seule :
-- - boutons de vote sans effet, navigation libre entre les résolutions et le récapitulatif ;
-- - leur tour du congrès est soumis dès qu'il est actif, sans vote : la session ne les attend pas ;
-- - la fin de leur tour ne ferme pas la fenêtre, le bouton du congrès la rouvre ;
-- - le bouton de validation devient « Fermer ».
-- Ce bloc partage les locales du fichier (m_CurrentStage, m_kResolutionVotes…), auquel il est ajouté.
-- ===========================================================================
do
	local function BGD_EstSpectateur()
		if Civ6RolloverProxy_Pol == nil or Civ6RolloverProxy_Pol() ~= 1 then
			return false;
		end
		local id = Game.GetLocalPlayer();
		local pConfig = id ~= nil and id >= 0 and PlayerConfigurations[id];
		return pConfig ~= nil and pConfig ~= false and pConfig:GetLeaderTypeName() == "LEADER_SPECTATOR";
	end

	local function BGD_FR()
		local language = Locale.GetCurrentLanguage and Locale.GetCurrentLanguage();
		local code = language and language.Type;
		return type(code) == "string" and string.sub(string.lower(code), 1, 2) == "fr";
	end

	local function BGD_Texte(fr, en)
		return BGD_FR() and fr or en;
	end

	-- Segment de vote (session ordinaire ou extraordinaire), et sa clé pour ce tour.
	local function BGD_EnVote()
		local segment = Game.GetCurrentTurnSegment();
		return segment == WORLD_CONGRESS_STAGE_1 or segment == WORLD_CONGRESS_STAGE_2;
	end
	local function BGD_Cle()
		return tostring(Game.GetCurrentGameTurn()) .. ":" .. tostring(Game.GetCurrentTurnSegment());
	end

	local m_BGD_Soumis = nil;	-- clé du segment déjà soumis
	local m_BGD_Vu = nil;		-- clé du segment dont les données de vote ont été remises à zéro

	-- Soumet le tour du congrès du spectateur, une fois par segment, quand son tour est actif.
	local function BGD_Soumettre()
		if not BGD_EstSpectateur() or not BGD_EnVote() then
			return;
		end
		local id = Game.GetLocalPlayer();
		local pPlayer = Players[id];
		if pPlayer == nil or not pPlayer:IsTurnActive() or m_BGD_Soumis == BGD_Cle() then
			return;
		end
		m_BGD_Soumis = BGD_Cle();
		UI.RequestPlayerOperation(id, PlayerOperations.WORLD_CONGRESS_SUBMIT_TURN, {});
		UI.RequestAction(ActionTypes.ACTION_ENDTURN);
	end

	local function BGD_Neutre()
		return function() end;
	end

	local BGD_OnVoteResolution = OnVoteResolution;
	function OnVoteResolution(...)
		if BGD_EstSpectateur() then return BGD_Neutre(); end
		return BGD_OnVoteResolution(...);
	end

	local BGD_OnVoteProposal = OnVoteProposal;
	function OnVoteProposal(...)
		if BGD_EstSpectateur() then return BGD_Neutre(); end
		return BGD_OnVoteProposal(...);
	end

	local BGD_OnToggleProposal = OnToggleProposal;
	function OnToggleProposal(...)
		if BGD_EstSpectateur() and m_IsInSession then return BGD_Neutre(); end
		return BGD_OnToggleProposal(...);
	end

	local BGD_CanMoveToNextPhase = CanMoveToNextPhase;
	function CanMoveToNextPhase()
		if BGD_EstSpectateur() and m_IsInSession then
			return not GameConfiguration.IsPaused() and m_CurrentPhase < PHASE_STEP_MAX;
		end
		return BGD_CanMoveToNextPhase();
	end

	local BGD_CanSubmit = CanSubmit;
	function CanSubmit()
		if BGD_EstSpectateur() and m_IsInSession then return true; end
		return BGD_CanSubmit();
	end

	-- Nouveau segment de vote : données remises à zéro, comme la fin de tour le fait pour un votant.
	local BGD_SetStage = SetStage;
	function SetStage(stageNum, beginCongress)
		if BGD_EstSpectateur() and (stageNum == 1 or stageNum == 2) and m_BGD_Vu ~= BGD_Cle() then
			m_BGD_Vu = BGD_Cle();
			m_kProposalVotes = nil;
			m_kResolutionVotes = nil;
			m_kResolutionChoices = nil;
		end
		return BGD_SetStage(stageNum, beginCongress);
	end

	local BGD_SetPhase = SetPhase;
	function SetPhase(phaseNum)
		local shown = BGD_SetPhase(phaseNum);
		if shown and BGD_EstSpectateur() and m_IsInSession then
			Controls.Description:SetText(Controls.Description:GetText() .. "[NEWLINE][COLOR_RED]"
				.. BGD_Texte("Spectateur : consultation seule, vos votes ne comptent pas.",
					"Spectator: read only, your votes are not counted.") .. "[ENDCOLOR]");
			BGD_Soumettre();
		end
		return shown;
	end

	local BGD_UpdateNavButtons = UpdateNavButtons;
	function UpdateNavButtons()
		BGD_UpdateNavButtons();
		if BGD_EstSpectateur() and m_IsInSession and Controls.AcceptButton:IsVisible() then
			Controls.AcceptButton:SetText(BGD_Texte("Fermer", "Close"));
			Controls.AcceptButton:SetDisabled(false);
		end
	end

	-- Le bouton de validation ferme : le tour est déjà soumis, sans vote.
	local BGD_OnAccept = OnAccept;
	function OnAccept()
		if BGD_EstSpectateur() and m_IsInSession then
			BGD_Soumettre();
			ClosePopup();
			return;
		end
		BGD_OnAccept();
	end

	-- Rouvrir pendant le vote : la fenêtre en lecture seule, plutôt que l'écran d'attente.
	local BGD_OnResumeCongress = OnResumeCongress;
	function OnResumeCongress()
		if BGD_EstSpectateur() and BGD_EnVote() then
			local stage = Game.GetCurrentTurnSegment() == WORLD_CONGRESS_STAGE_1 and 1 or 2;
			SetupWorldCongress(stage, true);
			return;
		end
		BGD_OnResumeCongress();
	end

	-- La fin du tour (soumis d'office) ne ferme pas la fenêtre du spectateur pendant le vote.
	local BGD_OnLocalPlayerTurnEnd = OnLocalPlayerTurnEnd;
	local function BGD_FinDeTour()
		if BGD_EstSpectateur() and BGD_EnVote() then
			return;
		end
		BGD_OnLocalPlayerTurnEnd();
	end
	pcall(function() Events.LocalPlayerTurnEnd.Remove(BGD_OnLocalPlayerTurnEnd); end);
	OnLocalPlayerTurnEnd = BGD_FinDeTour;
	if GameCapabilities.HasCapability("CAPABILITY_WORLD_CONGRESS") then
		Events.LocalPlayerTurnEnd.Add(BGD_FinDeTour);
		Events.WorldCongressStage1.Add(function() BGD_Soumettre(); end);
		Events.WorldCongressStage2.Add(function() BGD_Soumettre(); end);
		Events.LocalPlayerTurnBegin.Add(function() BGD_Soumettre(); end);
	end
end
