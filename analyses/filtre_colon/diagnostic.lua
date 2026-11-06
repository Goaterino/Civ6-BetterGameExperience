-- Diagnostic de la lentille du colon (à coller dans FireTuner, contexte InGame ou MinimapPanel).
-- Pour chaque ville du joueur local, affiche la catégorie que le jeu donne aux cases à 3 cases ou
-- moins : F = eau douce, C = côte, N = sans eau (les trois sont « fondable »), X = trop proche ou
-- interdit, . = absente des quatre listes (non coloriée).
-- Attendu par la règle : X partout à 3 cases ou moins d'un centre-ville (même masse terrestre).
-- Lancer avant et après un rechargement de la partie : si des F/C/N deviennent X au rechargement,
-- c'est le cache périmé décrit dans README.md.

local kCat = {};
local F, C, N, X = Map.GetContinentPlotsWaterAvailability();
for _, i in ipairs(F) do kCat[i] = "F"; end
for _, i in ipairs(C) do kCat[i] = "C"; end
for _, i in ipairs(N) do kCat[i] = "N"; end
for _, i in ipairs(X) do kCat[i] = "X"; end
print("listes : F=" .. #F .. " C=" .. #C .. " N=" .. #N .. " X=" .. #X);

local R = GlobalParameters.CITY_MIN_RANGE or 3;
local pPlayer = Players[Game.GetLocalPlayer()];
for _, pCity in pPlayer:GetCities():Members() do
	local cx, cy = pCity:GetX(), pCity:GetY();
	local kParDistance = {};
	for dx = -R, R do
		for dy = -R, R do
			local pPlot = Map.GetPlotXYWithRangeCheck(cx, cy, dx, dy, R);
			if pPlot ~= nil and not pPlot:IsWater() then
				local d = Map.GetPlotDistance(cx, cy, pPlot:GetX(), pPlot:GetY());
				local s = kCat[pPlot:GetIndex()] or ".";
				kParDistance[d] = (kParDistance[d] or "") .. s;
			end
		end
	end
	print(Locale.Lookup(pCity:GetName()) .. " (" .. cx .. "," .. cy .. ")");
	for d = 0, R do
		print("  distance " .. d .. " : " .. (kParDistance[d] or ""));
	end
end
