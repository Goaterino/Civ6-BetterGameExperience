-- ===========================================================================
-- Civ6RolloverProxy (src/ui_accord_or.lua), ajouté par le proxy à la fin de DiplomacyDealView.lua :
-- bouton d'or de la colonne « disponible » parfois désactivé sans raison.
-- Les icônes de l'écran d'accord viennent de deux réserves partagées (g_IconOnlyIM,
-- g_IconAndTextIM). ResetInstances les rend à chaque ouverture sans remettre leur état à zéro.
-- Une ressource stratégique sans place sous le plafond, ou l'icône réduite d'un accord, d'une cité
-- ou d'une œuvre non valide, désactive son bouton (SetDisabled(true)). PopulateAvailableGold
-- reprend ensuite une de ces instances pour l'or ponctuel ou par tour : elle remet l'infobulle et
-- la couleur, mais pas SetDisabled(false) (PopulateAvailableFavor, elle, le fait). Quand la
-- disposition change d'une ouverture à l'autre, l'or reçoit un bouton désactivé qui paraît normal.
-- On enveloppe PopulateAvailableGold (celle de MPH aussi, incluse par le joker
-- include("DiplomacyDealView_") avant ce bloc) et on réactive les boutons qu'elle vient de prendre :
-- GetInstance les ajoute à la fin de m_AllocatedInstances.
-- ===========================================================================
do
	local AccordOr_Ok, AccordOr_Err = pcall(function()
		if BGD_AccordOr ~= nil or type(PopulateAvailableGold) ~= "function" then
			return;
		end
		BGD_AccordOr = true;

		local PopulateAvailableGold_jeu = PopulateAvailableGold;

		local function Count(im)
			local list = im and im.m_AllocatedInstances;
			if type(list) ~= "table" then
				return nil;
			end
			return #list;
		end

		local function Enable(im, from)
			if from == nil then
				return;
			end
			local list = im.m_AllocatedInstances;
			for i = from + 1, #list do
				local button = list[i] and list[i].SelectButton;
				if button ~= nil then
					button:SetDisabled(false);
				end
			end
		end

		function PopulateAvailableGold(...)
			local iconOnly = g_IconOnlyIM;
			local iconAndText = g_IconAndTextIM;
			local n1 = Count(iconOnly);
			local n2 = Count(iconAndText);
			local count = PopulateAvailableGold_jeu(...);
			local ok, err = pcall(function()
				Enable(iconOnly, n1);
				Enable(iconAndText, n2);
			end);
			if not ok then
				print("BGE accord or : echec " .. tostring(err));
			end
			return count;
		end
	end);
	if not AccordOr_Ok then
		print("BGE accord or : echec " .. tostring(AccordOr_Err));
	end
end
