-- ===========================================================================
-- BetterGameExperience (proxy/src/mods/routes_choix.lua), ajouté par le proxy à la fin de
-- TradeRouteChooser.lua (celui du jeu ou celui de BBG) : réglage ui_routes.
--
-- Une capacité comme celle de Poundmaker (Cree, « Conditions favorables ») donne de l'or à la ville
-- de DESTINATION de la route : +1 or par camp et par pâturage de cette ville
-- (MODIFIER_PLAYER_ADJUST_TRADE_ROUTE_YIELD_PER_IMPROVEMENT_IN_TARGET, Destination = true).
-- L'écran de choix ne montre, pour chaque route, que les rendements de la ville d'origine : cet or
-- n'y apparaît pas. Vers une ville du même joueur, c'est pourtant lui qui le reçoit. On l'ajoute
-- donc aux rendements montrés pour une route intérieure (liste des destinations, en-tête de la
-- route choisie, filtre par rendement), avec une ligne d'infobulle qui dit quelle ville le reçoit.
--
-- Le bonus est relu dans la base (capacités du dirigeant et de la civilisation du joueur), pas
-- codé pour Poundmaker : seuls les modificateurs de ce type, sans condition, vers la destination.
-- Les aménagements pillés ne sont pas comptés.
-- ===========================================================================
do
	local TYPE_BONUS = "MODIFIER_PLAYER_ADJUST_TRADE_ROUTE_YIELD_PER_IMPROVEMENT_IN_TARGET";
	local m_bonus = {};	-- par joueur : { { amenagement, rendement, montant, source } }

	local function Vrai(v)
		return v ~= nil and (v == true or v == 1 or string.lower(tostring(v)) == "true" or tostring(v) == "1");
	end

	-- Les bonus d'un joueur vers la ville de destination, lus une fois.
	local function Bonus(playerID)
		if m_bonus[playerID] ~= nil then return m_bonus[playerID]; end
		local liste = {};
		m_bonus[playerID] = liste;
		local config = PlayerConfigurations[playerID];
		if config == nil or GameInfo.TraitModifiers == nil then return liste; end
		local traits = {};
		local dirigeant = config:GetLeaderTypeName();
		local civ = config:GetCivilizationTypeName();
		if GameInfo.LeaderTraits ~= nil then
			for row in GameInfo.LeaderTraits() do
				if row.LeaderType == dirigeant then traits[row.TraitType] = true; end
			end
		end
		if GameInfo.CivilizationTraits ~= nil then
			for row in GameInfo.CivilizationTraits() do
				if row.CivilizationType == civ then traits[row.TraitType] = true; end
			end
		end
		local modifs = {};
		for row in GameInfo.TraitModifiers() do
			if traits[row.TraitType] then
				local m = GameInfo.Modifiers[row.ModifierId];
				if m ~= nil and m.ModifierType == TYPE_BONUS and m.SubjectRequirementSetId == nil and m.OwnerRequirementSetId == nil then
					modifs[row.ModifierId] = { trait = row.TraitType };
				end
			end
		end
		if next(modifs) == nil then return liste; end
		for row in GameInfo.ModifierArguments() do
			local m = modifs[row.ModifierId];
			if m ~= nil then m[row.Name] = row.Value; end
		end
		for _, m in pairs(modifs) do
			local amenagement = m.ImprovementType and GameInfo.Improvements[m.ImprovementType];
			local rendement = m.YieldType and GameInfo.Yields[m.YieldType];
			local montant = tonumber(m.Amount);
			if Vrai(m.Destination) and not Vrai(m.Origin) and amenagement ~= nil and rendement ~= nil and montant ~= nil and montant ~= 0 then
				local trait = GameInfo.Traits[m.trait];
				table.insert(liste, {
					amenagement = amenagement.Index,
					rendement = rendement.Index,
					montant = montant,
					source = (trait ~= nil and trait.Name ~= nil) and Locale.Lookup(trait.Name) or m.trait,
				});
			end
		end
		return liste;
	end

	-- Le bonus que reçoit la destination d'une route intérieure : { [rendement] = { total, { source = montant } } }.
	local function BonusDestination(pOrigine, pDestination)
		local resultat = {};
		if pOrigine == nil or pDestination == nil or pOrigine:GetOwner() ~= pDestination:GetOwner() then return resultat; end
		local liste = Bonus(pOrigine:GetOwner());
		if #liste == 0 then return resultat; end
		local nombre = {};
		local cases = Map.GetCityPlots():GetPurchasedPlots(pDestination);
		for _, index in ipairs(cases or {}) do
			local plot = Map.GetPlotByIndex(index);
			if plot ~= nil then
				local a = plot:GetImprovementType();
				if a ~= nil and a >= 0 and not plot:IsImprovementPillaged() then
					nombre[a] = (nombre[a] or 0) + 1;
				end
			end
		end
		for _, b in ipairs(liste) do
			local n = nombre[b.amenagement] or 0;
			if n > 0 then
				local r = resultat[b.rendement] or { total = 0, sources = {}, ordre = {} };
				resultat[b.rendement] = r;
				r.total = r.total + n * b.montant;
				if r.sources[b.source] == nil then
					r.sources[b.source] = 0;
					table.insert(r.ordre, b.source);
				end
				r.sources[b.source] = r.sources[b.source] + n * b.montant;
			end
		end
		return resultat;
	end

	if type(GetYieldsForRoute) == "function" then
		local GetYieldsForRoute_jeu = GetYieldsForRoute;
		function GetYieldsForRoute(pOriginCity, pDestinationCity, bReturnDestiationYields)
			local info = GetYieldsForRoute_jeu(pOriginCity, pDestinationCity, bReturnDestiationYields);
			if bReturnDestiationYields or info == nil or info.kYieldValues == nil then return info; end
			local ok, bonus = pcall(BonusDestination, pOriginCity, pDestinationCity);
			if not ok then
				print("BetterGameExperience : bonus de destination des routes : " .. tostring(bonus));
				return info;
			end
			for rendement, r in pairs(bonus) do
				local i = rendement + 1;
				local yieldInfo = GameInfo.Yields[rendement];
				info.kYieldValues[i] = (info.kYieldValues[i] or 0) + r.total;
				for _, source in ipairs(r.ordre) do
					local ligne = BGD_Txt("LOC_BGD_ROUTE_OR_DESTINATION", r.sources[source], yieldInfo.IconString,
						Locale.Lookup(yieldInfo.Name), source, Locale.Lookup(pDestinationCity:GetName()));
					info.TooltipText = (info.TooltipText ~= nil and info.TooltipText ~= "") and (info.TooltipText .. "[NEWLINE]" .. ligne) or ligne;
				end
			end
			return info;
		end
	end

	-- Le filtre par rendement trie les destinations par GetYieldForCity.
	if type(GetYieldForCity) == "function" and type(GetOriginCity) == "function" then
		local GetYieldForCity_jeu = GetYieldForCity;
		function GetYieldForCity(yieldIndex, city, originCity)
			local total, bFromRouteBonus = GetYieldForCity_jeu(yieldIndex, city, originCity);
			if originCity and city ~= nil then
				local ok, bonus = pcall(BonusDestination, GetOriginCity(), city);
				if ok and bonus[yieldIndex] ~= nil then
					total = (total or 0) + bonus[yieldIndex].total;
				end
			end
			return total, bFromRouteBonus;
		end
	end
end
