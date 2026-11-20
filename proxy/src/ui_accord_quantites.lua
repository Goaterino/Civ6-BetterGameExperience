-- ===========================================================================
-- Civ6RolloverProxy (src/ui_accord_quantites.lua), appended by the proxy to the end of
-- DiplomacyDealView.lua with `ui_accord_quantites=on`: quantity shortcuts on the resources of the
-- "available" column of the deal screen.
--   Shift+click : as many of the resource as the deal accepts ;
--   Ctrl+click  : 10 more (or what is left).
--
-- The game adds one unit per click. Asking for a whole stock of a strategic resource takes a
-- click on the available icon, a click on the icon in the deal, a number typed in the amount
-- editor ("1111") and a confirmation.
--
-- OnClickAvailableResource is wrapped (the game's, or the one a "DiplomacyDealView_" file put in
-- its place: XP2, a mod; the wildcard include runs before this block). A plain click goes to the
-- game untouched. With Shift or Ctrl held:
--   1. the game's own click runs first. It decides everything a click decides: whose demand or
--      gift cannot be changed, which item of the deal receives the unit (the 30-turn one, or the
--      one-time one of a stockpiled resource), a new item and its duration, the sound ;
--   2. the item it created or raised is found by comparing the amounts before and after, and its
--      amount is raised to the target, capped by GetMaxAmount, the bound of the game's amount
--      editor (OnValueEditButton). If the item is not valid at that amount (the game tests
--      IsValid after each unit: no room left under the other player's stockpile cap), the
--      largest valid amount is kept, found by bisection.
-- UpdateDealPanel and UpdateProposedWorkingDeal, which the game's click calls, are held back
-- during step 1 and run once after step 2. Facing an AI, UpdateProposedWorkingDeal sends the deal
-- for inspection, and no second one leaves while that one is pending: the AI would answer on one
-- unit, not on the amount shown.
--
-- A mouse callback receives no modifier state, and the UI has no global query for it. The input
-- handler of the context is wrapped: each input, the click's own button events included, carries
-- IsShiftDown and IsControlDown. The state used is therefore the one of the click itself, not
-- one left over from before an Alt+Tab.
--
-- Local only: the deal is built and sent by the game's own calls, as with the amount editor.
-- ===========================================================================
do
	local AccordQuantites_Ok, AccordQuantites_Err = pcall(function()
		if BGD_AccordQuantites ~= nil or type(OnClickAvailableResource) ~= "function"
			or type(InputHandler) ~= "function" or type(UpdateDealPanel) ~= "function"
			or type(UpdateProposedWorkingDeal) ~= "function" then
			return;
		end
		BGD_AccordQuantites = true;

		local PAS_CTRL = 10;

		local OnClickAvailableResource_jeu = OnClickAvailableResource;
		local InputHandler_jeu = InputHandler;
		local m_Maj = false;
		local m_Ctrl = false;

		local function ReadModifiers(pInputStruct)
			local msg = pInputStruct:GetMessageType();
			if msg == KeyEvents.KeyDown or msg == KeyEvents.KeyUp then
				-- The event of the modifier key itself: its own state is the event.
				local key = pInputStruct:GetKey();
				if key == Keys.VK_SHIFT then
					m_Maj = (msg == KeyEvents.KeyDown);
					return;
				elseif key == Keys.VK_CONTROL then
					m_Ctrl = (msg == KeyEvents.KeyDown);
					return;
				end
			end
			m_Maj = pInputStruct:IsShiftDown() == true;
			m_Ctrl = pInputStruct:IsControlDown() == true;
		end

		ContextPtr:SetInputHandler(function(pInputStruct)
			pcall(ReadModifiers, pInputStruct);
			return InputHandler_jeu(pInputStruct);
		end, true);

		-- Amounts of the deal items of this resource given by this player: { [item id] = amount }.
		local function Amounts(player, resourceType)
			local pDeal = DealManager.GetWorkingDeal(DealDirection.OUTGOING, g_LocalPlayer:GetID(), g_OtherPlayer:GetID());
			if pDeal == nil then
				return nil;
			end
			local amounts = {};
			local items = {};
			local dealItems = pDeal:FindItemsByType(DealItemTypes.RESOURCES, DealItemSubTypes.NONE, player:GetID());
			if dealItems ~= nil then
				for _, pDealItem in ipairs(dealItems) do
					if pDealItem:GetValueType() == resourceType then
						local id = pDealItem:GetID();
						amounts[id] = pDealItem:GetAmount();
						items[id] = pDealItem;
					end
				end
			end
			return amounts, items;
		end

		-- The game's click, with its two refreshes recorded instead of run.
		local function ClickHeld(player, resourceType)
			local UpdateDealPanel_jeu = UpdateDealPanel;
			local UpdateProposedWorkingDeal_jeu = UpdateProposedWorkingDeal;
			local held = {};
			UpdateDealPanel = function(a) held[#held + 1] = { UpdateDealPanel_jeu, a }; end
			UpdateProposedWorkingDeal = function(a) held[#held + 1] = { UpdateProposedWorkingDeal_jeu, a }; end
			local ok, err = pcall(OnClickAvailableResource_jeu, player, resourceType);
			UpdateDealPanel = UpdateDealPanel_jeu;
			UpdateProposedWorkingDeal = UpdateProposedWorkingDeal_jeu;
			if not ok then
				print("BGE accord quantites : echec du clic " .. tostring(err));
			end
			return held;
		end

		-- Raises the item the click touched: `before` is its amount before the click (0 for a new
		-- item), the click left it valid at before + 1.
		local function Raise(pDealItem, before, wanted)
			local after = pDealItem:GetAmount();
			local max = pDealItem:GetMaxAmount();
			if type(max) ~= "number" then
				return;
			end
			local target = math.min(before + wanted, max);
			if target <= after then
				return;
			end
			pDealItem:SetAmount(target);
			if pDealItem:IsValid() then
				return;
			end
			local low, high = after, target;
			while high - low > 1 do
				local mid = math.floor((low + high) / 2);
				pDealItem:SetAmount(mid);
				if pDealItem:IsValid() then
					low = mid;
				else
					high = mid;
				end
			end
			pDealItem:SetAmount(low);
		end

		function OnClickAvailableResource(player, resourceType)
			local wanted = (m_Maj and math.huge) or (m_Ctrl and PAS_CTRL) or nil;
			local okBefore, before = false, nil;
			if wanted ~= nil then
				okBefore, before = pcall(Amounts, player, resourceType);
			end
			if wanted == nil or not okBefore or before == nil then
				return OnClickAvailableResource_jeu(player, resourceType);
			end

			local held = ClickHeld(player, resourceType);
			local ok, err = pcall(function()
				local after, items = Amounts(player, resourceType);
				if after == nil then
					return;
				end
				for id, amount in pairs(after) do
					if amount > (before[id] or 0) then
						Raise(items[id], before[id] or 0, wanted);
						return;
					end
				end
			end);
			if not ok then
				print("BGE accord quantites : echec " .. tostring(err));
			end
			for _, call in ipairs(held) do
				call[1](call[2]);
			end
		end
	end);
	if not AccordQuantites_Ok then
		print("BGE accord quantites : echec " .. tostring(AccordQuantites_Err));
	end
end
