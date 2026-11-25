	-- BetterGameDLLs (src/mods/politiques_carte.lua) : effet de la carte, calculé par RMA, dans
	-- l'infobulle et dans un cadre en bas de la carte (EffectContainer, ajouté au XML par le proxy).
	-- Chaque chemin sans effet est signalé une fois dans Lua.log (BGD_PolTrace, en tête du fichier).
	local BGD_effet = "";
	if BGD_ChargerRMA == nil then
		if BGD_PolTrace ~= nil then BGD_PolTrace("rma", "BGD_ChargerRMA absent (BGD_Commun non exécuté)"); end
	elseif cardInstance.EffectContainer == nil then
		if BGD_PolTrace ~= nil then BGD_PolTrace("cadre", "EffectContainer absent de la carte (XML non modifié ?)"); end
	else
		local BGD_rma = BGD_ChargerRMA();
		if BGD_rma.CalculateModifierEffect == nil then
			if BGD_PolTrace ~= nil then BGD_PolTrace("calcul", "RMA chargé sans CalculateModifierEffect"); end
		else
			local ok, s = pcall(BGD_rma.CalculateModifierEffect, "Policy", policyType, 0, nil, nil);
			if ok and type(s) == "string" then
				BGD_effet = s;
			elseif BGD_PolTrace ~= nil then
				BGD_PolTrace(ok and "type" or "erreur", tostring(policyType) .. " : " .. (ok and ("résultat " .. type(s)) or tostring(s)));
			end
		end
	end
	cardInstance.Draggable:SetToolTipString(cardName .. "[NEWLINE][NEWLINE]" .. policy.Description .. (BGD_effet == "" and "" or "[NEWLINE][NEWLINE]" .. BGD_effet));
	if cardInstance.EffectContainer ~= nil then
		cardInstance.EffectContainer:SetHide(BGD_effet == "");
		cardInstance.Effect:SetText(BGD_effet);
		cardInstance.Effect:SetToolTipString(BGD_effet);
	end
