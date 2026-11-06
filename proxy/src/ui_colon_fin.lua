-- ===========================================================================
-- Civ6RolloverProxy (src/ui_colon_fin.lua), ajouté par le proxy à la fin de MinimapPanel.lua : les
-- locales du fichier restent visibles.
-- Lentille du colon vide : OnCityAddedToMap vidait la couche de l'eau à chaque ville fondée (par
-- n'importe quel joueur) sans la recolorier ; le proxy y lève m_refreshLens. Sélectionner une
-- unité recolorie aussi la couche si elle est allumée : ToggleLayerOn sur une couche déjà affichée
-- n'envoie pas LensLayerOn, donc rien ne la recalculait. Voir analyses/filtre_colon.
-- ===========================================================================
Events.UnitSelectionChanged.Add(function(playerID, unitID, hexI, hexJ, hexK, isSelected, isEditable)
	if isSelected and playerID == Game.GetLocalPlayer() and UILens.IsLayerOn(m_HexColoringWaterAvail) then
		SetWaterHexes();
	end
end);
