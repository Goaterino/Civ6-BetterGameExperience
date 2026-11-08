
-- ===========================================================================
-- Civ6RolloverProxy (src/ui_emissaires.lua), ajouté par le proxy à la fin de CityStates.lua (jeu
-- ou Better Spectator Mod) : les variables locales du fichier restent visibles.
-- Les émissaires en attente (m_kEnvoyChanges, pas encore envoyés à la partie) étaient effacés par
-- chaque Refresh, et l'écran se rafraîchit à chaque émissaire, quête, guerre ou paix de n'importe
-- quel joueur. Ils ne sont plus retirés que pour une cité-État détruite ou à laquelle l'équipe du
-- joueur local déclare la guerre. Une cité-État qui entre en guerre contre le joueur par son
-- suzerain garde ses émissaires en attente : ils restent affichés, ne partent pas à la confirmation
-- tant qu'elle les refuse, et le joueur peut les retirer. Voir analyses/emissaires_reinit.
-- Fermer l'écran (Close) vide toujours tout, comme dans le jeu.
-- ===========================================================================
do
	-- Les gestionnaires sont branchés par Initialize, qui ne fait rien sans l'écran des cités-États.
	local branche = m_kScreenSlideAnim ~= nil;

	local function PeutRecevoir(iCityState)
		local pLocal = Players[Game.GetLocalPlayer()];
		local pInfluence = pLocal ~= nil and pLocal:GetInfluence() or nil;
		return pInfluence ~= nil and pInfluence:CanGiveTokensToPlayer(iCityState);
	end

	-- Émissaires en attente qu'on peut envoyer maintenant.
	local function SommeEnvoyable()
		local sum = 0;
		for iPlayer, n in pairs(m_kEnvoyChanges) do
			if PeutRecevoir(iPlayer) then
				sum = sum + n;
			end
		end
		return sum;
	end

	-- GetData remet m_kEnvoyChanges à {} : on reprend les choix d'avant, sauf pour une cité-État
	-- détruite, et sans dépasser les jetons restants.
	local GetData_jeu = GetData;
	function GetData()
		local kPending = m_kEnvoyChanges;
		GetData_jeu();
		if kPending == nil or m_kPlayerData == nil or m_kPlayerData.EnvoyTokens == nil then
			return;
		end
		local left = m_kPlayerData.EnvoyTokens;
		for iPlayer, n in pairs(kPending) do
			local kCityState = m_kCityStates[iPlayer];
			if n > 0 and left > 0 and kCityState ~= nil and kCityState.isAlive then
				local k = math.min(n, left);
				m_kEnvoyChanges[iPlayer] = k;
				left = left - k;
			end
		end
	end

	-- AddCityStateRow n'affiche que les émissaires déjà posés : on y ajoute ceux en attente.
	local ViewList_jeu = ViewList;
	function ViewList()
		ViewList_jeu();
		if m_kPlayerData == nil or m_kPlayerData.EnvoyTokens == nil
			or m_mode ~= MODE.SendEnvoys or next(m_kEnvoyChanges) == nil then
			return;
		end
		for iPlayer, n in pairs(m_kEnvoyChanges) do
			local kInst = m_uiCityStateRows[iPlayer];
			if kInst ~= nil and m_kCityStates[iPlayer] ~= nil then
				RealizeEnvoyToken(m_kCityStates[iPlayer].Tokens + n, kInst.EnvoyCount);
			end
		end
		RealizeListHeader();
	end

	-- Le jeu grise « − » chez une cité-État qui refuse les émissaires : on le laisse actif s'il y en
	-- a en attente, pour pouvoir les retirer. « Confirmer » ne s'active que s'il y a de quoi envoyer.
	local RealizeEnvoyChangeButtons_jeu = RealizeEnvoyChangeButtons;
	function RealizeEnvoyChangeButtons()
		RealizeEnvoyChangeButtons_jeu();
		for iPlayer, inst in pairs(m_uiCityStateRows) do
			local amount = m_kEnvoyChanges[iPlayer];
			if amount ~= nil and amount > 0 then
				inst.EnvoyLessButton:SetDisabled(not m_isLocalPlayerTurn);
			end
		end
		Controls.ConfirmButton:SetDisabled(not m_isLocalPlayerTurn or SommeEnvoyable() == 0);
	end

	-- N'envoie que vers les cités-États qui acceptent ; les autres restent en attente.
	local OnConfirmPlacement_jeu = OnConfirmPlacement;
	function OnConfirmPlacement()
		local kBloques = {};
		for iPlayer, n in pairs(m_kEnvoyChanges) do
			if not PeutRecevoir(iPlayer) then
				kBloques[iPlayer] = n;
			end
		end
		for iPlayer in pairs(kBloques) do
			m_kEnvoyChanges[iPlayer] = nil;
		end
		OnConfirmPlacement_jeu();
		if next(kBloques) ~= nil and not ContextPtr:IsHidden() then
			m_kEnvoyChanges = kBloques;
			RealizeEnvoyChangeButtons();
			RealizeListHeader();
		end
	end

	-- Le jeu vidait tout dès que le joueur local entrait en guerre ou en paix avec qui que ce soit.
	-- On ne retire que la cité-État visée par une déclaration de l'équipe du joueur local
	-- (déclarant = premier argument). Celle d'une cité-État qui suit son suzerain a la cité-État pour
	-- déclarant : rien n'est retiré.
	local function OnDiplomacyDeclareWar_BGE(actingPlayer, reactingPlayer)
		local localPlayerID = Game.GetLocalPlayer();
		if localPlayerID ~= -1 and m_kEnvoyChanges[reactingPlayer] ~= nil then
			local pActing = Players[actingPlayer];
			local pLocal = Players[localPlayerID];
			if pActing ~= nil and pLocal ~= nil and pActing:GetTeam() == pLocal:GetTeam() then
				m_kEnvoyChanges[reactingPlayer] = nil;
			end
		end
		if not ContextPtr:IsHidden() then
			Refresh();
		end
	end

	local function OnDiplomacyMakePeace_BGE()
		if not ContextPtr:IsHidden() then
			Refresh();
		end
	end

	if branche then
		Events.DiplomacyDeclareWar.Remove(OnDiplomacyDeclareWar);
		Events.DiplomacyMakePeace.Remove(OnDiplomacyMakePeace);
		OnDiplomacyDeclareWar = OnDiplomacyDeclareWar_BGE;
		OnDiplomacyMakePeace = OnDiplomacyMakePeace_BGE;
		Events.DiplomacyDeclareWar.Add(OnDiplomacyDeclareWar);
		Events.DiplomacyMakePeace.Add(OnDiplomacyMakePeace);
		Controls.ConfirmButton:RegisterCallback(Mouse.eLClick, OnConfirmPlacement);
	end
end
