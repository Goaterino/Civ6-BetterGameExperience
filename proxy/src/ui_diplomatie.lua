-- ===========================================================================
-- Civ6RolloverProxy (src/ui_diplomatie.lua), ajouté par le proxy à la fin de DiplomacyActionView.lua
-- (inclus par XP1, XP2, MPH et BSM) : une erreur à l'ouverture de l'écran diplomatique ne laisse plus
-- l'interface masquée.
-- OnOpenDiplomacyActionView appelle d'abord InitializeView : interface masquée (BulkHide Diplomacy),
-- contexte d'entrée Diplomacy empilé, événements du moteur retenus (UI.ReferenceCurrentEvent).
-- L'écran ne s'affiche qu'une fois le dirigeant chargé, après SelectPlayer. Une erreur entre les deux
-- (rapport BGE-DB5687 : LeaderIcon.lua:143 sur une équipe absente de Teams) laisse l'écran caché
-- mais initialisé. Les fermetures de fin et de début de tour ne regardent que l'écran visible, donc
-- l'interface ne revient plus. En cas d'erreur, on referme l'écran comme le bouton Fermer : Close
-- rend l'interface, dépile le contexte et libère les événements.
-- Dans une fonction : les locales du bloc ne comptent pas dans les registres du fichier.
-- ===========================================================================
(function()
	local BGD_Ouvrir = OnOpenDiplomacyActionView;
	if BGD_Ouvrir == nil then
		return;
	end
	OnOpenDiplomacyActionView = function(...)
		local ok, err = pcall(BGD_Ouvrir, ...);
		if not ok then
			print("BGD diplomatie : ouverture en échec, écran refermé : " .. tostring(err));
			if ms_bIsViewInitialized then
				local okClose, errClose = pcall(Close);
				if not okClose then
					print("BGD diplomatie : fermeture en échec : " .. tostring(errClose));
				end
			end
		end
	end
end)();
