-- ===========================================================================
-- Civ6RolloverProxy (src/ui_depart_panneau.lua) : bouton de fin de tour pendant la retenue du début
-- de tour (départ simultané, depart_tour/README.md). Ajouté par le proxy de GameCore à la fin de
-- ActionPanel.lua, qu'incluent les versions des extensions (ActionPanel_Expansion1/2.lua).
-- Le bouton ne vit pas seulement des événements retenus : EndTurnDirty et EndTurnBlockingChanged
-- arrivent avant LocalPlayerTurnBegin (ils ne sont donc pas retenus), et OnRefresh lit l'état
-- réel de la simulation (tour local actif). Tant que le proxy retient le début du tour :
-- - UI.IsProcessingMessages() répond vrai (seul ActionPanel.lua s'en sert) : OnRefresh affiche
--   l'attente (SetEndTurnWaiting) et DoEndTurn refuse. Le contexte reçoit sa propre table UI, qui
--   renvoie le reste à celle du jeu ;
-- - OnEndTurnBlockingChanged (sélection de la première unité, fin de tour automatique) est
--   remis à LocalPlayerTurnBegin, après le départ ;
-- - les mises à jour du minuteur sont ignorées : la jauge reste dans son état du passage de tour.
-- Ces gestionnaires sont enregistrés plus tard (LateInitialize, à OnInit) : ce sont ces versions
-- qui le seront.
-- ===========================================================================
do
local BGD_DepartPanneauOk, BGD_DepartPanneauErr = pcall(function()
	if BGD_DepartPanneau ~= nil then
		return;
	end
	BGD_DepartPanneau = true;

	local function J(texte)
		if Civ6RolloverProxy_DepartJournal ~= nil then
			Civ6RolloverProxy_DepartJournal("panneau : " .. texte);
		end
	end

	local vu = false;
	local function Retenu()
		if Civ6RolloverProxy_DepartEtat == nil then
			return false;
		end
		local _, _, _, retenu = Civ6RolloverProxy_DepartEtat();
		if retenu == 1 and not vu then
			vu = true;
			J("début de tour retenu, bouton en attente");
		elseif retenu ~= 1 then
			vu = false;
		end
		return retenu == 1;
	end

	local BGD_UI = UI;
	UI = setmetatable({
		IsProcessingMessages = function(...)
			if Retenu() then
				return true;
			end
			return BGD_UI.IsProcessingMessages(...);
		end,
	}, { __index = BGD_UI });

	if OnTurnTimerUpdated ~= nil then
		local BGD_MinuteurBase = OnTurnTimerUpdated;
		OnTurnTimerUpdated = function(...)
			if Retenu() then
				return;
			end
			return BGD_MinuteurBase(...);
		end
	end

	if OnEndTurnBlockingChanged ~= nil then
		local BGD_BlocageBase = OnEndTurnBlockingChanged;
		local remis = nil;
		OnEndTurnBlockingChanged = function(...)
			if Retenu() then
				remis = { ... };
				return;
			end
			remis = nil;
			return BGD_BlocageBase(...);
		end
		Events.LocalPlayerTurnBegin.Add(function()
			if remis ~= nil then
				local args = remis;
				remis = nil;
				local ok, err = pcall(BGD_BlocageBase, args[1], args[2]);
				if not ok then
					J("erreur : " .. tostring(err));
				end
			end
		end);
	end
	J("posé");
end);
if not BGD_DepartPanneauOk then
	print("BGD depart panneau echec " .. tostring(BGD_DepartPanneauErr));
	if Civ6RolloverProxy_DepartJournal ~= nil then
		Civ6RolloverProxy_DepartJournal("panneau : erreur au chargement : " .. tostring(BGD_DepartPanneauErr));
	end
end
end
