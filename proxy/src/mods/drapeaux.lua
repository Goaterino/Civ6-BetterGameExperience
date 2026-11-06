-- ===========================================================================
-- BetterGameDLLs (src/mods/drapeaux.lua), ajouté par le proxy à la fin de UnitFlagManager.lua (celui
-- du jeu ou celui de BSM), précédé de `local BGD_Charges, BGD_Routes = <bool>, <bool>;`.
-- Le macaron des promotions du drapeau (Promotion_Flag / UnitNumPromotions) montre :
-- - ui_charges : les charges restantes d'une unité qui en a (bâtisseur, ingénieur militaire, légion
--   romaine, unités de mods), d'après Better Builder Charges Tracking (wltk). Corrigé : le macaron est
--   d'abord masqué comme dans le jeu (drapeaux recyclés), et toute unité dont le type a des charges
--   est couverte. S'il n'en reste plus, les promotions reviennent ;
-- - ui_routes : sur un négociant du joueur local en route, les tours avant la fin de sa route
--   commerciale. La durée minimale et le départ viennent du proxy : Civ6Proxy_FinRoute(origine
--   joueur, ville, destination joueur, ville, négociant joueur, unité) rend (fin, départ), lus dans
--   le gestionnaire de commerce de GameCore (départ + TRADE_ROUTE_TURN_DURATION_BASE et
--   Eras_XP2.TradeRouteMinimumEndTurnChange de l'ère courante, à l'échelle de la vitesse de jeu :
--   15 tours en ligne à l'ère médiévale). La route finit au premier retour du négociant chez lui après cette fin (BGD_ToursRestants).
-- Affichage seulement : rien ne part sur le réseau.
-- ===========================================================================
do
	local BGD_UpdatePromotions_jeu = UnitFlag.UpdatePromotions;
	local function BGD_Charges_De(unit)
		if not BGD_Charges then return nil; end
		local info = GameInfo.Units[unit:GetUnitType()];
		if info == nil or info.BuildCharges == nil or info.BuildCharges <= 0 then return nil; end
		local n = unit:GetBuildCharges();
		if n == nil or n <= 0 then return nil; end
		return n;
	end

	-- Tours avant la fin de la route. Règle du jeu (Civilopédie, et 18066bf70 dans GameCore) : la
	-- route dure au moins jusqu'à `fin`, puis s'arrête au premier retour du négociant sur la case de
	-- sa ville d'origine. Sur une route, il avance d'une case par tour : un aller-retour dure 2 × L
	-- tours, L étant le nombre de pas du trajet (GetTradeRoutePath). Sa place sur le trajet, et le
	-- temps écoulé depuis le départ, disent s'il va ou revient. Sans trajet lisible : la durée
	-- minimale seule.
	local function BGD_ToursRestants(unit, route, depart, fin, tour)
		local minimum = math.max(0, fin - tour);
		local tm = Game.GetTradeManager and Game.GetTradeManager();
		if tm == nil or tm.GetTradeRoutePath == nil then return minimum; end
		local ok, chemin = pcall(function()
			return tm:GetTradeRoutePath(route.OriginCityPlayer, route.OriginCityID, route.DestinationCityPlayer, route.DestinationCityID);
		end);
		if not ok or type(chemin) ~= "table" or #chemin < 2 then return minimum; end
		local pas = #chemin - 1;
		local cycle = 2 * pas;
		local ici = Map.GetPlotIndex(unit:GetX(), unit:GetY());
		local position = nil;
		for i, plot in ipairs(chemin) do
			if plot == ici then position = i - 1; break; end
		end
		local phase = (tour - depart) % cycle;
		local retour;
		if position == nil then
			retour = (cycle - phase) % cycle;
		elseif position == 0 then
			retour = 0;						-- chez lui : fin, ou nouvel aller-retour
		elseif phase < pas then
			retour = cycle - position;		-- à l'aller
		else
			retour = position;				-- au retour
		end
		local reste = retour;
		while tour + reste < fin do reste = reste + cycle; end
		return reste;
	end

	-- Tours restants de la route du négociant, et son texte d'aide ; nil hors route.
	local function BGD_Route_De(unit)
		if not BGD_Routes or Civ6Proxy_FinRoute == nil then return nil; end
		local owner = unit:GetOwner();
		if owner ~= Game.GetLocalPlayer() then return nil; end
		local info = GameInfo.Units[unit:GetUnitType()];
		if info == nil or not info.MakeTradeRoute then return nil; end
		local unitID = unit:GetID();
		local pPlayer = Players[owner];
		if pPlayer == nil then return nil; end
		for _, pCity in pPlayer:GetCities():Members() do
			for _, route in ipairs(pCity:GetTrade():GetOutgoingRoutes()) do
				if route.TraderUnitID == unitID and route.TraderUnitPlayer == owner then
					local fin, depart = Civ6Proxy_FinRoute(route.OriginCityPlayer, route.OriginCityID,
						route.DestinationCityPlayer, route.DestinationCityID, owner, unitID);
					if fin == nil then return nil; end
					local reste = BGD_ToursRestants(unit, route, depart or fin, fin, Game.GetCurrentGameTurn());
					local aide = BGD_Txt("LOC_BGD_ROUTE_AIDE", reste);
					return reste, aide;
				end
			end
		end
		return nil;
	end

	function UnitFlag.UpdatePromotions(self)
		local unit = self:GetUnit();
		if unit ~= nil and unit:GetUnitType() ~= -1 then
			local n, aide = BGD_Charges_De(unit), nil;
			if n == nil then n, aide = BGD_Route_De(unit); end
			if n ~= nil then
				self.m_Instance.UnitNumPromotions:SetText(tostring(n));
				self.m_Instance.Promotion_Flag:SetToolTipString(aide or "");
				self.m_Instance.Promotion_Flag:SetHide(false);
				return;
			end
			self.m_Instance.Promotion_Flag:SetToolTipString("");
		end
		BGD_UpdatePromotions_jeu(self);
	end

	local function BGD_Rafraichir(playerID, unitID)
		local flag = GetUnitFlag(playerID, unitID);
		if flag ~= nil then flag:UpdatePromotions(); end
	end

	-- Les négociants du joueur local : leurs tours restants changent à chaque tour et quand une
	-- route commence ou finit.
	local function BGD_RafraichirNegociants()
		local ePlayer = Game.GetLocalPlayer();
		local pPlayer = ePlayer ~= nil and ePlayer >= 0 and Players[ePlayer] or nil;
		if pPlayer == nil then return; end
		for _, unit in pPlayer:GetUnits():Members() do
			local info = GameInfo.Units[unit:GetUnitType()];
			if info ~= nil and info.MakeTradeRoute then BGD_Rafraichir(ePlayer, unit:GetID()); end
		end
	end

	local function BGD_OnUnitChargesChanged(playerID, unitID)
		BGD_Rafraichir(playerID, unitID);
	end

	-- Un négociant du joueur local qui bouge : sa route vient de commencer (TradeRouteAddedToMap ne
	-- suffit pas : le drapeau restait sans compteur jusqu'au tour suivant), ou il avance sur son
	-- trajet (le début de tour passe avant le déplacement automatique).
	local function BGD_OnNegociantBouge(playerID, unitID)
		if playerID ~= Game.GetLocalPlayer() then return; end
		local pPlayer = Players[playerID];
		local unit = pPlayer ~= nil and pPlayer:GetUnits():FindID(unitID) or nil;
		if unit == nil then return; end
		local info = GameInfo.Units[unit:GetUnitType()];
		if info ~= nil and info.MakeTradeRoute then BGD_Rafraichir(playerID, unitID); end
	end
	local BGD_EvenementsNegociant = { "UnitMoveComplete", "UnitMovementPointsChanged", "UnitOperationStarted" };

	local BGD_Subscribe_jeu = Subscribe;
	local BGD_Unsubscribe_jeu = Unsubscribe;
	function Subscribe()
		BGD_Subscribe_jeu();
		if BGD_Charges then Events.UnitChargesChanged.Add(BGD_OnUnitChargesChanged); end
		if BGD_Routes then
			Events.LocalPlayerTurnBegin.Add(BGD_RafraichirNegociants);
			Events.TradeRouteAddedToMap.Add(BGD_RafraichirNegociants);
			Events.TradeRouteRemovedFromMap.Add(BGD_RafraichirNegociants);
			Events.LoadGameViewStateDone.Add(BGD_RafraichirNegociants);
			for _, nom in ipairs(BGD_EvenementsNegociant) do
				if Events[nom] ~= nil then Events[nom].Add(BGD_OnNegociantBouge); end
			end
		end
	end
	function Unsubscribe()
		BGD_Unsubscribe_jeu();
		if BGD_Charges then Events.UnitChargesChanged.Remove(BGD_OnUnitChargesChanged); end
		if BGD_Routes then
			Events.LocalPlayerTurnBegin.Remove(BGD_RafraichirNegociants);
			Events.TradeRouteAddedToMap.Remove(BGD_RafraichirNegociants);
			Events.TradeRouteRemovedFromMap.Remove(BGD_RafraichirNegociants);
			Events.LoadGameViewStateDone.Remove(BGD_RafraichirNegociants);
			for _, nom in ipairs(BGD_EvenementsNegociant) do
				if Events[nom] ~= nil then Events[nom].Remove(BGD_OnNegociantBouge); end
			end
		end
	end
end
