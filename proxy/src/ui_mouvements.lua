
-- ===========================================================================
-- Civ6RolloverProxy (src/ui_mouvements.lua), ajouté par le proxy à la fin de SelectedUnit.lua : les
-- locales m_MovementRange et m_MovementZoneOfControl du fichier restent visibles.
-- Zone de déplacement d'une unité posée sur un quartier défendable d'un joueur en guerre (centre-ville,
-- campement, ikanda, oppidum, thanh), terminé ou non. UnitManager.GetReachableMovement rend nil : le
-- parcours en largeur de GameCore (1801a6e00) applique à la case de départ la règle « entrer dans un
-- quartier ennemi qui a des points de vie arrête l'unité » (HitPoints de la base, sans regarder si le
-- quartier est terminé). GetReachableZonesOfControl (1800d11f0) passe par le même parcours : la
-- couche des zones de contrôle (cases rouges) est vide aussi. Le vrai déplacement (A*, celui du
-- survol et du clic droit) n'applique pas cette règle. On refait la zone avec
-- UnitManager.GetMoveToPathEx, au moindre coût :
--   - exploration par front : on ne teste que les voisins d'une case déjà atteignable (ou traversée),
--     dans la portée ; pas au-delà d'une case sous zone de contrôle ennemie (on y entre, on n'en
--     ressort pas ce tour), sauf si les chemins du jeu montrent que l'unité l'ignore ;
--   - chaque chemin sert plusieurs fois : ses cases atteintes ce tour-ci, sans unité dessus, sont
--     retenues sans nouveau chemin ;
--   - cases inaccessibles écartées sans chemin, d'après la vraie case de la carte, brouillard compris
--     (l'A* traite une case non révélée comme franchissable) : infranchissable sans tunnel ni route ;
--     eau pour une unité terrestre qui ne peut pas embarquer, océan si l'océan n'est pas débloqué ;
--     terre sans quartier pour une unité navale. Un chemin qui passe par une telle case est faux :
--     ni sa cible ni ses cases ne sont retenues ;
--   - zone mise en cache (joueur, unité, case, PM, tour, époque) : l'époque avance quand une unité
--     bouge, apparaît ou disparaît, ou qu'une guerre, une paix, une route ou un aménagement change ;
--   - budget de chemins et de temps : au-delà, on dessine ce qui est trouvé.
-- La couche des zones de contrôle est redessinée comme le jeu : cases de la zone sous zone de
-- contrôle ennemie et visibles, plus les indicateurs d'attaque.
-- Règles relues dans GameCore (180378a70, 1801928a0, 1803a30f0) : une case est sous zone de contrôle
-- si une case voisine porte un quartier ennemi (en guerre) à points de vie, terminé et non pillé, ou
-- une unité ennemie visible, vivante, du même domaine, qui exerce une zone de contrôle
-- (Units.ZoneOfControl), sans rivière entre les deux. La case d'où l'on part n'en exerce pas.
-- Voir analyses/quartier_capture_mouvements.
--
-- Mesure (réglage ui_mesure_mouvements, off par défaut : le proxy fait précéder ce bloc de
-- `local BGD_MOUV_MESURE = true|false;`, absent = on pour les tests hors jeu) : chaque appel de
-- RealizeMoveRadius est chronométré. Une ligne « BGD mouvements: » va dans
-- Lua.log quand le repli a tourné ou que l'appel dépasse BGD_MOUV_SEUIL_MS, au plus
-- BGD_MOUV_LIGNES_MAX lignes ; un bilan tous les BGD_MOUV_BILAN appels et à la fermeture du contexte.
-- Horloge du proxy (Civ6RolloverProxy_Clock) si elle est posée : os.clock de HavokScript n'avance que
-- par secondes entières.
-- ===========================================================================
local BGD_MOUV_ECRIRE = (BGD_MOUV_MESURE ~= false);
local BGD_MOUV_PORTEE_MAX = 10;
local BGD_MOUV_CHEMINS_MAX = 200;		-- chemins par calcul de zone (garde-fou sans horloge du proxy)
local BGD_MOUV_TEMPS_MAX = 0.05;		-- secondes par calcul de zone (horloge du proxy seulement)
local BGD_MOUV_SEUIL_MS = 5;
local BGD_MOUV_LIGNES_MAX = 150;
local BGD_MOUV_BILAN = 50;

-- État de la mesure : pile des appels en cours (RealizeMoveRadius peut être rappelée pendant qu'elle
-- tourne, par LensLayerOn), cause courante, totaux.
local BGD_Mesure = { pile = {}, cause = "direct", lignes = 0, appels = 0, replis = 0, cumul = 0, max = 0, maxRepli = 0, cumulRepli = 0, chemins = 0 };
-- Dernière zone calculée ; époque : avance à chaque changement de la carte qui peut changer un chemin.
local BGD_Cache = { cle = nil, zone = nil, zoc = nil, budget = false, epoque = 0 };
-- Accès débloqués par joueur et par tour (embarquement, océan).
local BGD_Acces = {};

local function BGD_Horloge()
	local f = Civ6RolloverProxy_Clock;
	if f ~= nil then return f(), "proxy"; end
	return os.clock(), "os.clock";
end

local function BGD_MesureCourante()
	return BGD_Mesure.pile[#BGD_Mesure.pile];
end

local function BGD_Vrai(v)
	return v == true or v == 1 or v == "true" or v == "1";
end

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

-- ---------------------------------------------------------------------------
-- Accès : embarquement et océan, lus dans la base de façon générique (BBG et mods compris).
-- Dans le doute, l'accès est accordé : on ne filtre que ce qui est sûrement interdit.
-- ---------------------------------------------------------------------------
local function BGD_ModifiersDe(sTable, sCol, kTypes)
	local kIds = {};
	local t = GameInfo[sTable];
	if t == nil then return kIds; end
	for row in t() do
		if kTypes[row[sCol]] then kIds[row.ModifierId] = true; end
	end
	return kIds;
end

local function BGD_TypeDeModifier(sId)
	local kMod = GameInfo.Modifiers and GameInfo.Modifiers[sId];
	return (kMod and kMod.ModifierType) or "";
end

-- Traits du joueur (civilisation et dirigeant).
local function BGD_Traits(ePlayer)
	local kTraits = {};
	local pConfig = PlayerConfigurations and PlayerConfigurations[ePlayer];
	if pConfig == nil then return kTraits; end
	local sCiv, sLeader = pConfig:GetCivilizationTypeName(), pConfig:GetLeaderTypeName();
	if GameInfo.CivilizationTraits ~= nil then
		for row in GameInfo.CivilizationTraits() do if row.CivilizationType == sCiv then kTraits[row.TraitType] = true; end end
	end
	if GameInfo.LeaderTraits ~= nil then
		for row in GameInfo.LeaderTraits() do if row.LeaderType == sLeader then kTraits[row.TraitType] = true; end end
	end
	return kTraits;
end

local function BGD_CalculerAcces(ePlayer, sUnitType)
	local pPlayer = Players[ePlayer];
	local kAcces = { embarque = {}, ocean = false };
	local pTechs = pPlayer and pPlayer.GetTechs and pPlayer:GetTechs();
	local pCulture = pPlayer and pPlayer.GetCulture and pPlayer:GetCulture();
	local function ATech(row) return pTechs ~= nil and pTechs:HasTech(row.Index); end
	local function ACivic(row) return pCulture ~= nil and pCulture:HasCivic(row.Index); end
	-- Embarquement : technologies et dogmes EmbarkAll / EmbarkUnitType.
	local bTous, kTypes = false, {};
	for _, k in ipairs({ { "Technologies", ATech }, { "Civics", ACivic } }) do
		if GameInfo[k[1]] ~= nil then
			for row in GameInfo[k[1]]() do
				if (BGD_Vrai(row.EmbarkAll) or row.EmbarkUnitType ~= nil) and k[2](row) then
					if BGD_Vrai(row.EmbarkAll) then bTous = true; end
					if row.EmbarkUnitType ~= nil then kTypes[row.EmbarkUnitType] = true; end
				end
			end
		end
	end
	-- Doute : paramètre global, ou un trait du joueur qui touche à l'embarquement (Maoris…).
	local kGlobal = GameInfo.GlobalParameters and GameInfo.GlobalParameters["UNLOCK_EMBARKATION"];
	if kGlobal ~= nil and BGD_Vrai(kGlobal.Value) then bTous = true; end
	local kTraits = BGD_Traits(ePlayer);
	local kTraitMods = BGD_ModifiersDe("TraitModifiers", "TraitType", kTraits);
	for sId in pairs(kTraitMods) do
		local sType = BGD_TypeDeModifier(sId);
		if string.find(sId, "EMBARK", 1, true) or string.find(sType, "EMBARK", 1, true) then bTous = true; end
	end
	kAcces.embarqueTous, kAcces.embarqueTypes = bTous, kTypes;
	-- Océan : un modificateur *VALID_TERRAIN* (Cartographie, Norvège, Maoris, Leif Erikson…) venu
	-- d'une technologie, d'un dogme, d'un trait ou d'un personnage illustre recruté.
	local function Ocean(sId) return string.find(BGD_TypeDeModifier(sId), "VALID_TERRAIN", 1, true) ~= nil; end
	for sId in pairs(kTraitMods) do if Ocean(sId) then kAcces.ocean = true; end end
	for _, k in ipairs({ { "Technologies", "TechnologyModifiers", "TechnologyType", ATech }, { "Civics", "CivicModifiers", "CivicType", ACivic } }) do
		if not kAcces.ocean and GameInfo[k[1]] ~= nil and GameInfo[k[2]] ~= nil then
			local kAcquis = {};
			for row in GameInfo[k[1]]() do if k[4](row) then kAcquis[row[k[3]]] = true; end end
			for sId in pairs(BGD_ModifiersDe(k[2], k[3], kAcquis)) do if Ocean(sId) then kAcces.ocean = true; end end
		end
	end
	if not kAcces.ocean then
		local ok, kTimeline = pcall(function() return Game.GetGreatPeople():GetPastTimeline(); end);
		if ok and kTimeline ~= nil then
			local kRecrues = {};
			for _, e in ipairs(kTimeline) do
				if e.Claimant == ePlayer and e.Individual ~= nil then
					local kInd = GameInfo.GreatPersonIndividuals and GameInfo.GreatPersonIndividuals[e.Individual];
					if kInd ~= nil then kRecrues[kInd.GreatPersonIndividualType] = true; end
				end
			end
			for _, sTable in ipairs({ "GreatPersonIndividualActionModifiers", "GreatPersonIndividualBirthModifiers" }) do
				for sId in pairs(BGD_ModifiersDe(sTable, "GreatPersonIndividualType", kRecrues)) do
					if Ocean(sId) then kAcces.ocean = true; end
				end
			end
		else
			kAcces.ocean = true;		-- historique illisible : dans le doute
		end
	end
	return kAcces;
end

-- Contexte du calcul : domaine de l'unité, accès, case de départ.
local function BGD_Contexte(kUnit)
	local ePlayer = kUnit:GetOwner();
	local kInfo = GameInfo.Units[kUnit:GetUnitType()];
	local sCle = tostring(ePlayer) .. ":" .. tostring(Game.GetCurrentGameTurn());
	if BGD_Acces.cle ~= sCle then BGD_Acces = { cle = sCle }; end
	local ok, kAcces = true, BGD_Acces.acces;
	if kAcces == nil then
		ok, kAcces = pcall(BGD_CalculerAcces, ePlayer, kInfo and kInfo.UnitType);
		if not ok then kAcces = nil; end
		BGD_Acces.acces = kAcces;
	end
	local ctx = { ePlayer = ePlayer, sDomaine = kInfo and kInfo.Domain or "DOMAIN_LAND", embarque = true, ocean = true };
	if kAcces ~= nil then
		ctx.embarque = kAcces.embarqueTous or kAcces.embarqueTypes[kInfo and kInfo.UnitType or ""] == true;
		ctx.ocean = kAcces.ocean;
	end
	if kUnit.IsEmbarked ~= nil and kUnit:IsEmbarked() then ctx.embarque = true; end
	local kOcean = GameInfo.Terrains and GameInfo.Terrains["TERRAIN_OCEAN"];
	ctx.iOcean = kOcean and kOcean.Index;
	ctx.iDepart = Map.GetPlot(kUnit:GetX(), kUnit:GetY()):GetIndex();
	ctx.pDiplo = Players[ePlayer]:GetDiplomacy();
	ctx.pVis = PlayersVisibility and PlayersVisibility[ePlayer];
	ctx.kZoc = {};
	return ctx;
end

-- Case où l'unité ne peut sûrement pas entrer, d'après la vraie case (brouillard compris).
local function BGD_Inaccessible(ctx, pPlot)
	if pPlot.IsImpassable ~= nil and pPlot:IsImpassable() then
		local eImpr = pPlot.GetImprovementType and pPlot:GetImprovementType();
		local eRoute = pPlot.GetRouteType and pPlot:GetRouteType();
		if (eImpr == nil or eImpr < 0) and (eRoute == nil or eRoute < 0) then return true; end
	end
	if pPlot.IsWater == nil then return false; end
	if ctx.sDomaine == "DOMAIN_LAND" then
		if pPlot:IsWater() then
			if not ctx.embarque then return true; end
			if not ctx.ocean and ctx.iOcean ~= nil and pPlot.GetTerrainType ~= nil and pPlot:GetTerrainType() == ctx.iOcean then return true; end
		end
	elseif ctx.sDomaine == "DOMAIN_SEA" then
		if not pPlot:IsWater() and CityManager.GetDistrictAt(pPlot) == nil then return true; end
	end
	return false;
end

-- ---------------------------------------------------------------------------
-- Zones de contrôle (GameCore 180378a70).
-- ---------------------------------------------------------------------------
-- Rivière entre pA et sa voisine pB dans la direction dir (bords de rivière portés par la case à
-- l'ouest, au nord-ouest ou au nord-est). Méthodes absentes : pas de rivière.
local function BGD_Riviere(pA, dir, pB)
	local D = DirectionTypes;
	if pA.IsWOfRiver == nil or D == nil then return false; end
	if dir == D.DIRECTION_EAST then return pA:IsWOfRiver(); end
	if dir == D.DIRECTION_WEST then return pB:IsWOfRiver(); end
	if dir == D.DIRECTION_SOUTHEAST then return pA:IsNWOfRiver(); end
	if dir == D.DIRECTION_NORTHWEST then return pB:IsNWOfRiver(); end
	if dir == D.DIRECTION_SOUTHWEST then return pA:IsNEOfRiver(); end
	if dir == D.DIRECTION_NORTHEAST then return pB:IsNEOfRiver(); end
	return false;
end

local function BGD_EnGuerre(ctx, eOwner)
	return eOwner ~= nil and eOwner ~= -1 and eOwner ~= ctx.ePlayer and ctx.pDiplo:IsAtWarWith(eOwner);
end

-- Quartier ennemi qui exerce une zone de contrôle : points de vie (base), terminé, non pillé.
local function BGD_QuartierZoc(ctx, pDistrict)
	local kInfo = GameInfo.Districts[pDistrict:GetType()];
	if kInfo == nil or kInfo.HitPoints == nil or kInfo.HitPoints < 1 then return false; end
	if pDistrict.IsComplete ~= nil and not pDistrict:IsComplete() then return false; end
	if pDistrict.IsPillaged ~= nil and pDistrict:IsPillaged() then return false; end
	return BGD_EnGuerre(ctx, pDistrict:GetOwner());
end

-- Unité ennemie qui exerce une zone de contrôle sur l'unité déplacée.
local function BGD_UniteZoc(ctx, pUnit)
	if not BGD_EnGuerre(ctx, pUnit:GetOwner()) then return false; end
	local kInfo = GameInfo.Units[pUnit:GetUnitType()];
	if kInfo == nil or not BGD_Vrai(kInfo.ZoneOfControl) or (kInfo.Domain or "DOMAIN_LAND") ~= ctx.sDomaine then return false; end
	if pUnit.IsEmbarked ~= nil and pUnit:IsEmbarked() then return false; end
	if pUnit.GetDamage ~= nil and pUnit.GetMaxDamage ~= nil and pUnit:GetDamage() >= pUnit:GetMaxDamage() then return false; end
	if pUnit.IsDelayedDeath ~= nil and pUnit:IsDelayedDeath() then return false; end
	return ctx.pVis == nil or ctx.pVis.IsUnitVisible == nil or ctx.pVis:IsUnitVisible(pUnit);
end

local function BGD_SousZoc(ctx, pPlot)
	local iPlot = pPlot:GetIndex();
	if ctx.kZoc[iPlot] ~= nil then return ctx.kZoc[iPlot]; end
	local bZoc = false;
	local x, y = pPlot:GetX(), pPlot:GetY();
	for dir = 0, DirectionTypes.NUM_DIRECTION_TYPES - 1 do
		local pN = Map.GetAdjacentPlot(x, y, dir);
		if pN ~= nil and pN:GetIndex() ~= ctx.iDepart then
			local pDistrict = CityManager.GetDistrictAt(pN);
			if pDistrict ~= nil and BGD_QuartierZoc(ctx, pDistrict) then bZoc = true; break; end
			if not BGD_Riviere(pPlot, dir, pN) then
				local kUnits = Units.GetUnitsInPlotLayerID(pN:GetX(), pN:GetY(), MapLayers.ANY) or {};
				for _, pUnit in ipairs(kUnits) do
					if BGD_UniteZoc(ctx, pUnit) then bZoc = true; break; end
				end
				if bZoc then break; end
			end
		end
	end
	ctx.kZoc[iPlot] = bZoc;
	return bZoc;
end

-- Unités sur la case (toutes couches, comme le survol de WorldInput.lua). Dans le doute : occupée.
local function BGD_Occupee(pPlot)
	if Units ~= nil and Units.GetUnitsInPlotLayerID ~= nil and MapLayers ~= nil then
		local kUnits = Units.GetUnitsInPlotLayerID(pPlot:GetX(), pPlot:GetY(), MapLayers.ANY);
		return kUnits == nil or table.count(kUnits) > 0;
	end
	if pPlot.GetUnitCount ~= nil then return pPlot:GetUnitCount() > 0; end
	return true;
end

-- Chemin du survol vers iPlot : (atteinte ce tour-ci, chemin). turns commence à 1 = ce tour.
local function BGD_Chemin(kUnit, iPlot)
	local m = BGD_MesureCourante();
	local t0 = m and BGD_Horloge();
	local kPath = UnitManager.GetMoveToPathEx(kUnit, iPlot);
	if m then
		m.chemins = m.chemins + 1;
		m.cheminsS = m.cheminsS + (BGD_Horloge() - t0);
	end
	if kPath == nil or kPath.plots == nil or kPath.turns == nil then return false, nil; end
	local n = table.count(kPath.plots);
	local t = table.count(kPath.turns);
	return n > 1 and t > 0 and kPath.plots[n] == iPlot and kPath.turns[t] <= 1, kPath;
end

local function BGD_ZoneParChemins(kUnit, ctx)
	local m = BGD_MesureCourante() or { chemins = 0, cheminsS = 0 };
	local t0 = BGD_Horloge();
	local bTemps = Civ6RolloverProxy_Clock ~= nil;
	local x, y = kUnit:GetX(), kUnit:GetY();
	local iMoves = math.max(1, math.ceil(kUnit:GetMovesRemaining()));
	local iPortee = math.min(BGD_MOUV_PORTEE_MAX, iMoves * 4);	-- chemin de fer : 1/4 de PM par case
	m.portee = iPortee;
	m.arret = "front_epuise";
	m.anneaux = 0;
	m.filtrees = 0;
	m.faux = 0;
	local iDepart = ctx.iDepart;
	local kZone = { iDepart };
	local kVu = { [iDepart] = true };		-- case retenue, testée ou écartée : plus de chemin vers elle
	local kSource = { [iDepart] = true };	-- case d'où l'on teste les voisins
	local kFile = { iDepart };
	local kBloquees = {};					-- cases retenues sous zone de contrôle : pas sources
	local bZocIgnoree = false;				-- un chemin du jeu traverse une zone de contrôle ce tour-ci
	local iTete = 1;
	local nChemins0 = m.chemins;
	local kInaccessible = {};

	local function Inaccessible(pPlot)
		local i = pPlot:GetIndex();
		if kInaccessible[i] == nil then kInaccessible[i] = BGD_Inaccessible(ctx, pPlot); end
		return kInaccessible[i];
	end
	local function DansPortee(pPlot)
		local d = Map.GetPlotDistance(x, y, pPlot:GetX(), pPlot:GetY());
		return d > 0 and d <= iPortee, d;
	end
	local function Source(iPlot)
		if not kSource[iPlot] then
			kSource[iPlot] = true;
			table.insert(kFile, iPlot);
		end
	end
	local function IgnorerZoc()
		if bZocIgnoree then return; end
		bZocIgnoree = true;
		for _, iPlot in ipairs(kBloquees) do Source(iPlot); end
	end
	-- Une case retenue est source du front, sauf sous zone de contrôle ennemie.
	local function Retenir(iPlot, pPlot)
		table.insert(kZone, iPlot);
		if bZocIgnoree or not BGD_SousZoc(ctx, pPlot) then Source(iPlot); else table.insert(kBloquees, iPlot); end
	end
	-- Chemin faux : il passe par une case inaccessible (case non révélée prise pour franchissable).
	local function Faux(kPath)
		for i = 2, table.count(kPath.plots) do
			local pPlot = Map.GetPlotByIndex(kPath.plots[i]);
			if pPlot ~= nil and Inaccessible(pPlot) then return true; end
		end
		return false;
	end
	-- Cases du chemin atteintes ce tour-ci : retenues si personne n'y est (on peut s'y arrêter), et
	-- dans tous les cas sources du front (une case traversée mène à ses voisines). Une case sous zone
	-- de contrôle suivie d'une autre case du même tour : l'unité ignore les zones de contrôle.
	local function Reutiliser(kPath)
		local n = table.count(kPath.plots);
		if table.count(kPath.turns) ~= n then return; end
		for i = 2, n - 1 do
			local iPlot = kPath.plots[i];
			if kPath.turns[i] ~= nil and kPath.turns[i] <= 1 then
				local pPlot = Map.GetPlotByIndex(iPlot);
				if pPlot ~= nil then
					if kPath.turns[i + 1] ~= nil and kPath.turns[i + 1] <= 1 and BGD_SousZoc(ctx, pPlot) then IgnorerZoc(); end
					if DansPortee(pPlot) then
						if not kVu[iPlot] and not BGD_Occupee(pPlot) then
							kVu[iPlot] = true;
							Retenir(iPlot, pPlot);
						elseif bZocIgnoree or not BGD_SousZoc(ctx, pPlot) then
							Source(iPlot);
						end
					end
				end
			end
		end
	end

	while iTete <= #kFile do
		local pSrc = Map.GetPlotByIndex(kFile[iTete]);
		iTete = iTete + 1;
		for dir = 0, DirectionTypes.NUM_DIRECTION_TYPES - 1 do
			local pPlot = Map.GetAdjacentPlot(pSrc:GetX(), pSrc:GetY(), dir);
			local iPlot = pPlot and pPlot:GetIndex();
			if iPlot ~= nil and not kVu[iPlot] then
				local bDans, d = DansPortee(pPlot);
				if bDans then
					kVu[iPlot] = true;
					if d > m.anneaux then m.anneaux = d; end
					if Inaccessible(pPlot) then
						m.filtrees = m.filtrees + 1;
					elseif m.chemins - nChemins0 >= BGD_MOUV_CHEMINS_MAX or (bTemps and BGD_Horloge() - t0 > BGD_MOUV_TEMPS_MAX) then
						m.arret = "budget";
						m.budget = true;
						return kZone, true;
					else
						local bAtteinte, kPath = BGD_Chemin(kUnit, iPlot);
						if kPath ~= nil and Faux(kPath) then
							m.faux = m.faux + 1;
						else
							if kPath ~= nil then Reutiliser(kPath); end
							if bAtteinte then
								Retenir(iPlot, pPlot);
							elseif BGD_Occupee(pPlot) and (bZocIgnoree or not BGD_SousZoc(ctx, pPlot)) then
								Source(iPlot);		-- peut-être traversable sans s'y arrêter
							end
						end
					end
				end
			end
		end
	end
	return kZone, false;
end

-- Cases de la zone sous zone de contrôle ennemie et visibles (GetReachableZonesOfControl(kUnit, true)).
local function BGD_ZocDeZone(ctx, kZone)
	local kZoc = {};
	for _, iPlot in ipairs(kZone) do
		local pPlot = Map.GetPlotByIndex(iPlot);
		if iPlot ~= ctx.iDepart and pPlot ~= nil and BGD_SousZoc(ctx, pPlot)
			and (ctx.pVis == nil or ctx.pVis.IsVisible == nil or ctx.pVis:IsVisible(pPlot:GetX(), pPlot:GetY())) then
			table.insert(kZoc, iPlot);
		end
	end
	return kZoc;
end

-- Zone de la dernière demande si rien n'a changé depuis, sinon calculée.
local function BGD_Zone(kUnit, m)
	local cle = table.concat({ tostring(kUnit:GetOwner()), tostring(kUnit:GetID()), tostring(kUnit:GetX()), tostring(kUnit:GetY()),
		tostring(kUnit:GetMovesRemaining()), tostring(Game.GetCurrentGameTurn()), tostring(BGD_Cache.epoque) }, ":");
	if BGD_Cache.cle == cle then
		m.cache = true;
		m.arret = "cache";
		m.budget = BGD_Cache.budget;
		return BGD_Cache.zone, BGD_Cache.zoc;
	end
	local ctx = BGD_Contexte(kUnit);
	m.embarque, m.ocean = ctx.embarque, ctx.ocean;
	local kZone, bBudget = BGD_ZoneParChemins(kUnit, ctx);
	local kZoc = BGD_ZocDeZone(ctx, kZone);
	BGD_Cache.cle, BGD_Cache.zone, BGD_Cache.zoc, BGD_Cache.budget = cle, kZone, kZoc, bBudget;
	return kZone, kZoc;
end

-- Indicateurs d'attaque, comme RealizeMoveRadius du jeu (cibles au contact et à distance).
local function BGD_IndicateursAttaque(kUnit, kZone)
	local kIndicateurs = {};
	if kUnit:GetAttacksRemaining() > 0 then
		for _, iPlot in ipairs(UnitManager.GetReachableTargets(kUnit) or {}) do
			table.insert(kIndicateurs, { "AttackRange_Target", iPlot });
			table.insert(kZone, iPlot);
		end
	end
	local pResults = UnitManager.GetOperationTargets(kUnit, UnitOperationTypes.RANGE_ATTACK);
	local pAllPlots = pResults and pResults[UnitOperationResults.PLOTS];
	if pAllPlots ~= nil then
		for i, modifier in ipairs(pResults[UnitOperationResults.MODIFIERS] or {}) do
			if modifier == UnitOperationResults.MODIFIER_IS_TARGET then
				local bUnique = true;
				for _, v in ipairs(kIndicateurs) do if v[2] == pAllPlots[i] then bUnique = false; break; end end
				if bUnique then table.insert(kIndicateurs, { "AttackRange_Target", pAllPlots[i] }); end
			end
		end
	end
	return kIndicateurs;
end

-- Le corps du bloc ; m.raison dit où il s'est arrêté.
local BGD_RealizeMoveRadius_Jeu = RealizeMoveRadius;
local function BGD_RealizeMoveRadius(kUnit, m)
	local t0 = BGD_Horloge();
	BGD_RealizeMoveRadius_Jeu(kUnit);
	m.jeuS = BGD_Horloge() - t0;
	if kUnit == nil then m.raison = "sans_unite"; return; end
	if UI.IsGameCoreBusy() then m.raison = "gamecore_occupe"; return; end
	local kInfo = GameInfo.Units[kUnit:GetUnitType()];
	if kInfo == nil or kInfo.IgnoreMoves or kInfo.Spy or kInfo.MakeTradeRoute then m.raison = "type_unite"; return; end
	if not (kUnit:GetMovesRemaining() > 0) then m.raison = "sans_pm"; return; end
	if kUnit:HasMovedIntoZOC() then m.raison = "zoc"; return; end
	local kMove = UnitManager.GetReachableMovement(kUnit);
	if kMove ~= nil and table.count(kMove) > 1 then m.raison = "zone_du_jeu"; return; end		-- le jeu a trouvé une zone
	if not BGD_SurQuartierEnnemiDefendable(kUnit) then m.raison = "pas_quartier_ennemi"; return; end
	m.repli = true;
	local kTrouvee, kZocTrouvee = BGD_Zone(kUnit, m);
	m.zone = table.count(kTrouvee);
	m.zoc = table.count(kZocTrouvee or {});
	if m.zone <= 1 then m.raison = "repli_vide"; return; end
	m.raison = "repli";
	local kZone = {};
	for _, iPlot in ipairs(kTrouvee) do table.insert(kZone, iPlot); end		-- le cache reste intact
	local kIndicateurs = BGD_IndicateursAttaque(kUnit, kZone);
	local eLocal = Game.GetLocalPlayer();
	if m_MovementZoneOfControl ~= nil and m.zoc > 0 then
		local kZoc = {};
		for _, iPlot in ipairs(kZocTrouvee) do table.insert(kZoc, iPlot); end
		UILens.ClearLayerHexes(m_MovementZoneOfControl);
		UILens.SetLayerHexesArea(m_MovementZoneOfControl, eLocal, kZoc, kIndicateurs);
	end
	UILens.ClearLayerHexes(m_MovementRange);
	UILens.SetLayerHexesArea(m_MovementRange, eLocal, kZone);
end

-- Champs du quartier sous l'unité, lus seulement pour écrire la ligne (hors chronométrage).
local function BGD_InfoUnite(kUnit)
	if kUnit == nil then return "unite=- case=- quartier=-"; end
	local s = string.format("unite=%s case=%s,%s pm=%s", tostring(kUnit:GetID()), tostring(kUnit:GetX()), tostring(kUnit:GetY()), tostring(kUnit:GetMovesRemaining()));
	local pPlot = Map.GetPlot(kUnit:GetX(), kUnit:GetY());
	local pDistrict = pPlot and CityManager.GetDistrictAt(pPlot);
	if pDistrict == nil then return s .. " quartier=-"; end
	local kInfo = GameInfo.Districts[pDistrict:GetType()];
	local eOwner = pDistrict:GetOwner();
	local pPlayer = Players[kUnit:GetOwner()];
	local bGuerre = eOwner ~= -1 and eOwner ~= kUnit:GetOwner() and pPlayer ~= nil and pPlayer:GetDiplomacy():IsAtWarWith(eOwner);
	local bFini = "?";
	if pDistrict.IsComplete ~= nil then bFini = pDistrict:IsComplete() and "1" or "0"; end
	return string.format("%s quartier=%s pv_base=%s proprio=%s unite_proprio=%s guerre=%s fini=%s", s,
		kInfo and kInfo.DistrictType or tostring(pDistrict:GetType()), tostring(kInfo and kInfo.HitPoints),
		tostring(eOwner), tostring(kUnit:GetOwner()), bGuerre and "1" or "0", bFini);
end

local function BGD_Bilan(quand)
	local M = BGD_Mesure;
	print(string.format("BGD mouvements: bilan(%s) appels=%d replis=%d cumul_ms=%.1f max_ms=%.1f repli_cumul_ms=%.1f repli_max_ms=%.1f chemins=%d lignes=%d/%d",
		quand, M.appels, M.replis, M.cumul * 1000, M.max * 1000, M.cumulRepli * 1000, M.maxRepli * 1000, M.chemins, M.lignes, BGD_MOUV_LIGNES_MAX));
end

local function BGD_Drapeau(v)
	if v == nil then return "-"; end
	return v and "1" or "0";
end

function RealizeMoveRadius( kUnit:table )
	local M = BGD_Mesure;
	local m = { chemins = 0, cheminsS = 0, jeuS = 0, repli = false, cache = false, budget = false, filtrees = 0, faux = 0, zoc = 0, portee = 0, anneaux = 0, zone = 0, arret = "-", raison = "?", cause = M.cause, prof = #M.pile + 1 };
	table.insert(M.pile, m);
	local t0, horloge = BGD_Horloge();
	local ok, err = pcall(BGD_RealizeMoveRadius, kUnit, m);
	local dt = BGD_Horloge() - t0;
	table.remove(M.pile);
	if not ok then
		print("BGD mouvements: echec " .. tostring(err));
		return;
	end
	M.appels = M.appels + 1;
	M.cumul = M.cumul + dt;
	if dt > M.max then M.max = dt; end
	M.chemins = M.chemins + m.chemins;
	if m.repli then
		M.replis = M.replis + 1;
		M.cumulRepli = M.cumulRepli + dt;
		if dt > M.maxRepli then M.maxRepli = dt; end
	end
	if BGD_MOUV_ECRIRE and (m.repli or dt * 1000 > BGD_MOUV_SEUIL_MS) and M.lignes < BGD_MOUV_LIGNES_MAX then
		M.lignes = M.lignes + 1;
		local okInfo, info = pcall(BGD_InfoUnite, kUnit);
		print(string.format("BGD mouvements: n=%d cause=%s prof=%d ms=%.1f jeu_ms=%.1f repli=%d raison=%s portee=%d anneaux=%d arret=%s chemins=%d chemins_ms=%.1f zone=%d cache=%d filtrees=%d budget=%d faux=%d zoc=%d embarque=%s ocean=%s %s horloge=%s",
			M.appels, m.cause, m.prof, dt * 1000, m.jeuS * 1000, m.repli and 1 or 0, m.raison, m.portee, m.anneaux, m.arret,
			m.chemins, m.cheminsS * 1000, m.zone, m.cache and 1 or 0, m.filtrees, m.budget and 1 or 0, m.faux, m.zoc,
			BGD_Drapeau(m.embarque), BGD_Drapeau(m.ocean), okInfo and info or ("info_echec=" .. tostring(info)), horloge));
		if M.lignes == BGD_MOUV_LIGNES_MAX then
			print("BGD mouvements: plafond de " .. BGD_MOUV_LIGNES_MAX .. " lignes atteint, seuls les bilans suivent");
		end
	end
	if BGD_MOUV_ECRIRE and M.appels % BGD_MOUV_BILAN == 0 then BGD_Bilan(tostring(M.appels)); end
end

-- Cause des appels : les gestionnaires d'événements qui appellent RealizeMoveRadius sont réinscrits
-- enveloppés (même fonction d'origine, appelée telle quelle) pour nommer l'événement en cours.
-- Époque du cache : avancée avant le gestionnaire (unité retirée, téléportée) ou par un écouteur.
do
	local kEnveloppes = {};
	local function NouvelleEpoque()
		BGD_Cache.epoque = BGD_Cache.epoque + 1;
	end
	local function Envelopper(kEvent, nom, fn, bEpoque)
		if kEvent == nil or fn == nil then return; end
		local function f(...)
			if bEpoque then NouvelleEpoque(); end
			local avant = BGD_Mesure.cause;
			BGD_Mesure.cause = nom;
			local ok, err = pcall(fn, ...);
			BGD_Mesure.cause = avant;
			if not ok then error(err, 0); end
		end
		kEvent.Remove(fn);
		kEvent.Add(f);
		table.insert(kEnveloppes, { kEvent, f });
	end
	local function Ecouter(kEvent)
		if kEvent == nil then return; end
		kEvent.Add(NouvelleEpoque);
		table.insert(kEnveloppes, { kEvent, NouvelleEpoque });
	end
	local ok, err = pcall(function()
		Envelopper(Events.UnitSelectionChanged, "selection", OnUnitSelectionChanged);
		Envelopper(Events.UnitSimPositionChanged, "fin_mouvement", UnitSimPositionChanged);
		Envelopper(Events.LensLayerOn, "LensLayerOn", OnLensLayerOn);
		Envelopper(Events.PlayerTurnActivated, "debut_tour", OnPlayerTurnActivated);
		Envelopper(Events.UnitPromoted, "promotion", OnUnitPromotionChanged);
		Envelopper(Events.UnitTeleported, "teleportation", OnUnitTeleported, true);
		Envelopper(Events.UnitMovementPointsChanged, "MovementPointsChanged", OnUnitMovementPointsChanged);
		Envelopper(Events.UnitRemovedFromMap, "unite_retiree", OnUnitRemovedFromMap, true);
		Ecouter(Events.UnitMoveComplete);
		Ecouter(Events.UnitAddedToMap);
		Ecouter(Events.UnitVisibilityChanged);
		Ecouter(Events.UnitEmbarkedStateChanged);
		Ecouter(Events.DiplomacyDeclareWar);
		Ecouter(Events.DiplomacyMakePeace);
		Ecouter(Events.ImprovementAddedToMap);
		Ecouter(Events.RouteAddedToMap);
		Ecouter(Events.DistrictPillaged);
		-- Technologie ou dogme : les accès (embarquement, océan) sont relus.
		for _, kEvent in ipairs({ Events.ResearchCompleted, Events.CivicCompleted }) do
			local function f() BGD_Acces = {}; NouvelleEpoque(); end
			if kEvent ~= nil then
				kEvent.Add(f);
				table.insert(kEnveloppes, { kEvent, f });
			end
		end
		-- À la fermeture : bilan, enveloppes et écouteurs retirés, puis la fermeture du fichier.
		local BGD_Fermeture_Jeu = OnShutdown;
		ContextPtr:SetShutdown(function()
			if BGD_MOUV_ECRIRE then pcall(BGD_Bilan, "fermeture"); end
			for _, e in ipairs(kEnveloppes) do pcall(e[1].Remove, e[2]); end
			if BGD_Fermeture_Jeu ~= nil then BGD_Fermeture_Jeu(); end
		end);
	end);
	if not ok then print("BGD mouvements: causes non suivies " .. tostring(err)); end
end
