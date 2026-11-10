-- ===========================================================================
-- Civ6RolloverProxy (src/ui_quartiers.lua), ajouté par le proxy à la fin de ProductionPanel.lua
-- avec `ui_reduction_quartiers=on` (BET-22) : badge « -40 % » sur les quartiers dont le coût est
-- réduit, dans la liste de production.
--
-- La règle, lue dans GameCore (1802c4f10, 1803703c0, 18032c710, 18032b6e0) :
-- - seuls les quartiers au modèle COST_PROGRESSION_NUM_UNDER_AVG_PLUS_TECH sont concernés, quel que
--   soit leur RequiresPopulation (le Thanh, les quartiers du Brésil de BBG en font partie) ;
-- - moyenne = quartiers terminés des types « comptés » / nombre de types comptés. Un type est
--   compté s'il a RequiresPopulation, si ses prérequis (techno, dogme) sont acquis, et s'il est la
--   version du joueur (pas un type remplacé par son quartier unique, pas l'unique d'un autre) ;
-- - nombre du type = quartiers du joueur de ce type ou de sa version unique, terminés ou non,
--   pillés compris : poser un quartier compte tout de suite ;
-- - réduction de CostProgressionParam1 % si la moyenne vaut au moins 1 et que le nombre lui est
--   strictement inférieur (moyenne en virgule fixe, fraction sur 8 bits, comme le jeu).
-- Un quartier déjà posé dans la ville garde le coût fixé à la pose : pas de badge pour lui.
-- Tout s'exécute sous pcall ; un échec écrit `Quartiers echec <erreur>`.
-- ===========================================================================
do
local Quartiers_Ok, Quartiers_Err = pcall(function()
	if BGD_Quartiers_Etat ~= nil or type(PopulateGenericBuildData) ~= "function" then
		return;
	end
	local MODELE = "COST_PROGRESSION_NUM_UNDER_AVG_PLUS_TECH";
	local S = { reductions = nil };
	BGD_Quartiers_Etat = S;

	-- Type de base d'un quartier (celui qu'il remplace, ou lui-même).
	local function Base(row)
		local r = GameInfo.DistrictReplaces[row.DistrictType];
		return r ~= nil and r.ReplacesDistrictType or row.DistrictType;
	end

	local function Traits(playerID)
		local t = {};
		local cfg = PlayerConfigurations[playerID];
		if cfg == nil then return t; end
		local civ, chef = cfg:GetCivilizationTypeName(), cfg:GetLeaderTypeName();
		for row in GameInfo.CivilizationTraits() do
			if row.CivilizationType == civ then t[row.TraitType] = true; end
		end
		for row in GameInfo.LeaderTraits() do
			if row.LeaderType == chef then t[row.TraitType] = true; end
		end
		return t;
	end

	-- Réductions en cours du joueur : { [DistrictType] = pourcentage }.
	function BGD_ReductionsQuartiers(playerID)
		local pPlayer = Players[playerID];
		if pPlayer == nil then return {}; end
		local traits = Traits(playerID);
		local function AuJoueur(row)
			return row.TraitType == nil or traits[row.TraitType] == true;
		end
		-- Version du joueur de chaque type de base.
		local version = {};
		for row in GameInfo.Districts() do
			if row.DistrictType == Base(row) and version[row.DistrictType] == nil then
				version[row.DistrictType] = row.DistrictType;
			end
		end
		for row in GameInfo.Districts() do
			if row.DistrictType ~= Base(row) and AuJoueur(row) then
				version[Base(row)] = row.DistrictType;
			end
		end
		local techs, culture = pPlayer:GetTechs(), pPlayer:GetCulture();
		local function Acquis(row)
			if row.PrereqTech ~= nil then
				local t = GameInfo.Technologies[row.PrereqTech];
				if t == nil or not techs:HasTech(t.Index) then return false; end
			end
			if row.PrereqCivic ~= nil then
				local c = GameInfo.Civics[row.PrereqCivic];
				if c == nil or not culture:HasCivic(c.Index) then return false; end
			end
			return true;
		end
		local comptes, types = {}, 0;
		for row in GameInfo.Districts() do
			if row.RequiresPopulation and AuJoueur(row) and version[Base(row)] == row.DistrictType and Acquis(row) then
				comptes[row.Index] = true;
				types = types + 1;
			end
		end
		-- Quartiers du joueur : terminés des types comptés, et nombre par type de base.
		local termines, parBase = 0, {};
		for _, d in pPlayer:GetDistricts():Members() do
			local row = GameInfo.Districts[d:GetType()];
			if row ~= nil then
				if comptes[row.Index] and d:IsComplete() then
					termines = termines + 1;
				end
				local b = Base(row);
				parBase[b] = (parBase[b] or 0) + 1;
			end
		end
		local r = {};
		if types == 0 then return r; end
		local moyenne = math.floor(termines * 256 / types) / 256;
		if moyenne < 1 then return r; end
		for row in GameInfo.Districts() do
			if row.CostProgressionModel == MODELE and AuJoueur(row) then
				local n = parBase[Base(row)] or 0;
				-- Le jeu tronque (moyenne - nombre) × 100 : la réduction suit dès qu'il vaut 1.
				if math.floor((moyenne - n) * 100) >= 1 then
					r[row.DistrictType] = row.CostProgressionParam1;
				end
			end
		end
		return r;
	end

	-- Le quartier est déjà posé dans la ville (coût fixé à la pose).
	local function Pose(pCity, row)
		if pCity == nil then return false; end
		for _, d in pCity:GetDistricts():Members() do
			if d:GetType() == row.Index then return true; end
		end
		return false;
	end

	local Populate_jeu = PopulateGenericBuildData;
	function PopulateGenericBuildData(kInstance, kItem)
		Populate_jeu(kInstance, kItem);
		local row = kItem ~= nil and GameInfo.Districts[kItem.Type] or nil;
		if row == nil or kInstance.LabelText == nil then
			return;
		end
		if S.reductions == nil then
			S.reductions = BGD_ReductionsQuartiers(Game.GetLocalPlayer());
		end
		local p = S.reductions[row.DistrictType];
		if p == nil or p <= 0 or Pose(UI.GetHeadSelectedCity(), row) then
			return;
		end
		kInstance.LabelText:SetText(kInstance.LabelText:GetText() .. " [COLOR_GREEN]-" .. p .. "%[ENDCOLOR]");
		local tip = BGD_Txt("LOC_BGD_QUARTIER_REDUCTION", p);
		if kItem.ToolTip ~= nil and kItem.ToolTip ~= "" then
			tip = tip .. "[NEWLINE][NEWLINE]" .. kItem.ToolTip;
		end
		kInstance.Button:SetToolTipString(tip);
	end

	-- Réductions relues à chaque remplissage du panneau.
	local View_jeu = View;
	function View(data)
		S.reductions = nil;
		View_jeu(data);
	end

	-- Un quartier posé, terminé, une techno ou un dogme acquis changent la règle : panneau ouvert,
	-- on le redessine.
	local function Redessiner(playerID)
		if playerID ~= nil and playerID ~= Game.GetLocalPlayer() then return; end
		if ContextPtr:IsHidden() then return; end
		Refresh();
	end
	Events.DistrictAddedToMap.Add(function(playerID) Redessiner(playerID); end);
	Events.DistrictBuildProgressChanged.Add(function(playerID) Redessiner(playerID); end);
	Events.DistrictRemovedFromMap.Add(function(playerID) Redessiner(playerID); end);
	Events.ResearchCompleted.Add(function(playerID) Redessiner(playerID); end);
	Events.CivicCompleted.Add(function(playerID) Redessiner(playerID); end);
end);
if not Quartiers_Ok then
	print("Quartiers echec " .. tostring(Quartiers_Err));
end
end
