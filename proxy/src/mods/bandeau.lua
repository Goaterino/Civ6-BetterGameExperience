-- ===========================================================================
-- BetterGameDLLs (src/mods/bandeau.lua) : TopPanel Extension Pro (号码菌 / Azukii, remerciements JNNR),
-- repris dans le proxy (voir mods_ui/README.md). Ajouté à la fin du script du contexte TopPanel,
-- quel qu'il soit (TopPanel_Expansion2.lua du jeu, TopPanel_Spectator.lua de BSM) : toute la chaîne
-- est alors chargée, et on enveloppe les fonctions définitives.
-- Bandeau : nourriture (excédent réel), production, population (croissance par tour) et luxes, avec
-- leurs infobulles par ville ; ressources stratégiques des coéquipiers dans l'infobulle des
-- ressources. Changements :
-- - les infobulles (TopPanel_TT du mod, un contexte à part) vivent dans ce contexte : leurs types,
--   préfixés BGD_, sont ajoutés à TopPanel.xml par le proxy, et elles sont remplies directement ;
-- - avec le fichier de BSM, son RefreshResources reste (bandeau du joueur observé) : c'est la version
--   de XP2 qu'il appelle pour les autres joueurs (XP2_RefreshResources) qui est remplacée ;
-- - textes du mod embarqués (BGD_Lookup).
-- ===========================================================================
do
	local f = Civ6Proxy_Charger and Civ6Proxy_Charger("BGD_Commun");
	if type(f) == "function" then f(); end
	local L = BGD_Lookup or Locale.Lookup;

	local BGD_RefreshYields_jeu = RefreshYields;
	local BGD_LateInitialize_jeu = LateInitialize;
	local BGD_RefreshResources_jeu = RefreshResources;

	local g_LocalplayerLuxury = {};
	local g_TopPanelResources = {};
	local g_TeamVisibleResources = {};

	local m_FoodYieldButton = nil;
	local m_PopulationYieldButton = nil;
	local m_ProductionYieldButton = nil;
	local m_LuxuryResourcesTypeYieldButton = nil;

	-- Modes « sans échange » de Team PVP Tools, et jeu sans accords.
	local isLuxuriesTradingAllowed = true;
	local isStrategicsTradingAllowed = true;
	if GameConfiguration.GetValue("TPT_NO_TRADING_STRATEGICS") == true or GameConfiguration.GetValue("SETTINGS_DIPLOMATIC_DEAL") == "SETTINGS_DEAL_ALONE" then
		isStrategicsTradingAllowed = false;
	end
	if GameConfiguration.GetValue("TPT_NO_TRADING_LUXURIES") == true then
		isLuxuriesTradingAllowed = false;
	end

	-- Sans équipes : chaque joueur rencontré compte comme coéquipier pour les luxes.
	local IsFFA = true;

	-- =======================================================================
	-- Infobulles (TopPanel_TT.lua du mod)
	-- =======================================================================
	local m_TT_Production = {};
	local m_TT_Food = {};
	local m_TT_Population = {};

	local UV_CITIZEN_STARVING_STATUS = {};
	UV_CITIZEN_STARVING_STATUS[0] = {u=0, v=0};		-- famine
	UV_CITIZEN_STARVING_STATUS[1] = {u=0, v=100};	-- stable
	UV_CITIZEN_STARVING_STATUS[2] = {u=0, v=150};	-- croissance

	local function TT_Production(Production_Info)
		if m_TT_Production.Production_Header == nil then return; end
		m_TT_Production.Production_Header:SetText(L("LOC_TOPPANEL_PRODUCTION_TOOLTIP_HEADER", Production_Info.TotalProduction));
		m_TT_Production.TopPanel_Production_Citys_Stack:DestroyAllChildren();
		if m_TT_Production.IM == nil then
			m_TT_Production.IM = InstanceManager:new("BGD_TopPanel_ProductionInstance", "BG", m_TT_Production.TopPanel_Production_Citys_Stack);
		end
		for _, kdate in ipairs(Production_Info.CitysInfo) do
			local tInstance = m_TT_Production.IM:GetInstance();
			tInstance.CityName:SetText(kdate.CityName);
			tInstance.CityProduction:SetText(Locale.ToNumber(kdate.CityProduction, "+#####.#[icon_Production]"));
		end
		m_TT_Production.TopPanel_Production_Citys_Stack:CalculateSize();
	end

	local function UpdateCitizenGrowthStatusIcon(tControls, FoodSurplus, GrowthModifier)
		local color;
		local statusIndex;
		if FoodSurplus < 0 then
			statusIndex = 0;
			color = "StatBadCSGlow";
		elseif FoodSurplus == 0 then
			statusIndex = 1;
			color = "StatNormalCSGlow";
		else
			statusIndex = 2;
			if GrowthModifier >= 1 then
				color = "StatGoodCSGlow";
			else
				color = "PolicyEconomic";
			end
		end
		tControls.CitizenGrowthStatus:SetColorByName(color);
		tControls.CitizenGrowthStatusIcon:SetColorByName(color);
		local uv = UV_CITIZEN_STARVING_STATUS[statusIndex];
		tControls.CitizenGrowthStatus:SetTextureOffsetVal(uv.u, uv.v);
	end

	local function TT_Food(Food_Info)
		if m_TT_Food.Food_Header == nil then return; end
		m_TT_Food.Food_Header:SetText(L("LOC_TOPPANEL_FOOD_TOOLTIP_HEADER", Food_Info.TotalFood));
		m_TT_Food.TopPanel_Food_Citys_Stack:DestroyAllChildren();
		if m_TT_Food.IM == nil then
			m_TT_Food.IM = InstanceManager:new("BGD_TopPanel_FoodInstance", "BG", m_TT_Food.TopPanel_Food_Citys_Stack);
		end
		for _, kdate in ipairs(Food_Info.CitysInfo) do
			local tInstance = m_TT_Food.IM:GetInstance();
			local GrowthModifierText = 100 * kdate.GrowthModifier .. "%";
			if kdate.GrowthModifier > 1 then
				GrowthModifierText = "[COLOR:StatGoodCS]" .. GrowthModifierText;
			elseif kdate.GrowthModifier < 1 then
				GrowthModifierText = "[COLOR:ResMilitaryLabelCS]" .. GrowthModifierText;
			end
			if kdate.FoodSurplus < 0 then
				GrowthModifierText = "[COLOR:ResMilitaryLabelCS]" .. Locale.Lookup("LOC_HUD_CITY_STARVING");
			end
			tInstance.CityName:SetText(kdate.CityName);
			tInstance.Food:SetText(Locale.ToNumber(kdate.CityFood, "+#####.#[Icon_Food]"));
			tInstance.FoodSurplus:SetText(Locale.ToNumber(kdate.FoodSurplus, "+#####.#[Icon_FoodSurplus];[COLOR:ResMilitaryLabelCS]-#####.#[Icon_FoodDeficit]"));
			tInstance.GrowthModifier:SetText(GrowthModifierText);
			UpdateCitizenGrowthStatusIcon(tInstance, kdate.FoodSurplus, kdate.GrowthModifier);
		end
		m_TT_Food.TopPanel_Food_Citys_Stack:CalculateSize();
	end

	local function TT_Population(Population_Info)
		if m_TT_Population.Population_Header == nil then return; end
		m_TT_Population.Population_Header:SetText(L("LOC_TOPPANEL_POPULATION_TOOLTIP_HEADER", Population_Info.TotalPopulation));
		m_TT_Population.TopPanel_Population_Citys_Stack:DestroyAllChildren();
		if m_TT_Population.IM == nil then
			m_TT_Population.IM = InstanceManager:new("BGD_TopPanel_PopulationInstance", "BG", m_TT_Population.TopPanel_Population_Citys_Stack);
		end
		for _, kdate in ipairs(Population_Info.CitysInfo) do
			local tInstance = m_TT_Population.IM:GetInstance();
			local HouseText;
			if kdate.HousingMultiplier >= 1 then
				HouseText = Locale.ToNumber(kdate.Housing, "+#####[Icon_Housing];-#####[Icon_Housing]");
			else
				-- Évite un débordement de pile sur Mac (commentaire du mod).
				local housingNum = Locale.ToNumber(kdate.Housing, "+#####;-#####");
				HouseText = "[COLOR:ResMilitaryLabelCS]" .. housingNum .. "[Icon_HousingInsufficient]";
			end
			local AmenityText = Locale.ToNumber(kdate.Amenity, "+####[ICON_Amenities];-####[ICON_Amenities]");
			if kdate.HappinessGrowthModifier > 0 then
				AmenityText = Locale.ToNumber(kdate.Amenity, "[COLOR:StatGoodCS]+####[ICON_Amenities];[COLOR:StatGoodCS]-####[ICON_Amenities]");
			elseif kdate.HappinessGrowthModifier < 0 then
				AmenityText = Locale.ToNumber(kdate.Amenity, "[COLOR:ResMilitaryLabelCS]+####[ICON_Amenities];[COLOR:ResMilitaryLabelCS]-####[ICON_Amenities]");
			end
			tInstance.CityName:SetText(kdate.CityName);
			tInstance.YieldModifier:SetText(kdate.HappinessGrowthModifier == 0 and "0%" or Locale.ToNumber(kdate.HappinessGrowthModifier, "[COLOR:StatGoodCS]+#####.#%;[COLOR:ResMilitaryLabelCS]-#####.#%"));
			tInstance.House:SetText(HouseText);
			tInstance.Amenity:SetText(AmenityText);
			tInstance.Population:SetText(kdate.Population);
		end
		m_TT_Population.TopPanel_Population_Citys_Stack:CalculateSize();
	end

	TTManager:GetTypeControlTable("BGD_TooltipType_TopPanel_Production", m_TT_Production);
	TTManager:GetTypeControlTable("BGD_TooltipType_TopPanel_Food", m_TT_Food);
	TTManager:GetTypeControlTable("BGD_TooltipType_TopPanel_Population", m_TT_Population);

	-- Infobulle remplie au premier survol après chaque rafraîchissement.
	local function SetToolTip(button, sType, fill, info)
		local canRefresh = true;
		button.YieldBacking:SetToolTipType(sType);
		button.YieldBacking:ClearToolTipCallback();
		button.YieldBacking:SetToolTipCallback(function()
			if canRefresh then
				fill(info);
				canRefresh = false;
			end
		end);
	end

	-- =======================================================================
	-- Bandeau (TopPanelExtension.lua du mod)
	-- =======================================================================

	-- Excédent de nourriture réel de la ville, et son multiplicateur de croissance.
	local function GetFoodSurplus(pCity)
		local FoodSurplusNum = 0;
		local growthModifier = 1;
		local pCityGrowth = pCity:GetGrowth();
		local isStarving = pCityGrowth:GetTurnsUntilStarvation() ~= -1;
		local HappinessGrowthModifier = pCityGrowth:GetHappinessGrowthModifier();
		local OtherGrowthModifiers = pCityGrowth:GetOtherGrowthModifier();
		local FoodSurplus = Round(pCityGrowth:GetFoodSurplus(), 1);
		local HousingMultiplier = pCityGrowth:GetHousingGrowthModifier();
		local Occupied = pCity:IsOccupied();
		local OccupationMultiplier = pCityGrowth:GetOccupationGrowthModifier();
		if not isStarving then
			growthModifier = math.max(1 + (HappinessGrowthModifier / 100) + OtherGrowthModifiers, 0);
			local iModifiedFood = Round(FoodSurplus * growthModifier, 2);
			FoodSurplusNum = iModifiedFood * HousingMultiplier;
			if Occupied then
				FoodSurplusNum = iModifiedFood * OccupationMultiplier;
			end
		else
			FoodSurplusNum = FoodSurplus;
		end
		return FoodSurplusNum, Round(growthModifier, 2);
	end

	-- Croissance par tour : excédent sur le seuil de croissance.
	local function GetPopulationPerTurn(pCity)
		local growthThreshold = pCity:GetGrowth():GetGrowthThreshold();
		local FoodSurPlus = GetFoodSurplus(pCity);
		return FoodSurPlus / growthThreshold;
	end

	local function RefreshFood()
		m_FoodYieldButton = m_FoodYieldButton or m_YieldButtonDoubleManager:GetInstance();
		local Food_Info = { TotalFood = 0, CitysInfo = {} };
		local pTotalFood = 0;
		local pTotalFoodSurplus = 0;
		for _, pCity in Players[Game.GetLocalPlayer()]:GetCities():Members() do
			local pCityFood = pCity:GetYield(YieldTypes.FOOD);
			local pFoodSurplus, growthModifier = GetFoodSurplus(pCity);
			pTotalFood = pTotalFood + pCityFood;
			pTotalFoodSurplus = pTotalFoodSurplus + pFoodSurplus;
			table.insert(Food_Info.CitysInfo, {
				CityName = Locale.Lookup(pCity:GetName()),
				CityFood = pCityFood,
				FoodSurplus = pFoodSurplus,
				GrowthModifier = growthModifier,
			});
		end
		Food_Info.TotalFood = Locale.ToNumber(pTotalFood, "#####.#");
		m_FoodYieldButton.YieldIconString:SetText("[ICON_FoodLarge]");
		m_FoodYieldButton.YieldPerTurn:SetColorByName("ResFoodLabelCS");
		m_FoodYieldButton.YieldPerTurn:SetText(Locale.ToNumber(pTotalFoodSurplus, "+#####.#;-#####.#"));
		m_FoodYieldButton.YieldBalance:SetText(Locale.ToNumber(pTotalFood, "#####.#"));
		m_FoodYieldButton.YieldBalance:SetColorByName("ResFoodLabelCS");
		m_FoodYieldButton.YieldBacking:SetColorByName("ResFoodLabelCS");
		SetToolTip(m_FoodYieldButton, "BGD_TooltipType_TopPanel_Food", TT_Food, Food_Info);
		m_FoodYieldButton.YieldButtonStack:CalculateSize();
	end

	local function RefreshPopulation()
		m_PopulationYieldButton = m_PopulationYieldButton or m_YieldButtonDoubleManager:GetInstance();
		local Population_Info = { TotalPopulation = 0, PopulationPerTurn = 0, CitysInfo = {} };
		local pTotalPopulation = 0;
		local pTotalPopulationPerTurn = 0;
		for _, pCity in Players[Game.GetLocalPlayer()]:GetCities():Members() do
			local pPopulation = pCity:GetPopulation();
			local pCityGrowth = pCity:GetGrowth();
			pTotalPopulation = pTotalPopulation + pPopulation;
			pTotalPopulationPerTurn = pTotalPopulationPerTurn + GetPopulationPerTurn(pCity);
			table.insert(Population_Info.CitysInfo, {
				CityName = Locale.Lookup(pCity:GetName()),
				Population = pPopulation,
				Housing = pCityGrowth:GetHousing() - pPopulation,
				HousingMultiplier = pCityGrowth:GetHousingGrowthModifier(),
				Amenity = pCityGrowth:GetAmenities() - pCityGrowth:GetAmenitiesNeeded(),
				HappinessGrowthModifier = pCityGrowth:GetHappinessNonFoodYieldModifier() / 100,
			});
		end
		Population_Info.TotalPopulation = Locale.ToNumber(pTotalPopulation, "#####.#");
		Population_Info.PopulationPerTurn = Locale.ToNumber(Round(pTotalPopulationPerTurn, 1), "#####.#");
		m_PopulationYieldButton.YieldIconString:SetText("[ICON_Citizen]");
		m_PopulationYieldButton.YieldIconString:SetOffsetY(6);
		m_PopulationYieldButton.YieldPerTurn:SetColorByName("StatNormalCS");
		m_PopulationYieldButton.YieldPerTurn:SetText(Locale.ToNumber(Round(pTotalPopulationPerTurn, 1), "+####.#;-####.#"));
		m_PopulationYieldButton.YieldPerTurn:SetOffsetY(-2);
		m_PopulationYieldButton.YieldBalance:SetText(Locale.ToNumber(pTotalPopulation, "#####"));
		m_PopulationYieldButton.YieldBalance:SetOffsetY(-1);
		m_PopulationYieldButton.YieldBalance:SetColorByName("StatNormalCS");
		m_PopulationYieldButton.YieldBacking:SetColorByName("ChatMessage_Whisper");
		SetToolTip(m_PopulationYieldButton, "BGD_TooltipType_TopPanel_Population", TT_Population, Population_Info);
		m_PopulationYieldButton.YieldButtonStack:CalculateSize();
	end

	local function RefreshProduction()
		m_ProductionYieldButton = m_ProductionYieldButton or m_YieldButtonSingleManager:GetInstance();
		local Production_Info = { TotalProduction = 0, CitysInfo = {} };
		local pTotalProduction = 0;
		for _, pCity in Players[Game.GetLocalPlayer()]:GetCities():Members() do
			local pCityProduction = pCity:GetYield(YieldTypes.PRODUCTION);
			pTotalProduction = pTotalProduction + pCityProduction;
			table.insert(Production_Info.CitysInfo, {
				CityName = Locale.Lookup(pCity:GetName()),
				CityProduction = pCityProduction,
			});
		end
		Production_Info.TotalProduction = Locale.ToNumber(pTotalProduction, "#####.#");
		m_ProductionYieldButton.YieldIconString:SetText("[ICON_ProductionLarge]");
		m_ProductionYieldButton.YieldPerTurn:SetText(Locale.ToNumber(pTotalProduction, "+#####.#;-#####.#"));
		m_ProductionYieldButton.YieldPerTurn:SetColorByName("ResProductionLabelCS");
		m_ProductionYieldButton.YieldBacking:SetColorByName("ChatMessage_Whisper");
		SetToolTip(m_ProductionYieldButton, "BGD_TooltipType_TopPanel_Production", TT_Production, Production_Info);
		m_ProductionYieldButton.YieldButtonStack:CalculateSize();
	end

	-- Coéquipier (ou, sans équipes, joueur rencontré) en paix avec le joueur local.
	local function IsPartner(playerID)
		local localPlayerID = Game.GetLocalPlayer();
		local pLocal = Players[localPlayerID];
		return (pLocal:GetTeam() == Players[playerID]:GetTeam() or IsFFA)
			and localPlayerID ~= playerID
			and not pLocal:GetDiplomacy():IsAtWarWith(playerID)
			and pLocal:GetDiplomacy():HasMet(playerID);
	end

	-- Luxe que le joueur local pourrait céder au premier partenaire.
	local function IsTradableResources(Resource)
		local localPlayerID = Game.GetLocalPlayer();
		for _, playerID in ipairs(PlayerManager.GetAliveMajorIDs()) do
			if IsPartner(playerID) then
				local pForDeal = DealManager.GetWorkingDeal(DealDirection.OUTGOING, localPlayerID, playerID);
				local possibleResources = DealManager.GetPossibleDealItems(localPlayerID, playerID, DealItemTypes.RESOURCES, pForDeal);
				if possibleResources ~= nil then
					for _, entry in ipairs(possibleResources) do
						if GameInfo.Resources[entry.ForType] == Resource then return true; end
					end
				end
				break;
			end
		end
		return false;
	end

	-- Le partenaire peut-il céder plus d'un exemplaire de ce luxe ?
	local function PopulateAvailableResources(otherPlayerID, Resource)
		local localPlayerID = Game.GetLocalPlayer();
		local pForDeal = DealManager.GetWorkingDeal(DealDirection.OUTGOING, localPlayerID, otherPlayerID);
		local possibleResources = DealManager.GetPossibleDealItems(otherPlayerID, localPlayerID, DealItemTypes.RESOURCES, pForDeal);
		if possibleResources ~= nil then
			for _, entry in ipairs(possibleResources) do
				if GameInfo.Resources[entry.ForType] == Resource and entry.MaxAmount > 1 then return true; end
			end
		end
		return false;
	end

	local function IsNewLuxury(resource)
		for _, iResourceType in ipairs(g_LocalplayerLuxury) do
			if resource.ResourceType == iResourceType then return false; end
		end
		return true;
	end

	-- Luxes en surplus du partenaire que le joueur local n'a pas.
	local function GetMoreLuxuryString(playerID)
		local leaderType = PlayerConfigurations[playerID]:GetLeaderTypeName();
		local s = "[NEWLINE][NEWLINE][icon_Bullet]" .. Locale.Lookup(GameInfo.Leaders[leaderType].Name);
		local pPlayerResources = Players[playerID]:GetResources();
		local More = false;
		for resource in GameInfo.Resources() do
			if resource.ResourceClassType == "RESOURCECLASS_LUXURY" then
				local amount = pPlayerResources:GetResourceAmount(resource.ResourceType);
				if amount > 1 and IsNewLuxury(resource) and PopulateAvailableResources(playerID, resource) then
					More = true;
					s = s .. "[NEWLINE][ICON_" .. resource.ResourceType .. "] " .. Locale.Lookup(resource.Name);
				end
			end
		end
		return More and s or nil;
	end

	local function RefreshLuxuryResourcesType()
		m_LuxuryResourcesTypeYieldButton = m_LuxuryResourcesTypeYieldButton or m_YieldButtonSingleManager:GetInstance();
		g_LocalplayerLuxury = {};
		local Morestr = L("LOC_TOP_PANEL_MORE_LUXURY_NAME");
		local sLuxuryResourceListText = "";
		local pLuxuryTotalAmount = 0;
		local pLuxuryTotalType = 0;
		local More = false;
		local pPlayerResources = Players[Game.GetLocalPlayer()]:GetResources();
		for resource in GameInfo.Resources() do
			if resource.ResourceClassType == "RESOURCECLASS_LUXURY" then
				local amount = pPlayerResources:GetResourceAmount(resource.ResourceType);
				if amount > 0 then
					sLuxuryResourceListText = sLuxuryResourceListText .. "[NEWLINE][ICON_" .. resource.ResourceType .. "] " .. Locale.Lookup(resource.Name);
					pLuxuryTotalAmount = pLuxuryTotalAmount + amount;
					pLuxuryTotalType = pLuxuryTotalType + 1;
					table.insert(g_LocalplayerLuxury, resource.ResourceType);
					if amount > 1 and IsTradableResources(resource) then
						More = true;
						Morestr = Morestr .. "[NEWLINE][ICON_" .. resource.ResourceType .. "] " .. Locale.Lookup(resource.Name) .. " " .. (amount - 1);
					end
				end
			end
		end
		local sYieldPerTurnText;
		if pLuxuryTotalAmount > pLuxuryTotalType and More then
			sYieldPerTurnText = "[COLOR:StatGoodCS]" .. pLuxuryTotalAmount .. "[ENDCOLOR]/" .. pLuxuryTotalType;
		else
			sYieldPerTurnText = pLuxuryTotalAmount .. "/" .. pLuxuryTotalType;
		end
		local sToolTip = L("LOC_TPT_TOP_PANEL_LUXURY_RESOURCES", pLuxuryTotalType) .. "[NEWLINE]" .. sLuxuryResourceListText;
		if More and isLuxuriesTradingAllowed then
			sToolTip = sToolTip .. Morestr;
		end
		local LUXURYtext = L("LOC_TOP_PANEL_TEAM_MORE_LUXURY_NAME");
		local TeamMore = false;
		for _, playerID in ipairs(PlayerManager.GetAliveMajorIDs()) do
			if IsPartner(playerID) then
				local s = GetMoreLuxuryString(playerID);
				if s then
					TeamMore = true;
					LUXURYtext = LUXURYtext .. s;
				end
			end
		end
		if TeamMore and isLuxuriesTradingAllowed then
			sToolTip = sToolTip .. LUXURYtext;
			sYieldPerTurnText = sYieldPerTurnText .. "[icon_PressureHigh]";
		end
		m_LuxuryResourcesTypeYieldButton.YieldIconString:SetText("[ICON_RESOURCE_TOYS]");
		m_LuxuryResourcesTypeYieldButton.YieldPerTurn:SetText(sYieldPerTurnText);
		m_LuxuryResourcesTypeYieldButton.YieldPerTurn:SetColorByName("StatNormalCS");
		m_LuxuryResourcesTypeYieldButton.YieldBacking:SetToolTipString(sToolTip);
		m_LuxuryResourcesTypeYieldButton.YieldBacking:SetColorByName("ChatMessage_Whisper");
		m_LuxuryResourcesTypeYieldButton.YieldButtonStack:CalculateSize();
	end

	-- Ressources stratégiques d'un coéquipier, pour l'infobulle d'une ressource.
	local function GetMoreStrategicString(playerID, resource)
		local leaderType = PlayerConfigurations[playerID]:GetLeaderTypeName();
		local LeaderName = Locale.Lookup(GameInfo.Leaders[leaderType].Name);
		local pPlayerResources = Players[playerID]:GetResources();
		local totalAmount = pPlayerResources:GetResourceAmount(resource.ResourceType) + pPlayerResources:GetReservedResourceAmount(resource.ResourceType);
		totalAmount = math.min(totalAmount, pPlayerResources:GetResourceStockpileCap(resource.ResourceType));
		if totalAmount > 0 then
			return "[NEWLINE][icon_bullet]" .. LeaderName .. "[NEWLINE][ICON_" .. resource.ResourceType .. "]" .. totalAmount;
		end
		return nil;
	end

	local function IsStrategic(resource)
		return resource.ResourceClassType ~= nil and resource.ResourceClassType ~= "RESOURCECLASS_BONUS"
			and resource.ResourceClassType ~= "RESOURCECLASS_LUXURY" and resource.ResourceClassType ~= "RESOURCECLASS_ARTIFACT";
	end

	-- Ressources que le joueur ou un coéquipier voit : affichées dans le bandeau dès ce moment.
	local function GetTeamVisibleResources(playerID)
		if playerID == nil or Players[playerID] == nil then return; end
		if Players[Game.GetLocalPlayer()]:GetTeam() == Players[playerID]:GetTeam() or playerID == Game.GetLocalPlayer() then
			local pPlayerResources = Players[playerID]:GetResources();
			for _, resource in ipairs(g_TopPanelResources) do
				if pPlayerResources:IsResourceVisible(resource.Hash) then
					g_TeamVisibleResources[resource.Index] = true;
				end
			end
		end
	end

	-- RefreshResources de XP2, avec les stratégiques des coéquipiers. Avec BSM, son RefreshResources
	-- reste (bandeau du joueur observé par un spectateur) ; pour les autres, il appelle la version de
	-- XP2 qu'il a gardée dans XP2_RefreshResources : c'est elle qu'on remplace.
	local function RefreshResourcesEquipe()
		if not GameCapabilities.HasCapability("CAPABILITY_DISPLAY_TOP_PANEL_RESOURCES") then
			m_kResourceIM:ResetInstances();
			return;
		end
		local localPlayerID = Game.GetLocalPlayer();
		if localPlayerID == -1 then return; end
		local localPlayer = Players[localPlayerID];
		m_kResourceIM:ResetInstances();
		local pPlayerResources = localPlayer:GetResources();
		local yieldStackX = Controls.YieldStack:GetSizeX();
		local infoStackX = Controls.StaticInfoStack:GetSizeX();
		local metaStackX = Controls.RightContents:GetSizeX();
		local screenX, _ = UIManager:GetScreenSizeVal();
		local maxSize = screenX - yieldStackX - infoStackX - metaStackX - m_viewReportsX - META_PADDING;
		if maxSize < 0 then maxSize = 0; end
		local currSize = 0;
		local isOverflow = false;
		local overflowString = "";
		local plusInstance;
		for resource in GameInfo.Resources() do
			if IsStrategic(resource) then
				local stockpileAmount = pPlayerResources:GetResourceAmount(resource.ResourceType);
				local stockpileCap = pPlayerResources:GetResourceStockpileCap(resource.ResourceType);
				local reservedAmount = pPlayerResources:GetReservedResourceAmount(resource.ResourceType);
				local accumulationPerTurn = pPlayerResources:GetResourceAccumulationPerTurn(resource.ResourceType);
				local importPerTurn = pPlayerResources:GetResourceImportPerTurn(resource.ResourceType);
				local bonusPerTurn = pPlayerResources:GetBonusResourcePerTurn(resource.ResourceType);
				local unitConsumptionPerTurn = pPlayerResources:GetUnitResourceDemandPerTurn(resource.ResourceType);
				local powerConsumptionPerTurn = pPlayerResources:GetPowerResourceDemandPerTurn(resource.ResourceType);
				local totalConsumptionPerTurn = unitConsumptionPerTurn + powerConsumptionPerTurn;
				local totalAmount = math.min(stockpileAmount + reservedAmount, stockpileCap);
				local iconName = "[ICON_" .. resource.ResourceType .. "]";
				local totalAccumulationPerTurn = accumulationPerTurn + importPerTurn + bonusPerTurn;
				local resourceText = iconName .. " " .. stockpileAmount;
				local numDigits = (stockpileAmount >= 10) and 4 or 3;
				local guessinstanceWidth = math.ceil(numDigits * FONT_MULTIPLIER);

				local tooltip = iconName .. " " .. Locale.Lookup(resource.Name);
				tooltip = tooltip .. "[NEWLINE]" .. totalAmount .. "/" .. stockpileCap .. " " .. Locale.Lookup("LOC_RESOURCE_ITEM_IN_STOCKPILE");
				if reservedAmount ~= 0 then
					tooltip = tooltip .. "[NEWLINE]-" .. reservedAmount .. " " .. Locale.Lookup("LOC_RESOURCE_ITEM_IN_RESERVE");
				end
				if totalAccumulationPerTurn >= 0 then
					tooltip = tooltip .. "[NEWLINE]" .. Locale.Lookup("LOC_RESOURCE_ACCUMULATION_PER_TURN", totalAccumulationPerTurn);
				else
					tooltip = tooltip .. "[NEWLINE][COLOR_RED]" .. Locale.Lookup("LOC_RESOURCE_ACCUMULATION_PER_TURN", totalAccumulationPerTurn) .. "[ENDCOLOR]";
				end
				if accumulationPerTurn > 0 then
					tooltip = tooltip .. "[NEWLINE] " .. Locale.Lookup("LOC_RESOURCE_ACCUMULATION_PER_TURN_EXTRACTED", accumulationPerTurn);
				end
				if importPerTurn > 0 then
					tooltip = tooltip .. "[NEWLINE] " .. Locale.Lookup("LOC_RESOURCE_ACCUMULATION_PER_TURN_FROM_CITY_STATES", importPerTurn);
				end
				if bonusPerTurn > 0 then
					tooltip = tooltip .. "[NEWLINE] " .. Locale.Lookup("LOC_RESOURCE_ACCUMULATION_PER_TURN_FROM_BONUS_SOURCES", bonusPerTurn);
				end
				if totalConsumptionPerTurn > 0 then
					tooltip = tooltip .. "[NEWLINE]" .. Locale.Lookup("LOC_RESOURCE_CONSUMPTION", totalConsumptionPerTurn);
					if unitConsumptionPerTurn > 0 then
						tooltip = tooltip .. "[NEWLINE]" .. Locale.Lookup("LOC_RESOURCE_UNIT_CONSUMPTION_PER_TURN", unitConsumptionPerTurn);
					end
					if powerConsumptionPerTurn > 0 then
						tooltip = tooltip .. "[NEWLINE]" .. Locale.Lookup("LOC_RESOURCE_POWER_CONSUMPTION_PER_TURN", powerConsumptionPerTurn);
					end
				end
				local TeamStrategicText = L("LOC_TOP_PANEL_TEAM_MORE_STRATEGIC_NAME");
				local TeamMore = false;
				for _, playerID in ipairs(PlayerManager.GetAliveMajorIDs()) do
					if localPlayer:GetTeam() == Players[playerID]:GetTeam() and localPlayerID ~= playerID then
						local s = GetMoreStrategicString(playerID, resource);
						if s then
							TeamMore = true;
							TeamStrategicText = TeamStrategicText .. s;
						end
					end
				end
				if TeamMore and isStrategicsTradingAllowed then
					tooltip = tooltip .. "[NEWLINE]" .. TeamStrategicText;
				end

				if stockpileAmount > 0 or totalAccumulationPerTurn > 0 or totalConsumptionPerTurn > 0 or g_TeamVisibleResources[resource.Index] then
					if currSize + guessinstanceWidth < maxSize and not isOverflow then
						if stockpileCap > 0 then
							local instance = m_kResourceIM:GetInstance();
							if totalAccumulationPerTurn > totalConsumptionPerTurn then
								instance.ResourceVelocity:SetHide(false);
								instance.ResourceVelocity:SetTexture("CityCondition_Rising");
							elseif totalAccumulationPerTurn < totalConsumptionPerTurn then
								instance.ResourceVelocity:SetHide(false);
								instance.ResourceVelocity:SetTexture("CityCondition_Falling");
							else
								instance.ResourceVelocity:SetHide(true);
							end
							instance.ResourceText:SetText(resourceText);
							instance.ResourceText:SetToolTipString(tooltip);
							currSize = currSize + instance.ResourceText:GetSizeX();
						end
					else
						if not isOverflow then
							overflowString = tooltip;
							local instance = m_kResourceIM:GetInstance();
							instance.ResourceText:SetText("[ICON_Plus]");
							plusInstance = instance.ResourceText;
						else
							overflowString = overflowString .. "[NEWLINE]" .. tooltip;
						end
						isOverflow = true;
					end
				end
			end
		end
		if plusInstance ~= nil then
			plusInstance:SetToolTipString(overflowString);
		end
		Controls.ResourceStack:CalculateSize();
		Controls.Resources:SetHide(Controls.ResourceStack:GetSizeX() == 0);
	end
	-- XP2_RefreshResources n'existe que si le fichier de BSM est chargé : BSM peut être actif sans
	-- (TPE présent : son fichier est alors perdu, voir mods_ui/README.md).
	if XP2_RefreshResources ~= nil then
		XP2_RefreshResources = RefreshResourcesEquipe;
	elseif BGD_RefreshResources_jeu ~= nil then
		RefreshResources = RefreshResourcesEquipe;
	end

	function RefreshYields()
		BGD_RefreshYields_jeu();
		local localPlayerID = Game.GetLocalPlayer();
		if localPlayerID == nil or localPlayerID < 0 or Players[localPlayerID] == nil then return; end
		RefreshFood();
		RefreshProduction();
		RefreshPopulation();
		RefreshLuxuryResourcesType();
		Controls.YieldStack:CalculateSize();
		Controls.StaticInfoStack:CalculateSize();
		Controls.InfoStack:CalculateSize();
	end

	function LateInitialize()
		BGD_LateInitialize_jeu();
		Events.ResearchCompleted.Add(GetTeamVisibleResources);
		Events.CivicCompleted.Add(GetTeamVisibleResources);
		for _, playerID in ipairs(PlayerManager.GetAliveMajorIDs()) do
			-- Sans équipes, l'équipe d'un joueur est son propre identifiant.
			if Players[playerID]:GetTeam() ~= playerID then IsFFA = false; end
		end
		for resource in GameInfo.Resources() do
			if IsStrategic(resource) then table.insert(g_TopPanelResources, resource); end
		end
		GetTeamVisibleResources(Game.GetLocalPlayer());
	end
end
