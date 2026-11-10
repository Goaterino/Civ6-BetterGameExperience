-- ===========================================================================
-- Civ6RolloverProxy (src/ui_production.lua), ajouté par le proxy à la fin de ProductionPanel.lua :
-- les variables locales du fichier restent visibles.
-- La production en cours (CurrentProductionContainer) et le message « aucune production »
-- (NoProductionContainer) ne sont montrés ou cachés qu'au changement d'onglet et, file ouverte, à
-- CityProductionQueueChanged. Un changement de ville passe par Refresh puis View, qui remplit la
-- production en cours sans toucher à sa visibilité : après une ville sans production, celle de la
-- ville suivante restait cachée (et inversement, l'ancienne restait affichée). Après View, on
-- applique la règle des onglets : file (OnTabChangeQueue) ou production (OnTabChangeProduction).
-- ===========================================================================
do
	local View_jeu = View;
	function View(data)
		View_jeu(data);
		if m_isQueueOpen then
			Controls.CurrentProductionContainer:SetHide(not m_hasProductionToShow);
			Controls.NoProductionContainer:SetHide(m_hasProductionToShow);
		elseif m_CurrentListMode == LISTMODE.PRODUCTION then
			Controls.CurrentProductionContainer:SetHide(not m_hasProductionToShow);
		end
	end
end

-- ===========================================================================
-- Clic droit sur la production en cours, onglet Production (BET-13). Le jeu branche déjà le clic
-- droit du bouton sur RemoveQueueItem(0), mais OnTabChangeProduction désactive le bouton, qui ne
-- reçoit alors plus aucun clic : le retrait ne marchait que file ouverte. On réactive le bouton
-- après chaque passage sur l'onglet Production. L'onglet garde sa propre copie de la fonction
-- (TabSupport.lua, CallbackFunc, rappelée par SelectTab et par le clic) : c'est elle qu'on
-- enveloppe. Le clic gauche, qui sélectionne l'élément pour un échange, ne fait rien hors de la
-- file, comme quand le bouton était désactivé.
-- ===========================================================================
do
	local Production_Ok, Production_Err = pcall(function()
		local tab = m_productionTab;
		if tab == nil or type(tab["CallbackFunc"]) ~= "function" then
			print("Production : onglet introuvable, clic droit sur la production en cours inchangé");
			return;
		end
		local Onglet_jeu = tab["CallbackFunc"];
		tab["CallbackFunc"] = function(...)
			Onglet_jeu(...);
			if not m_isQueueOpen then
				Controls.CurrentProductionButton:SetDisabled(false);
			end
		end
		tab:RegisterCallback(Mouse.eLClick, tab["CallbackFunc"]);
		Controls.CurrentProductionButton:RegisterCallback(Mouse.eLClick, function()
			if m_isQueueOpen then
				OnItemClicked(Controls, Controls.CurrentProductionButton);
			end
		end);
		-- Le panneau est déjà sur l'onglet Production (Initialize l'a sélectionné avant ce bloc).
		if not m_isQueueOpen and m_CurrentListMode == LISTMODE.PRODUCTION then
			Controls.CurrentProductionButton:SetDisabled(false);
		end
	end);
	if not Production_Ok then
		print("Production echec " .. tostring(Production_Err));
	end
end
