-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_attente.lua), ajouté par le proxy à la fin de
-- WorldCongressBetweenTurns.lua, l'écran d'attente qui suit le vote. Le bouton BGD_Revoter (ajouté
-- à WorldCongressBetweenTurns.xml par le proxy) annule la fin de tour du joueur pour revenir au vote.
-- Il n'est montré que si le jeu l'acceptera :
-- - le proxy de GameCore le permet (Civ6RolloverProxy_Revote, posée par le proxy : détour posé et
--   correctifs POL actifs, src/congres.rs), pendant un segment de vote ;
-- - Player:CanUnreadyTurn() : tour fini, au moins un autre joueur encore en train de voter ;
-- - il reste du temps au minuteur du joueur (l'exe refuserait sinon).
-- La fenêtre de vote se rouvre d'elle-même (ui_congres_vote.lua). Un autre joueur qui revient au
-- vote repasse « en attente » dans la liste.
-- Ce bloc partage les locales du fichier (m_kPlayerStages, m_uiPlayersByID…), auquel il est ajouté.
-- ===========================================================================
do
	local BGD_WC1 = DB.MakeHash("TURNSEG_WORLDCONGRESS_1");
	local BGD_WC2 = DB.MakeHash("TURNSEG_WORLDCONGRESS_2");

	local function BGD_Permis()
		if Civ6RolloverProxy_Revote == nil or Civ6RolloverProxy_Revote() ~= 1 then
			return false;
		end
		local segment = Game.GetCurrentTurnSegment();
		if segment ~= BGD_WC1 and segment ~= BGD_WC2 then
			return false;
		end
		local id = Game.GetLocalPlayer();
		local pPlayer = id ~= nil and id >= 0 and Players[id];
		local pConfig = pPlayer and PlayerConfigurations[id];
		if not pPlayer or (pConfig and pConfig:GetLeaderTypeName() == "LEADER_SPECTATOR") then
			return false;
		end
		return not pPlayer:IsTurnActive() and pPlayer:CanUnreadyTurn() and not UI.IsTurnTimerElapsed(id);
	end

	local function BGD_Maj()
		local bouton = Controls.BGD_Revoter;
		if bouton == nil then
			return;
		end
		bouton:SetHide(not (ContextPtr:IsVisible() and BGD_Permis()));
	end

	local function BGD_Revoter()
		if BGD_Permis() then
			UI.PlaySound("Play_UI_Click");
			UI.RequestAction(ActionTypes.ACTION_UNREADYTURN);
		end
		BGD_Maj();
	end

	-- Un joueur (local ou non) revenu au vote : de nouveau « en attente ».
	local function BGD_EnAttente(playerID)
		if playerID == nil or playerID < 0 then
			return;
		end
		m_kPlayerStages[playerID] = -1;
		local instance = m_uiPlayersByID[playerID];
		if instance then
			instance.BG:SetColor(COLOR_WAITING);
			instance.Label:SetText(LOC_PROPOSAL_WAITING);
		end
		UpdateCanContinue();
		BGD_Maj();
	end

	local function BGD_Ajouter(nom, f)
		pcall(function() Events[nom].Add(f); end);
	end

	if GameCapabilities.HasCapability("CAPABILITY_WORLD_CONGRESS") and Controls.BGD_Revoter ~= nil then
		Controls.BGD_Revoter:SetText(BGD_Txt("LOC_BGD_CONGRES_REVOTER"));
		Controls.BGD_Revoter:SetToolTipString(BGD_Txt("LOC_BGD_CONGRES_REVOTER_AIDE"));
		Controls.BGD_Revoter:RegisterCallback(Mouse.eLClick, BGD_Revoter);
		Controls.BGD_Revoter:SetHide(true);
		LuaEvents.WorldCongressPopup_ShowWorldCongressBetweenTurns.Add(function() BGD_Maj(); end);
		BGD_Ajouter("TurnTimerUpdated", function() BGD_Maj(); end);
		BGD_Ajouter("RemotePlayerTurnEnd", function() BGD_Maj(); end);
		BGD_Ajouter("LocalPlayerTurnEnd", function() BGD_Maj(); end);
		BGD_Ajouter("RemotePlayerTurnUnready", BGD_EnAttente);
		BGD_Ajouter("LocalPlayerTurnUnready", function()
			if Game.GetLocalPlayer() >= 0 then m_kPlayerStages[Game.GetLocalPlayer()] = math.huge; end
			OnHide();
		end);
	end
end
