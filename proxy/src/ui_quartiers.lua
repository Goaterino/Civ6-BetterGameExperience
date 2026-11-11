-- ===========================================================================
-- Civ6RolloverProxy (src/ui_quartiers.lua), ajouté par le proxy à la fin de ProductionPanel.lua
-- avec `ui_reduction_quartiers=on` (BET-22) : badge « -40 % » sur les quartiers dont le coût est
-- réduit, dans la liste de production.
--
-- Le badge suit la réduction que le jeu applique vraiment, pas la règle recalculée à chaque
-- instant. GameCore garde un coût par type de quartier et par joueur (gestionnaire de quartiers
-- +0x120, écrit par 1802c4f10) et ne le recalcule qu'à certains moments :
-- - tous les types : technologie acquise (1802841a0), dogme acquis (1803365f0), ville fondée
--   (180144c50), dogmes des ères passées accordés (180335a30) ;
-- - le seul type concerné : quartier posé (1801029c0), quartier achevé (180101ac0).
-- Achever un campus ne met donc à jour que le coût des campus ; la réduction gagnée par les autres
-- types n'arrive qu'au prochain recalcul complet. Le coût lu par l'interface
-- (BuildQueue:GetDistrictCost, 1800fd620) est ce coût enregistré, ou le coût fixé à la pose si le
-- quartier est déjà posé dans la ville.
--
-- On lit donc ce coût et on le compare au coût sans réduction, calculé comme le jeu (1803703c0) :
--   c = Cost (réduit : (100 - CostProgressionParam1) × Cost / 100)
--   c = c + ((c × GAME_COST_ESCALATION / 100 - c) × N) / 100, N = max(% de technologies, % de dogmes)
--   c = c × CostMultiplier de la vitesse / 100, puis × (100 - DistrictProductionRate de l'ère de
--   départ) / 100 (StartEras +0x1c lu par 1802ef1e0, appliqué dès que le joueur a une ère,
--   1802f7ed0 ; nul en départ antique).
-- Le coût lu est rapproché de la valeur pleine ou réduite la plus proche. Seuls les quartiers au
-- modèle COST_PROGRESSION_NUM_UNDER_AVG_PLUS_TECH peuvent être réduits (Thanh et quartiers du
-- Brésil de BBG compris). Un quartier déjà posé dans la ville garde le coût fixé à la pose : pas de
-- badge pour lui. Tout s'exécute sous pcall ; un échec écrit `Quartiers echec <erreur>`.
-- ===========================================================================
do
local Quartiers_Ok, Quartiers_Err = pcall(function()
	if BGD_Quartiers_Etat ~= nil or type(PopulateGenericBuildData) ~= "function" then
		return;
	end
	local MODELE = "COST_PROGRESSION_NUM_UNDER_AVG_PLUS_TECH";
	-- Paramètres communs à tous les types, relus à chaque remplissage du panneau.
	local S = { params = nil };
	BGD_Quartiers_Etat = S;

	local function Pourcentage(n, total)
		if total <= 0 or n >= total then return 100; end
		return math.floor(n * 100 / total);
	end

	-- Avancement N du joueur, escalade, vitesse et réduction de l'ère de départ.
	local function Parametres(playerID)
		local pPlayer = Players[playerID];
		if pPlayer == nil then return nil; end
		local techs, culture = pPlayer:GetTechs(), pPlayer:GetCulture();
		local nT, tT, nC, tC = 0, 0, 0, 0;
		for row in GameInfo.Technologies() do
			tT = tT + 1;
			if techs:HasTech(row.Index) then nT = nT + 1; end
		end
		for row in GameInfo.Civics() do
			tC = tC + 1;
			if culture:HasCivic(row.Index) then nC = nC + 1; end
		end
		local p = { avancement = math.max(Pourcentage(nT, tT), Pourcentage(nC, tC)), escalade = 1000, vitesse = 100, ere = 0 };
		local g = GameInfo.GlobalParameters ~= nil and GameInfo.GlobalParameters["GAME_COST_ESCALATION"] or nil;
		if g ~= nil and tonumber(g.Value) ~= nil then p.escalade = tonumber(g.Value); end
		local v = GameInfo.GameSpeeds ~= nil and GameInfo.GameSpeeds[GameConfiguration.GetGameSpeedType()] or nil;
		if v ~= nil and v.CostMultiplier ~= nil then p.vitesse = v.CostMultiplier; end
		local depart = GameConfiguration.GetStartEra ~= nil and GameConfiguration.GetStartEra() or nil;
		if depart ~= nil and GameInfo.StartEras ~= nil then
			for row in GameInfo.StartEras() do
				local era = GameInfo.Eras[row.EraType];
				if era ~= nil and (era.Hash == depart or era.Index == depart or row.EraType == depart) then
					p.ere = row.DistrictProductionRate or 0;
				end
			end
		end
		return p;
	end

	-- Coût du type pour le joueur, réduit ou non.
	local function Cout(row, p, reduit)
		local c = row.Cost;
		if reduit then
			c = math.floor((100 - row.CostProgressionParam1) * c / 100);
		end
		c = c + math.floor((math.floor(c * p.escalade / 100) - c) * p.avancement / 100);
		c = math.floor(c * p.vitesse / 100);
		c = math.floor(c * (100 - p.ere) / 100);
		return math.max(c, 1);
	end

	-- Réduction appliquée par le jeu au coût `cout` de ce type, ou nil.
	function BGD_ReductionQuartier(playerID, row, cout, p)
		if row == nil or row.CostProgressionModel ~= MODELE or type(cout) ~= "number" or cout <= 0 then
			return nil;
		end
		local pct = row.CostProgressionParam1 or 0;
		if pct <= 0 or row.Cost == nil then return nil; end
		p = p or Parametres(playerID);
		if p == nil then return nil; end
		-- Valeur la plus proche ; à égalité, la pleine (pas de badge douteux).
		local plein, reduit = Cout(row, p, false), Cout(row, p, true);
		if math.abs(reduit - cout) < math.abs(plein - cout) then
			return pct;
		end
		return nil;
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
		if row == nil or kInstance.LabelText == nil or row.CostProgressionModel ~= MODELE then
			return;
		end
		local pCity = UI.GetHeadSelectedCity();
		if pCity == nil or Pose(pCity, row) then
			return;
		end
		local cout = kItem.Cost;
		if type(cout) ~= "number" then
			cout = pCity:GetBuildQueue():GetDistrictCost(row.Index);
		end
		local playerID = Game.GetLocalPlayer();
		if S.params == nil then
			S.params = Parametres(playerID) or false;
		end
		local p = S.params ~= false and BGD_ReductionQuartier(playerID, row, cout, S.params) or nil;
		if p == nil then
			return;
		end
		kInstance.LabelText:SetText(kInstance.LabelText:GetText() .. " [COLOR_GREEN]-" .. p .. "%[ENDCOLOR]");
		local tip = BGD_Txt("LOC_BGD_QUARTIER_REDUCTION", p);
		if kItem.ToolTip ~= nil and kItem.ToolTip ~= "" then
			tip = tip .. "[NEWLINE][NEWLINE]" .. kItem.ToolTip;
		end
		kInstance.Button:SetToolTipString(tip);
	end

	-- Paramètres relus à chaque remplissage du panneau.
	local View_jeu = View;
	function View(data)
		S.params = nil;
		View_jeu(data);
	end

	-- Les moments où le jeu recalcule ses coûts : panneau ouvert, on le redessine.
	local function Redessiner(playerID)
		if playerID ~= nil and playerID ~= Game.GetLocalPlayer() then return; end
		if ContextPtr:IsHidden() then return; end
		Refresh();
	end
	Events.DistrictAddedToMap.Add(function(playerID) Redessiner(playerID); end);
	Events.DistrictBuildProgressChanged.Add(function(playerID) Redessiner(playerID); end);
	Events.ResearchCompleted.Add(function(playerID) Redessiner(playerID); end);
	Events.CivicCompleted.Add(function(playerID) Redessiner(playerID); end);
	Events.CityAddedToMap.Add(function(playerID) Redessiner(playerID); end);
end);
if not Quartiers_Ok then
	print("Quartiers echec " .. tostring(Quartiers_Err));
end
end
