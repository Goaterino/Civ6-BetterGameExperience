-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_fin_auto.lua), ajouté par le proxy à la fin de
-- ActionPanel_Expansion2.lua. La fin de tour automatique (option du joueur) ne doit pas partir
-- pendant le vote du congrès. Le jeu le vérifie par m_CongressIsInSession, relu seulement au début
-- du tour (Events.TurnBegin), avant le segment du congrès : la valeur peut être périmée pendant le
-- vote. On relit l'état à chaque vérification, et le segment courant en plus.
-- Analyse : congres/README.md (H2c).
-- ===========================================================================
do
	local BGD_WC1 = DB.MakeHash("TURNSEG_WORLDCONGRESS_1");
	local BGD_WC2 = DB.MakeHash("TURNSEG_WORLDCONGRESS_2");

	function AllowAutoEndTurn_Expansion()
		local segment = Game.GetCurrentTurnSegment();
		if segment == BGD_WC1 or segment == BGD_WC2 then
			return false;
		end
		local pWorldCongress = Game.GetWorldCongress();
		if pWorldCongress ~= nil and pWorldCongress:IsInSession() then
			return false;
		end
		if XP1_AllowAutoEndTurn_Expansion ~= nil then
			return XP1_AllowAutoEndTurn_Expansion();
		end
		return true;
	end
end
