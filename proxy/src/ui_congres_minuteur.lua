-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_minuteur.lua), ajouté par le proxy à la fin de
-- TurnProcessing.lua de MPH. Le minuteur intelligent de MPH (SmartTimer) prévoit 240 s pour le vote
-- du congrès, mais ne les applique jamais : WORLD_CONGRESS_STAGE_1 et _2 n'y sont pas définies
-- (nil), et la valeur n'est calculée qu'à la fin du tour, avant le congrès. Le vote hérite du
-- minuteur du tour, parfois court. Au début d'un segment de vote, TURN_TIMER_TIME (réglage local,
-- que MPH écrit sur chaque machine de la même façon) passe à 240 s s'il est plus court ; MPH le
-- recalcule à la fin du tour, comme d'habitude. Analyse : congres/README.md (H2a).
-- ===========================================================================
do
	local BGD_MINUTEUR_CONGRES = 240;
	local BGD_WC1 = DB.MakeHash("TURNSEG_WORLDCONGRESS_1");
	local BGD_WC2 = DB.MakeHash("TURNSEG_WORLDCONGRESS_2");

	local function BGD_MinuteurCongres()
		if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.GetValue("CPL_SMARTTIMER") == 1 then
			return;
		end
		local segment = Game.GetCurrentTurnSegment();
		if segment ~= BGD_WC1 and segment ~= BGD_WC2 then
			return;
		end
		local actuel = GameConfiguration.GetValue("TURN_TIMER_TIME");
		if type(actuel) == "number" and actuel > 0 and actuel < BGD_MINUTEUR_CONGRES then
			GameConfiguration.SetValue("TURN_TIMER_TIME", BGD_MINUTEUR_CONGRES);
			print("BGE congrès : minuteur du vote porté de " .. actuel .. " à " .. BGD_MINUTEUR_CONGRES .. " s (SmartTimer de MPH)");
		end
	end

	Events.LocalPlayerTurnBegin.Add(BGD_MinuteurCongres);
	pcall(function()
		Events.WorldCongressStage1.Add(function() BGD_MinuteurCongres(); end);
		Events.WorldCongressStage2.Add(function() BGD_MinuteurCongres(); end);
	end);
end
