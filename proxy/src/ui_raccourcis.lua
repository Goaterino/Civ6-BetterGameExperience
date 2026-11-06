
-- ===========================================================================
-- Civ6RolloverProxy (src/ui_raccourcis.lua), ajouté par le proxy à la fin de InGame.lua : la locale
-- DefaultMessageHandler du fichier reste visible.
-- Raccourcis clavier coupés : les actions de la partie ne partent que si le contexte d'entrée World
-- est en haut de la pile. Un contexte resté sur la pile après la fermeture de son écran les bloque
-- toutes : menu Échap fermé par une diplomatie (CloseImmediately ne dépile pas GameOptions), ou
-- congrès empilé deux fois pour un seul dépilement. On retire GameOptions ou Popup du haut de la pile
-- quand l'écran qui l'a empilé est masqué et hors de la file des fenêtres, dès que l'interface est au
-- repos, qu'une touche est enfoncée ou qu'une diplomatie se ferme. Voir analyses/raccourcis.
-- ===========================================================================
do
	local function BGD_Ferme(pContext)
		return pContext == nil or (pContext:IsHidden() and not UIManager:IsInPopupQueue(pContext));
	end

	local function BGD_Orphelin(eActif)
		if eActif == InputContext.GameOptions then
			return BGD_Ferme(ContextPtr:LookUpControl("/InGame/TopOptionsMenu"));
		elseif eActif == InputContext.Popup then
			return BGD_Ferme(ContextPtr:LookUpControl("/InGame/WorldCongressPopup"));
		end
		return false;
	end

	local function BGD_RetablirContexte(qui)
		for _ = 1, 8 do
			local eActif = Input.GetActiveContext();
			if not BGD_Orphelin(eActif) then
				return;
			end
			print("BGD raccourcis : contexte d'entrée " .. tostring(eActif) .. " retiré (" .. qui .. ")");
			Input.PopContext();
		end
	end

	local BGD_KeyDown = DefaultMessageHandler[KeyEvents.KeyDown];
	DefaultMessageHandler[KeyEvents.KeyDown] = function(pInputStruct)
		BGD_RetablirContexte("touche");
		if BGD_KeyDown ~= nil then
			return BGD_KeyDown(pInputStruct);
		end
		return false;
	end

	Events.UIIdle.Add(function() BGD_RetablirContexte("repos"); end);
	-- Après InGame, qui dépile Diplomacy sur le même événement (inscrit avant).
	LuaEvents.DiplomacyActionView_ShowIngameUI.Add(function() BGD_RetablirContexte("diplomatie"); end);
end
