-- ===========================================================================
-- Civ6RolloverProxy (src/ui_accord_luxes.lua), appended by the proxy to the end of
-- DiplomacyDealView.lua with `ui_accord_luxes=on`: the luxuries a player holds but cannot trade,
-- greyed out after the tradeable ones in the "available" column of the deal screen, on both sides.
--
-- The game lists what a player can put in a deal (DealManager.GetPossibleDealItems): the copies
-- of a luxury that come from their own tiles. A copy received from another deal or from a
-- city-state gives its Amenities but is not listed. In a luxury swap between players, neither
-- side sees that the other already has the luxury it is about to receive, nor that the single
-- copy it gives is not its last one.
--
-- PopulateAvailableResources is wrapped (the game's, or the one a "DiplomacyDealView_" file put in
-- its place: the wildcard include runs before this block). After the game filled the luxury list
-- of a player, one icon is added per luxury of which the player holds more copies than they can
-- trade:
--   copies held      : player:GetResources():GetResourceAmount, which counts every source, net of
--                      what the player exports (ReportScreen.lua builds its totals on it) ;
--   tradeable copies : MaxAmount of the possible deal item. When the deal already holds the
--                      luxury (a proposal received, a deal filled by another screen), the
--                      GetMaxAmount of that item, the bound of the game's amount editor, is read
--                      too and the larger of the two is kept: the count does not depend on
--                      whether MaxAmount leaves out what is already in the deal.
-- The icon shows the difference. A luxury can therefore be there twice: one copy from a tile and
-- one from a city-state give a tradeable icon "1" and a greyed icon "1". A column then shows all
-- the luxuries of its player: a luxury absent from the other side's column is one they lack, and
-- a tradeable "1" without a greyed twin is a last copy.
--
-- The tooltip names where the copies come from, found the way ReportScreen.lua finds them for
-- the local player, in the game's own words:
--   city-states : the ones whose suzerain is the player, and the copies each has: its
--                 GetResourceAmount plus its GetExportedResourceAmount, the sum the game's
--                 city-state screen shows (CityStates_Expansion2.lua). ReportScreen.lua reads
--                 the exported amount alone, which is usually 0: the copies of a city-state
--                 end up under its "gameplay bonuses". One the local player has not met is
--                 not named ;
--   deals       : DealManager.GetPlayerDeals with each other major player, the resource items
--                 that player gives, and the turns left (duration, turn enacted) ; a partner the
--                 local player has not met is not named. The game only ever asks for the deals
--                 of the local player: for a deal between two other players the call may return
--                 nothing, and the copy then counts as "other source" ;
--   other source: what is left of the icon's count (a great person, a building, a deal that
--                 could not be read).
-- Each lookup runs under pcall: a failure is written to Lua.log and its copies go to "other
-- source".
--
-- The icon is one of the game's (g_IconOnlyIM), in the state the game gives to a resource that
-- cannot be added (grey, button disabled): every consumer of that pool sets the state of the
-- button again, the available gold through ui_accord_or.lua. The game fills the "available"
-- column once, when the screen opens: so does the list.
--
-- Display only, local: nothing is added to the deal, and the count of available items the game
-- sums (what it returns) is unchanged.
-- ===========================================================================
do
	local AccordLuxes_Ok, AccordLuxes_Err = pcall(function()
		if BGD_AccordLuxes ~= nil or type(PopulateAvailableResources) ~= "function"
			or type(GetOtherPlayer) ~= "function" or type(SetIconToSize) ~= "function"
			or g_IconOnlyIM == nil then
			return;
		end
		BGD_AccordLuxes = true;

		local LUXE = "RESOURCECLASS_LUXURY";
		local PopulateAvailableResources_jeu = PopulateAvailableResources;

		-- Copies of each resource the player can put in the deal: { [resource index] = copies }.
		local function Tradeable(player)
			local copies = {};
			local function Keep(resourceType, amount)
				if type(resourceType) == "number" and type(amount) == "number" and amount > (copies[resourceType] or 0) then
					copies[resourceType] = amount;
				end
			end
			local pDeal = DealManager.GetWorkingDeal(DealDirection.OUTGOING, g_LocalPlayer:GetID(), g_OtherPlayer:GetID());
			local possible = DealManager.GetPossibleDealItems(player:GetID(), GetOtherPlayer(player):GetID(), DealItemTypes.RESOURCES, pDeal);
			if possible ~= nil then
				for _, entry in ipairs(possible) do
					Keep(entry.ForType, entry.MaxAmount);
				end
			end
			if pDeal ~= nil then
				local dealItems = pDeal:FindItemsByType(DealItemTypes.RESOURCES, DealItemSubTypes.NONE, player:GetID());
				if dealItems ~= nil then
					for _, pDealItem in ipairs(dealItems) do
						Keep(pDealItem:GetValueType(), pDealItem:GetMaxAmount());
					end
				end
			end
			return copies;
		end

		local function Try(what, f)
			local ok, err = pcall(f);
			if not ok then
				print("BGE accord luxes : " .. what .. " : echec " .. tostring(err));
			end
		end

		-- Where the player's copies of the wanted resources come from, other than their tiles:
		-- { [resource index] = { { text = , amount = }, ... } }.
		local function Sources(player, wanted)
			local sources = {};
			local function Add(resourceType, text, amount)
				if type(amount) == "number" and amount > 0 then
					sources[resourceType] = sources[resourceType] or {};
					table.insert(sources[resourceType], { text = text, amount = amount });
				end
			end
			local playerID = player:GetID();
			local localID = g_LocalPlayer:GetID();
			local pDiplomacy = g_LocalPlayer:GetDiplomacy();
			local function Met(id)
				return id == localID or (pDiplomacy ~= nil and pDiplomacy:HasMet(id) == true);
			end
			local function Name(id)
				return Locale.Lookup(PlayerConfigurations[id]:GetPlayerName());
			end

			Try("cites-etats", function()
				for _, pMinor in ipairs(PlayerManager.GetAliveMinors()) do
					local pInfluence = pMinor:GetInfluence();
					if pInfluence ~= nil and pInfluence:GetSuzerain() == playerID then
						local minorID = pMinor:GetID();
						local text = Locale.Lookup("LOC_HUD_REPORTS_CITY_STATE");
						if Met(minorID) then
							text = text .. " (" .. Name(minorID) .. ")";
						end
						local pMinorResources = pMinor:GetResources();
						for resourceType in pairs(wanted) do
							local own = pMinorResources:GetResourceAmount(resourceType);
							local exported = pMinorResources:GetExportedResourceAmount(resourceType);
							Add(resourceType, text, (type(own) == "number" and own or 0) + (type(exported) == "number" and exported or 0));
						end
					end
				end
			end);

			local turn = Game.GetCurrentGameTurn();
			for _, pOther in ipairs(PlayerManager.GetAliveMajors()) do
				local otherID = pOther:GetID();
				if otherID ~= playerID then
					Try("accords", function()
						-- The local player first, as the game asks for its own deals.
						local pDeals;
						if otherID == localID then
							pDeals = DealManager.GetPlayerDeals(otherID, playerID);
						else
							pDeals = DealManager.GetPlayerDeals(playerID, otherID);
						end
						if pDeals == nil then
							return;
						end
						for _, pDeal in ipairs(pDeals) do
							local dealItems = pDeal:FindItemsByType(DealItemTypes.RESOURCES, DealItemSubTypes.NONE, otherID);
							if dealItems ~= nil then
								for _, pDealItem in ipairs(dealItems) do
									local duration = pDealItem:GetDuration();
									local resourceType = pDealItem:GetValueType();
									if duration ~= 0 and wanted[resourceType] then
										local turns = Locale.Lookup("LOC_REPORTS_NUMBER_OF_TURNS", duration - (turn - pDealItem:GetEnactedTurn()));
										local text = Locale.Lookup("LOC_HUD_REPORTS_ROW_DIPLOMATIC_DEALS") .. " ("
											.. (Met(otherID) and (Name(otherID) .. " ") or "") .. turns .. ")";
										Add(resourceType, text, pDealItem:GetAmount());
									end
								end
							end
						end
					end);
				end
			end
			return sources;
		end

		-- "Wine / Cannot be traded. Source: / - 1 City-State (Zanzibar) / - 1 Other source".
		local function ToolTip(row, untradeable, sources)
			local icon = " [ICON_" .. row.ResourceType .. "] ";
			local text = Locale.Lookup(row.Name) .. "[NEWLINE]" .. BGD_Txt("LOC_BGD_ACCORD_LUXE_NON_CESSIBLE");
			local found = 0;
			for _, source in ipairs(sources or {}) do
				text = text .. "[NEWLINE][ICON_Bullet]" .. tostring(source.amount) .. icon .. source.text;
				found = found + source.amount;
			end
			if untradeable > found then
				text = text .. "[NEWLINE][ICON_Bullet]" .. tostring(untradeable - found) .. icon .. BGD_Txt("LOC_BGD_ACCORD_LUXE_AUTRE_SOURCE");
			end
			return text;
		end

		local function AddUntradeable(player, iconList)
			local tradeable = Tradeable(player);
			local pResources = player:GetResources();
			local rows = {};
			local wanted = {};
			for row in GameInfo.Resources() do
				if row.ResourceClassType == LUXE then
					local held = pResources:GetResourceAmount(row.Index);
					local untradeable = (type(held) == "number" and held or 0) - (tradeable[row.Index] or 0);
					if untradeable > 0 then
						rows[#rows + 1] = { row = row, untradeable = untradeable };
						wanted[row.Index] = true;
					end
				end
			end
			if #rows == 0 then
				return;
			end
			local sources = {};
			Try("sources", function() sources = Sources(player, wanted); end);
			for _, entry in ipairs(rows) do
				local row = entry.row;
				local uiIcon = g_IconOnlyIM:GetInstance(iconList.ListStack);
				SetIconToSize(uiIcon.Icon, "ICON_" .. row.ResourceType);
				uiIcon.AmountText:SetText(tostring(entry.untradeable));
				uiIcon.AmountText:SetHide(false);
				uiIcon.RemoveButton:SetHide(true);
				uiIcon.StopAskingButton:SetHide(true);
				if uiIcon.UnacceptableIcon ~= nil then
					uiIcon.UnacceptableIcon:SetHide(true);
				end
				uiIcon.Icon:SetColor(0.5, 0.5, 0.5);
				-- The instance is recycled: no click left from its previous use.
				uiIcon.SelectButton:ClearCallback(Mouse.eLClick);
				uiIcon.SelectButton:ClearCallback(Mouse.eRClick);
				uiIcon.SelectButton:SetToolTipString(ToolTip(row, entry.untradeable, sources[row.Index]));
				uiIcon.SelectButton:SetDisabled(true);
			end
			-- The game hid the list if the player has no tradeable luxury.
			iconList.ListStack:CalculateSize();
			iconList.GetTopControl():SetHide(false);
		end

		function PopulateAvailableResources(player, iconList, className, ...)
			local count = PopulateAvailableResources_jeu(player, iconList, className, ...);
			if className == LUXE then
				local ok, err = pcall(AddUntradeable, player, iconList);
				if not ok then
					print("BGE accord luxes : echec " .. tostring(err));
				end
			end
			return count;
		end
	end);
	if not AccordLuxes_Ok then
		print("BGE accord luxes : echec " .. tostring(AccordLuxes_Err));
	end
end
