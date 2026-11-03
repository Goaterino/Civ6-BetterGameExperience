	-- BetterGameDLLs (src/mods/politiques_carte.lua) : effet de la carte, calculé par RMA, dans
	-- l'infobulle et dans un cadre en bas de la carte (EffectContainer, ajouté au XML par le proxy).
	local BGD_effet = "";
	if BGD_ChargerRMA ~= nil and cardInstance.EffectContainer ~= nil then
		local BGD_rma = BGD_ChargerRMA();
		if BGD_rma.CalculateModifierEffect ~= nil then
			local ok, s = pcall(BGD_rma.CalculateModifierEffect, "Policy", policyType, 0, nil, nil);
			if ok and type(s) == "string" then BGD_effet = s; end
		end
	end
	cardInstance.Draggable:SetToolTipString(cardName .. "[NEWLINE][NEWLINE]" .. policy.Description .. (BGD_effet == "" and "" or "[NEWLINE][NEWLINE]" .. BGD_effet));
	if cardInstance.EffectContainer ~= nil then
		cardInstance.EffectContainer:SetHide(BGD_effet == "");
		cardInstance.Effect:SetText(BGD_effet);
		cardInstance.Effect:SetToolTipString(BGD_effet);
	end
