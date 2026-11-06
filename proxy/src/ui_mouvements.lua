
-- ===========================================================================
-- Civ6RolloverProxy (src/ui_mouvements.lua), ajouté par le proxy à la fin de SelectedUnit.lua : la
-- locale m_MovementRange du fichier reste visible.
-- Zone de déplacement d'une unité posée sur un quartier défendable d'un joueur en guerre (centre-ville,
-- campement, ikanda, oppidum, thanh). UnitManager.GetReachableMovement rend nil : le parcours en
-- largeur de GameCore (1801a6e00) applique à la case de départ la règle « entrer dans un quartier
-- ennemi qui a des points de vie arrête l'unité ». Le vrai déplacement (A*, celui du survol et du clic
-- droit) ne l'applique pas. On refait la zone avec UnitManager.GetMoveToPathEx.
-- Voir analyses/quartier_capture_mouvements.
-- ===========================================================================
local BGD_MOUV_PORTEE_MAX = 10;

local function BGD_SurQuartierEnnemiDefendable(kUnit)
	local pPlot = Map.GetPlot(kUnit:GetX(), kUnit:GetY());
	if pPlot == nil then return false; end
	local pDistrict = CityManager.GetDistrictAt(pPlot);
	if pDistrict == nil then return false; end
	local kInfo = GameInfo.Districts[pDistrict:GetType()];
	if kInfo == nil or kInfo.HitPoints == nil or kInfo.HitPoints < 1 then return false; end
	local eOwner = pDistrict:GetOwner();
	local eUnitOwner = kUnit:GetOwner();
	if eOwner == -1 or eOwner == eUnitOwner then return false; end
	local pPlayer = Players[eUnitOwner];
	return pPlayer ~= nil and pPlayer:GetDiplomacy():IsAtWarWith(eOwner);
end

-- Case atteignable ce tour-ci selon le chemin du survol (turns commence à 1 = ce tour).
local function BGD_Atteignable(kUnit, iPlot)
	local kPath = UnitManager.GetMoveToPathEx(kUnit, iPlot);
	if kPath == nil or kPath.plots == nil or kPath.turns == nil then return false; end
	local n = table.count(kPath.plots);
	local t = table.count(kPath.turns);
	return n > 1 and t > 0 and kPath.plots[n] == iPlot and kPath.turns[t] <= 1;
end

-- Anneau par anneau ; on s'arrête après deux anneaux sans case atteignable (un anneau vide peut
-- venir de cases traversables où l'on ne peut pas s'arrêter, d'où la tolérance d'un anneau).
local function BGD_ZoneParChemins(kUnit)
	local x, y = kUnit:GetX(), kUnit:GetY();
	local iMoves = math.max(1, math.ceil(kUnit:GetMovesRemaining()));
	local iPortee = math.min(BGD_MOUV_PORTEE_MAX, iMoves * 4);	-- chemin de fer : 1/4 de PM par case
	local kAnneaux = {};
	for dx = -iPortee, iPortee do
		for dy = -iPortee, iPortee do
			local pPlot = Map.GetPlotXYWithRangeCheck(x, y, dx, dy, iPortee);
			if pPlot ~= nil then
				local d = Map.GetPlotDistance(x, y, pPlot:GetX(), pPlot:GetY());
				if d > 0 then
					kAnneaux[d] = kAnneaux[d] or {};
					table.insert(kAnneaux[d], pPlot:GetIndex());
				end
			end
		end
	end
	local kZone = { Map.GetPlot(x, y):GetIndex() };
	local iVides = 0;
	for d = 1, iPortee do
		local bTrouve = false;
		for _, iPlot in ipairs(kAnneaux[d] or {}) do
			if BGD_Atteignable(kUnit, iPlot) then
				table.insert(kZone, iPlot);
				bTrouve = true;
			end
		end
		if bTrouve then
			iVides = 0;
		else
			iVides = iVides + 1;
			if iVides >= 2 then break; end
		end
	end
	return kZone;
end

local BGD_RealizeMoveRadius_Jeu = RealizeMoveRadius;
function RealizeMoveRadius( kUnit:table )
	BGD_RealizeMoveRadius_Jeu(kUnit);
	if kUnit == nil or UI.IsGameCoreBusy() then return; end
	local kInfo = GameInfo.Units[kUnit:GetUnitType()];
	if kInfo == nil or kInfo.IgnoreMoves or kInfo.Spy or kInfo.MakeTradeRoute then return; end
	if not (kUnit:GetMovesRemaining() > 0) or kUnit:HasMovedIntoZOC() then return; end
	local kMove = UnitManager.GetReachableMovement(kUnit);
	if kMove ~= nil and table.count(kMove) > 1 then return; end		-- le jeu a trouvé une zone
	if not BGD_SurQuartierEnnemiDefendable(kUnit) then return; end
	local kZone = BGD_ZoneParChemins(kUnit);
	if table.count(kZone) <= 1 then return; end
	if kUnit:GetAttacksRemaining() > 0 then
		for _, iPlot in ipairs(UnitManager.GetReachableTargets(kUnit) or {}) do
			table.insert(kZone, iPlot);
		end
	end
	UILens.ClearLayerHexes(m_MovementRange);
	UILens.SetLayerHexesArea(m_MovementRange, Game.GetLocalPlayer(), kZone);
end
