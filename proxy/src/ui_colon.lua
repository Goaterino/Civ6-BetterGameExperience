-- ===========================================================================
-- Civ6RolloverProxy (src/ui_colon.lua), inséré par le proxy avant SetWaterHexes dans
-- MinimapPanel.lua. Lentille du colon : hors des terres du joueur local,
-- Map.GetContinentPlotsWaterAvailability lit un bit « fondable » du cache de GameCore qui n'est
-- refait que quand la case elle-même change. Une ville fondée à côté ne le refait pas : les cases à 2
-- ou 3 de cette ville restaient coloriées comme fondables. On les passe dans les cases interdites,
-- pour les villes dont la case est révélée au joueur. Voir analyses/filtre_colon.
-- ===========================================================================
local function BGD_MemeMasse(pA, pB)
	local ok, r = pcall(function() return pA:GetArea():GetID() == pB:GetArea():GetID(); end);
	if ok then return r; end
	return pA:GetContinentType() == pB:GetContinentType();
end

function BGD_FiltrerColon(kFull, kCoast, kNoWater, kNoSettle)
	local eLocal = Game.GetLocalPlayer();
	local pVis = PlayersVisibility[eLocal];
	if pVis == nil or kFull == nil or kCoast == nil or kNoWater == nil or kNoSettle == nil then
		return kFull, kCoast, kNoWater, kNoSettle;
	end
	local R = (GlobalParameters ~= nil and GlobalParameters.CITY_MIN_RANGE) or 3;
	local kTrop = {};
	for _, pPlayer in ipairs(PlayerManager.GetAlive()) do
		for _, pCity in pPlayer:GetCities():Members() do
			local cx, cy = pCity:GetX(), pCity:GetY();
			local pCentre = Map.GetPlot(cx, cy);
			if pCentre ~= nil and pVis:IsRevealed(pCentre:GetIndex()) then
				for dx = -R, R do
					for dy = -R, R do
						local pPlot = Map.GetPlotXYWithRangeCheck(cx, cy, dx, dy, R);
						if pPlot ~= nil then
							local d = Map.GetPlotDistance(cx, cy, pPlot:GetX(), pPlot:GetY());
							if d < R or BGD_MemeMasse(pPlot, pCentre) then
								kTrop[pPlot:GetIndex()] = true;
							end
						end
					end
				end
			end
		end
	end
	local function garder(t)
		local r = {};
		for _, i in ipairs(t) do
			if kTrop[i] then table.insert(kNoSettle, i); else table.insert(r, i); end
		end
		return r;
	end
	return garder(kFull), garder(kCoast), garder(kNoWater), kNoSettle;
end

