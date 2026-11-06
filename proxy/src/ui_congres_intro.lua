-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_intro.lua), ajouté par le proxy à la fin de
-- WorldCongressIntro.lua. L'intro (« Bienvenue au congrès mondial », avec son son) s'ouvre à chaque
-- Events.WorldCongressStage1 / 2, quel que soit le joueur concerné. Avec le retour au vote
-- (src/congres.rs), l'annulation de fin de tour d'un autre joueur relance l'événement : l'intro
-- revenait chez tout le monde. Elle ne s'ouvre plus qu'une fois par tour et par session (ordinaire
-- ou extraordinaire). Après un rechargement, l'état Lua est neuf : elle s'ouvre de nouveau.
-- ===========================================================================
do
	local m_BGD_Ouvertes = {};	-- clé « tour:session » -> true

	local BGD_OnOpen = OnOpen;
	function OnOpen(stageNum, ...)
		local cle = tostring(Game.GetCurrentGameTurn()) .. ":" .. tostring(stageNum);
		if m_BGD_Ouvertes[cle] then
			return;
		end
		m_BGD_Ouvertes[cle] = true;
		return BGD_OnOpen(stageNum, ...);
	end
end
