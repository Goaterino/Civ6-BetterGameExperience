-- ===========================================================================
-- Civ6RolloverProxy (src/ui_panneau_ville.lua), ajouté par le proxy à la fin de CityPanel.lua
-- (inclus par CityPanel_Expansion1/2) : le panneau de la ville sélectionnée (en bas à droite) se
-- redessine sur CityProductionChanged, CityProductionUpdated et CityProductionCompleted, mais pas
-- sur CityProductionQueueChanged. Retirer le seul élément de la file (clic droit, corbeille) ne
-- déclenche que ce dernier : le panneau gardait la production retirée. On le redessine aussi sur
-- cet événement, pour la ville sélectionnée seulement (RefreshIfMatch).
-- ===========================================================================
do
	if BGD_PanneauVille_Etat == nil and type(RefreshIfMatch) == "function" then
		BGD_PanneauVille_Etat = true;
		Events.CityProductionQueueChanged.Add(function(playerID, cityID)
			RefreshIfMatch(playerID, cityID);
		end);
	end
end
