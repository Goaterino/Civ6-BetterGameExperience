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
-- quartier est déjà posé dans la ville. Mais l'interface le lit dans le cache de la file de la
-- ville (18006b860), reconstruit seulement quand cette file change : après la pose d'un campus
-- ailleurs, une ville déjà vue garde l'ancien coût (BGE-36E0D7). Le badge lit donc la table de
-- GameCore en direct (BGD_CoutQuartier, debordement.rs), et le coût du panneau à défaut.
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

	-- -----------------------------------------------------------------------------------------
	-- Diagnostic (réglage diag_production du proxy, qui pose BGD_DIAG_PRODUCTION) : une ligne
	-- `BGD diag quartiers` dans Lua.log par ville et par type au modèle réduit, à chaque
	-- changement. Elle refait la règle de 1802c4f10 (cas 5) avec 18032c710 et 18032b6e0 :
	-- - nombre du type : quartiers du joueur de ce type ou de son remplaçant unique, achevés ou
	--   non (18032c710 avec param_3 = param_4 = 1) ;
	-- - moyenne : quartiers ACHEVÉS du type exact, sommés sur les types à RequiresPopulation
	--   débloqués (techno et dogme, 1802c48c0) et propres au joueur (1802c3fc0, approché ici :
	--   type sans TraitType ou au trait du joueur, et non remplacé par un unique du joueur),
	--   divisés par le nombre de ces types ;
	-- - réduit si partie entière de la moyenne >= 1 et nombre < moyenne.
	-- Le Lua ne lit pas la table de GameCore : `GetDistrictCost` (frais) et `kItem.Cost` (données
	-- du panneau) sont les deux coûts que l'interface voit.
	local Diag_Vus = {};

	local function Diag_Traits(playerID)
		local t = {};
		local cfg = PlayerConfigurations ~= nil and PlayerConfigurations[playerID] or nil;
		if cfg == nil then return t; end
		local civ, chef = cfg:GetCivilizationTypeName(), cfg:GetLeaderTypeName();
		if GameInfo.CivilizationTraits ~= nil then
			for r in GameInfo.CivilizationTraits() do
				if r.CivilizationType == civ then t[r.TraitType] = true; end
			end
		end
		if GameInfo.LeaderTraits ~= nil then
			for r in GameInfo.LeaderTraits() do
				if r.LeaderType == chef then t[r.TraitType] = true; end
			end
		end
		return t;
	end

	-- Comptes du joueur, refaits à chaque remplissage du panneau (S.diag remis à nil par View).
	local function Diag_Comptes(playerID)
		local pPlayer = Players[playerID];
		if pPlayer == nil then return nil; end
		local traits = Diag_Traits(playerID);
		-- Unique -> type remplacé, et types remplacés par un unique du joueur.
		local remplace, remplaceParJoueur = {}, {};
		if GameInfo.DistrictReplaces ~= nil then
			for r in GameInfo.DistrictReplaces() do
				remplace[r.CivUniqueDistrictType] = r.ReplacesDistrictType;
				local u = GameInfo.Districts[r.CivUniqueDistrictType];
				if u ~= nil and u.TraitType ~= nil and traits[u.TraitType] then
					remplaceParJoueur[r.ReplacesDistrictType] = true;
				end
			end
		end
		local acheves, enCours = {}, {};
		for _, d in pPlayer:GetDistricts():Members() do
			local r = GameInfo.Districts[d:GetType()];
			if r ~= nil then
				if d:IsComplete() then
					acheves[r.DistrictType] = (acheves[r.DistrictType] or 0) + 1;
				else
					enCours[r.DistrictType] = (enCours[r.DistrictType] or 0) + 1;
				end
			end
		end
		local techs, culture = pPlayer:GetTechs(), pPlayer:GetCulture();
		local types, somme, noms = 0, 0, {};
		for r in GameInfo.Districts() do
			local propre = (r.TraitType == nil or traits[r.TraitType]) and not remplaceParJoueur[r.DistrictType];
			local debloque = true;
			if r.PrereqTech ~= nil then
				local t = GameInfo.Technologies[r.PrereqTech];
				debloque = t ~= nil and techs:HasTech(t.Index);
			end
			if debloque and r.PrereqCivic ~= nil then
				local c = GameInfo.Civics[r.PrereqCivic];
				debloque = c ~= nil and culture:HasCivic(c.Index);
			end
			if r.RequiresPopulation and propre and debloque then
				types = types + 1;
				somme = somme + (acheves[r.DistrictType] or 0);
				noms[#noms + 1] = string.gsub(r.DistrictType, "^DISTRICT_", "") .. "=" .. (acheves[r.DistrictType] or 0);
			end
		end
		return { remplace = remplace, acheves = acheves, enCours = enCours, types = types, somme = somme, noms = table.concat(noms, ",") };
	end

	local function Diag(pCity, row, kItem, pose)
		if not BGD_DIAG_PRODUCTION or pCity == nil or row == nil then return; end
		local ok, err = pcall(function()
			local playerID = Game.GetLocalPlayer();
			if S.diag == nil then S.diag = Diag_Comptes(playerID) or false; end
			if S.params == nil then S.params = Parametres(playerID) or false; end
			local C, p = S.diag, S.params;
			if C == false or p == false then return; end
			-- Nombre du type : le type et ses remplaçants uniques.
			local nA, nC = 0, 0;
			for t, n in pairs(C.acheves) do
				if t == row.DistrictType or C.remplace[t] == row.DistrictType then nA = nA + n; end
			end
			for t, n in pairs(C.enCours) do
				if t == row.DistrictType or C.remplace[t] == row.DistrictType then nC = nC + n; end
			end
			local nombre = nA + nC;
			local moyenne = C.types > 0 and C.somme / C.types or 0;
			-- Moyenne en virgule fixe 24.8, comme 18032b6e0.
			local m8 = math.floor(moyenne) + math.floor((moyenne - math.floor(moyenne)) * 256) / 256;
			local regle = (math.floor(m8) >= 1 and nombre <= m8 and math.floor(m8 * 100 - nombre * 100) > 0) and "reduit" or "plein";
			local frais = pCity:GetBuildQueue():GetDistrictCost(row.Index);
			local plein, reduit = Cout(row, p, false), Cout(row, p, true);
			local direct = type(BGD_CoutQuartier) == "function" and BGD_CoutQuartier(playerID, row.Index) or nil;
			local lu = (type(direct) == "number" and direct > 0) and direct or ((type(kItem.Cost) == "number") and kItem.Cost or frais);
			local badge = (not pose) and BGD_ReductionQuartier(playerID, row, lu, p) ~= nil;
			local tour = Game.GetCurrentGameTurn ~= nil and Game.GetCurrentGameTurn() or -1;
			local corps = string.format(
				"ville %s (%d), %s : kItem.Cost %s, GetDistrictCost %s, GameCore %s, plein %d, reduit %d, badge %s, pose dans la ville %s, nombre %d (acheves %d, en construction %d), moyenne %.3f (%d acheves / %d types debloques : %s), regle %s, avancement %d, escalade %d, vitesse %d, ere %d",
				Locale.Lookup(pCity:GetName()), pCity:GetID(), row.DistrictType, tostring(kItem.Cost), tostring(frais), tostring(direct),
				plein, reduit, badge and "oui" or "non", pose and "oui" or "non", nombre, nA, nC, m8, C.somme, C.types, C.noms,
				regle, p.avancement, p.escalade, p.vitesse, p.ere);
			local cle = pCity:GetID() .. ":" .. row.DistrictType;
			if Diag_Vus[cle] == corps then return; end
			Diag_Vus[cle] = corps;
			print("BGD diag quartiers : tour " .. tour .. ", " .. corps);
		end);
		if not ok then print("BGD diag quartiers : echec " .. tostring(err)); end
	end

	local Populate_jeu = PopulateGenericBuildData;
	function PopulateGenericBuildData(kInstance, kItem)
		Populate_jeu(kInstance, kItem);
		local row = kItem ~= nil and GameInfo.Districts[kItem.Type] or nil;
		if row == nil or kInstance.LabelText == nil or row.CostProgressionModel ~= MODELE then
			return;
		end
		local pCity = UI.GetHeadSelectedCity();
		local pose = pCity ~= nil and Pose(pCity, row);
		if BGD_DIAG_PRODUCTION then Diag(pCity, row, kItem, pose); end
		if pCity == nil or pose then
			return;
		end
		local playerID = Game.GetLocalPlayer();
		-- Coût enregistré, lu en direct dans GameCore (BGD_CoutQuartier, posée par le proxy) : celui
		-- du panneau vient du cache de la ville, qui n'est reconstruit que quand sa file change. Une
		-- ville déjà vue gardait le coût réduit d'avant la pose d'un campus ailleurs (BGE-36E0D7).
		local cout = type(BGD_CoutQuartier) == "function" and BGD_CoutQuartier(playerID, row.Index) or nil;
		if type(cout) ~= "number" or cout <= 0 then
			cout = kItem.Cost;
		end
		if type(cout) ~= "number" then
			cout = pCity:GetBuildQueue():GetDistrictCost(row.Index);
		end
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
		S.diag = nil;
		View_jeu(data);
	end

	-- Les moments où le jeu recalcule ses coûts : panneau ouvert, on le redessine.
	local function Redessiner(playerID, evenement)
		if playerID ~= nil and playerID ~= Game.GetLocalPlayer() then return; end
		if BGD_DIAG_PRODUCTION and evenement ~= nil then
			local tour = Game.GetCurrentGameTurn ~= nil and Game.GetCurrentGameTurn() or -1;
			print("BGD diag quartiers : tour " .. tour .. ", evenement " .. evenement .. " (panneau " .. (ContextPtr:IsHidden() and "cache" or "ouvert") .. ")");
		end
		if ContextPtr:IsHidden() then return; end
		Refresh();
	end
	Events.DistrictAddedToMap.Add(function(playerID, districtID, cityID, x, y, districtType)
		local r = districtType ~= nil and GameInfo.Districts[districtType] or nil;
		Redessiner(playerID, "DistrictAddedToMap " .. (r ~= nil and r.DistrictType or tostring(districtType)) .. " ville " .. tostring(cityID));
	end);
	Events.DistrictBuildProgressChanged.Add(function(playerID) Redessiner(playerID); end);
	Events.ResearchCompleted.Add(function(playerID) Redessiner(playerID, "ResearchCompleted"); end);
	Events.CivicCompleted.Add(function(playerID) Redessiner(playerID, "CivicCompleted"); end);
	Events.CityAddedToMap.Add(function(playerID) Redessiner(playerID, "CityAddedToMap"); end);
end);
if not Quartiers_Ok then
	print("Quartiers echec " .. tostring(Quartiers_Err));
end
end
