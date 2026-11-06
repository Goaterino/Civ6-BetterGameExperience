-- ===========================================================================
-- Civ6RolloverProxy (src/ui_entree.lua), inséré par le proxy avant OnInputHandler dans
-- ActionPanel.lua. OnInputHandler teste Entrée en dur et lance DoEndTurn (« Action suivante »), en
-- plus de l'action configurable EndTurn : effacer la touche dans les options ne l'arrêtait pas.
-- Entrée ne fait plus l'action suivante que si l'un des deux emplacements de l'action EndTurn porte
-- Entrée seule. Maj+Entrée (fin de tour forcée) reste en dur : il n'a pas d'action réglable.
-- Voir analyses/raccourci_entree.
-- ===========================================================================
local BGD_EntreeNotee = false;
function BGD_EntreeEstActionSuivante()
	local id = Input.GetActionId("EndTurn");
	if id == nil then return true; end		-- action absente : comportement du jeu
	local entree = Locale.Lookup("LOC_OPTIONS_KEY_RETURN");
	local g0 = Input.GetGestureDisplayString(id, 0);
	local g1 = Input.GetGestureDisplayString(id, 1);
	if not BGD_EntreeNotee then
		BGD_EntreeNotee = true;
		print("Civ6RolloverProxy : Action suivante = [" .. tostring(g0) .. "] [" .. tostring(g1) .. "], Entrée = [" .. tostring(entree) .. "]");
	end
	return g0 == entree or g1 == entree;
end

