-- ===========================================================================
-- Civ6RolloverProxy (src/ui_logement.lua), appended by the proxy to the end of
-- CityBannerManager.lua (XP2 or BSM) with `ui_logement=on`: an early housing warning on the banner
-- of the local player's cities.
--
-- The game only shows its red "Insufficient Housing" icon once a city is over its cap
-- (`GetHousing() < GetPopulation()`), but growth is cut well before that
-- (CITY_HOUSING_LEFT_50PCT_GROWTH = 1, CITY_HOUSING_LEFT_25PCT_GROWTH = 0):
--   housing - population >= 2 : normal growth
--   housing - population == 1 : -50%   -> orange house (this block)
--   housing - population == 0 : -75%   -> red house (this block, with the game's icon)
--   housing - population <  0 : -75%, then no growth at -5 -> red house (the game's own icon)
-- One colour per tier of the modifier: the red house comes with -75%, at the cap, not one
-- population later.
-- The level is not recomputed here: it is read from the growth modifier the game applies
-- (CityGrowth:GetHousingGrowthModifier, the value CitySupport.lua puts in HousingMultiplier), so
-- fractional housing and mods that change the thresholds follow.
--
-- The orange house is the icon the city panel uses for housing (ICON_HOUSING, the white glyph of
-- the Stats atlas), tinted with WarningMajor of Civ6_ColorAtlas.xml. The red house is the game's
-- banner icon itself (ICON_CITY_EFFECTS_HOUSING), untouched: at the cap and over it, the same
-- tier shows the same icon. The tooltip needs no text of ours: population / housing with the
-- game's font icons, and the game's "Population growth rate slowed N%", already translated.
--
-- CityBanner:UpdateDetails is wrapped: the original fills the status and effect icons, then one
-- more effect instance is taken for the house. Instances are reused by the next update for any
-- other icon, and the game's SetDetailIcon resets neither the colour nor the size: the wrapper
-- puts them back before calling the original (orange house only, the red one changes neither).
-- No event is added: the icon is redrawn when the game redraws its own, on the same refreshes of
-- the banner.
-- Display only, local player's cities only. Everything runs under pcall; a failure writes
-- `Logement echec <error>`.
-- ===========================================================================
do
local Logement_Ok, Logement_Err = pcall(function()
	if BGD_Logement_Etat ~= nil or CityBanner == nil or type(CityBanner.UpdateDetails) ~= "function" then
		return;
	end
	BGD_Logement_Etat = true;

	local ICONE = "ICON_HOUSING";
	local ICONE_JEU = "ICON_CITY_EFFECTS_HOUSING";
	-- The Stats atlas has no 20 px sheet (16, 22, 32...) : the 20 px slot is widened for this icon.
	local TAILLE = 22;
	-- 0xAABBGGRR. WarningMajor (200,146,52).
	local ORANGE = UI.GetColorValueFromHexLiteral(0xFF3492C8);
	local BLANC = UI.GetColorValueFromHexLiteral(0xFFFFFFFF);

	-- Level of the warning for this city (1 : -50%, orange ; 2 : -75%, red), and the growth lost in
	-- percent ; nil when there is nothing to add (normal growth, or the game already shows its red
	-- icon).
	local function Niveau(pCity)
		local pGrowth = pCity:GetGrowth();
		if pGrowth == nil or type(pGrowth.GetHousingGrowthModifier) ~= "function" then return nil; end
		local m = pGrowth:GetHousingGrowthModifier();
		if type(m) ~= "number" or m > 0.5 then return nil; end
		local logement, population = pGrowth:GetHousing(), pCity:GetPopulation();
		if logement < population then return nil; end
		local perte = math.floor((1 - m) * 100 + 0.5);
		return (m > 0.25) and 1 or 2, perte, logement, population;
	end

	-- 6 -> "6", 6.5 -> "6.5" (farms and other improvements give half points).
	local function Nombre(v)
		if v == math.floor(v) then return tostring(math.floor(v)); end
		return string.format("%.1f", v);
	end

	-- The instance tinted by the previous update goes back to what the game expects.
	local function Remettre(banner)
		local e = banner.BGD_Logement;
		if e == nil then return; end
		banner.BGD_Logement = nil;
		-- The instance may have been destroyed with the banner since.
		pcall(function()
			e.icone:SetColor(BLANC);
			e.icone:SetSizeVal(e.x, e.y);
		end);
	end

	local Update_jeu = CityBanner.UpdateDetails;
	CityBanner.UpdateDetails = function(self, ...)
		Remettre(self);
		Update_jeu(self, ...);
		if self.m_DetailEffectsIM == nil or type(self.GetCity) ~= "function" then return; end
		local pCity = self:GetCity();
		if pCity == nil or pCity:GetOwner() ~= Game.GetLocalPlayer() then return; end
		local niveau, perte, logement, population = Niveau(pCity);
		if niveau == nil then return; end
		local inst = self.m_DetailEffectsIM:GetInstance();
		if inst == nil or inst.Icon == nil then return; end
		if niveau == 1 then
			self.BGD_Logement = { icone = inst.Icon, x = inst.Icon:GetSizeX(), y = inst.Icon:GetSizeY() };
			inst.Icon:SetSizeVal(TAILLE, TAILLE);
			inst.Icon:SetIcon(ICONE, TAILLE);
			inst.Icon:SetColor(ORANGE);
		else
			-- As the game sets it over the cap.
			inst.Icon:SetIcon(ICONE_JEU);
		end
		inst.Icon:SetHide(false);
		inst.Icon:SetToolTipString("[ICON_Citizen] " .. Nombre(population) .. " / " .. Nombre(logement) .. " [ICON_Housing]"
			.. "[NEWLINE]" .. Locale.Lookup("LOC_HUD_CITY_POPULATION_GROWTH_SLOWED", perte));
		-- The instance may come from the power icon, which has a click.
		if inst.Button ~= nil then inst.Button:ClearCallback(Mouse.eLClick); end
		-- The original sized the banner before this icon was added.
		local pile = self.m_Instance ~= nil and self.m_Instance.CityDetailsEffects or nil;
		if pile ~= nil then pile:CalculateSize(); end
		if type(self.Resize) == "function" then self:Resize(); end
	end
end);
if not Logement_Ok then
	print("Logement echec " .. tostring(Logement_Err));
end
end
