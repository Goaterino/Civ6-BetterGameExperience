-- ===========================================================================
-- Civ6RolloverProxy (src/ui_pause.lua), ajouté par le proxy à la fin de PausePanel.lua (celui du
-- jeu ou celui de Multiplayer Helper). En multijoueur, la partie est en pause pendant tout le
-- chargement, et la fenêtre « Partie en pause » s'affichait par-dessus l'écran de chargement :
-- - Initialize, à la fin du fichier d'origine, l'ouvre avant que ce bloc ne soit chargé ;
-- - CheckPausedState est rappelée à chaque GameConfigChanged et PlayerInfoChanged, et
--   OpenPausePanel la remettait en file et rejouait son animation tant que le contexte était caché.
-- Le contexte se charge sous l'écran de chargement : la fenêtre en est retirée, n'est plus ouverte
-- avant LoadScreenClose, puis la pause est réévaluée. Hors chargement, elle n'est plus remise en
-- file tant qu'elle y est déjà, jusqu'à ClosePausePanel. Les fonctions sont appelées par leur nom
-- global : l'enveloppe les atteint.
-- ===========================================================================
do
	local m_BGD_EnFile = false;
	local m_BGD_Chargement = true;

	-- La file des popups fait foi ; le drapeau ne sert que si UIManager ne sait pas répondre.
	local function BGD_DansLaFile()
		local ok, dedans = pcall(function() return UIManager:IsInPopupQueue(ContextPtr); end);
		if ok and type(dedans) == "boolean" then
			return dedans;
		end
		return m_BGD_EnFile;
	end

	local BGD_Ouvrir = OpenPausePanel;
	function OpenPausePanel(...)
		if m_BGD_Chargement or BGD_DansLaFile() then
			return;
		end
		m_BGD_EnFile = true;
		return BGD_Ouvrir(...);
	end

	local BGD_Fermer = ClosePausePanel;
	function ClosePausePanel(...)
		m_BGD_EnFile = false;
		return BGD_Fermer(...);
	end

	-- Ouverte par Initialize : retirée jusqu'à la fin du chargement.
	pcall(function() UIManager:DequeuePopup(ContextPtr); end);

	local function BGD_FinChargement()
		if not m_BGD_Chargement then
			return;
		end
		m_BGD_Chargement = false;
		if not GameConfiguration.IsHotseat() and not WorldBuilder.IsActive() then
			CheckPausedState();
		end
	end
	-- Pas de filet sur LocalPlayerTurnBegin : au départ d'une partie, il peut précéder
	-- LoadScreenClose et rouvrirait la fenêtre sur l'écran de chargement. Si le contexte était
	-- rechargé en pleine partie, la fenêtre manquerait jusqu'au chargement suivant ; la pause, elle,
	-- reste la même.
	Events.LoadScreenClose.Add(BGD_FinChargement);

	if type(OnShutdown) == "function" then
		local BGD_Arret = OnShutdown;
		ContextPtr:SetShutdown(function(...)
			Events.LoadScreenClose.Remove(BGD_FinChargement);
			return BGD_Arret(...);
		end);
	end
end
