-- ===========================================================================
-- Civ6RolloverProxy (src/ui_debordement.lua), ajouté par le proxy à la fin de CityBannerManager.lua,
-- de CityPanel.lua et de ProductionPanel.lua avec `ui_debordement=on` (BET-21) : la production
-- qu'une ville garde en réserve (débordement d'un élément terminé, ou production d'un tour passé
-- sans rien construire). GameCore l'ajoute à la production du tour suivant, quel que soit
-- l'élément en tête, et la plafonne à un tour.
-- La valeur n'est pas dans le cache de l'interface : le proxy la relève dans GameCore à chaque
-- mise à jour du cache de la file (debordement.rs) et la rend par BGD_Debordement(joueur, ville),
-- en points de production, ou nil s'il ne l'a pas encore vue. Villes du joueur local seulement,
-- à partir d'un point :
-- - bandeau de ville : une ligne d'infobulle ; file vide, « +N » à la place des tours restants ;
-- - panneau de la ville (en bas à droite) : une ligne sous la production en cours ;
-- - panneau de production : « +N » après la progression de la production en cours, et
--   l'infobulle du cadre « aucune production ».
-- Rien n'est montré quand l'hôte a coché l'option de partie « Disable Advanced UI indicators (BGE) ».
-- Tout s'exécute sous pcall ; un échec écrit `Debordement echec <erreur>`.
-- ===========================================================================
do
local Debordement_Ok, Debordement_Err = pcall(function()
	if BGD_Debordement_Etat ~= nil or type(BGD_Debordement) ~= "function" then
		return;
	end
	BGD_Debordement_Etat = true;

	-- Option de partie « Disable Advanced UI indicators (BGE) » (BGD_INDICATEURS_OFF, cochée par
	-- l'hôte) : rien n'est montré. Lue à chaque redessin, jamais retenue : une partie suivante sans
	-- l'option, dans la même session du jeu, retrouve l'affichage, et inversement.
	local function IndicateursCoupes()
		if GameConfiguration == nil or GameConfiguration.GetValue == nil then return false; end
		local ok, v = pcall(GameConfiguration.GetValue, "BGD_INDICATEURS_OFF");
		return ok and (v == true or v == 1);
	end

	-- Production en réserve de la ville, arrondie à l'unité inférieure ; 0 si inconnue ou masquée
	-- (les affichages effacent alors ce qu'un passage précédent aurait laissé).
	local function Reserve(pCity)
		if pCity == nil or pCity:GetOwner() ~= Game.GetLocalPlayer() or IndicateursCoupes() then return 0; end
		local v = BGD_Debordement(pCity:GetOwner(), pCity:GetID());
		if type(v) ~= "number" or v < 1 then return 0; end
		return math.floor(v);
	end

	-- Bandeau de ville.
	if CityBanner ~= nil and type(CityBanner.UpdateProduction) == "function" then
		local Update_jeu = CityBanner.UpdateProduction;
		CityBanner.UpdateProduction = function(self, pCity, ...)
			Update_jeu(self, pCity, ...);
			if self.m_StatProductionIM == nil or pCity == nil or pCity:GetOwner() ~= Game.GetLocalPlayer() then return; end
			local inst = self.m_StatProductionIM:GetAllocatedInstance(1);
			if inst == nil or inst.TurnsLeft == nil then return; end
			local q = pCity:GetBuildQueue();
			if q == nil then return; end
			local vide = q:GetCurrentProductionTypeHash() == 0;
			local n = Reserve(pCity);
			if n > 0 then
				local tip = vide and Locale.Lookup("LOC_CITY_BANNER_NO_PRODUCTION") or inst.Button:GetToolTipString();
				inst.Button:SetToolTipString(tip .. "[NEWLINE]" .. BGD_Txt("LOC_BGD_DEBORDEMENT", n));
				if vide then
					inst.TurnsLeft:SetText("+" .. n);
				end
			elseif vide then
				-- L'instance est réutilisée : sans production, rien ne doit rester du tour précédent.
				inst.TurnsLeft:SetText("");
			end
		end
	end

	-- Panneau de la ville sélectionnée : ligne ajoutée aux statistiques de la production.
	if type(ViewMain) == "function" and Controls ~= nil and Controls.ProductionStatString ~= nil then
		local ViewMain_jeu = ViewMain;
		function ViewMain(data, ...)
			ViewMain_jeu(data, ...);
			local n = Reserve(UI.GetHeadSelectedCity());
			local stats = data ~= nil and data.CurrentProductionStats or "";
			if n > 0 then
				local ligne = BGD_Txt("LOC_BGD_DEBORDEMENT", n);
				Controls.ProductionStatString:SetText(stats ~= "" and (stats .. "[NEWLINE]" .. ligne) or ligne);
			else
				Controls.ProductionStatString:SetText(stats);
			end
			if Controls.ProductionDataStack ~= nil then Controls.ProductionDataStack:CalculateSize(); end
			if Controls.ProductionDataScroll ~= nil then Controls.ProductionDataScroll:CalculateSize(); end
		end
	end

	-- Panneau de production.
	if type(View) == "function" and Controls ~= nil and Controls.NoProductionContainer ~= nil then
		local View_jeu = View;
		function View(data)
			View_jeu(data);
			local pCity = UI.GetHeadSelectedCity();
			local n = Reserve(pCity);
			local txt = n > 0 and BGD_Txt("LOC_BGD_DEBORDEMENT", n) or "";
			Controls.NoProductionContainer:SetToolTipString(txt);
			local s = Controls.CurrentProductionProgressString;
			-- Hors des onglets Production et File, View ne réécrit pas la progression : on n'ajoute
			-- la réserve qu'une fois.
			local marque = "  [COLOR:ResProductionLabelCS]+";
			if n > 0 and s ~= nil and pCity:GetBuildQueue():GetCurrentProductionTypeHash() ~= 0 then
				local t = s:GetText() or "";
				local avant = t:find(marque, 1, true);
				if avant ~= nil then t = t:sub(1, avant - 1); end
				s:SetText(t .. marque .. n .. "[ENDCOLOR]");
				s:SetToolTipString(txt);
			elseif s ~= nil then
				local t = s:GetText() or "";
				local avant = t:find(marque, 1, true);
				if avant ~= nil then s:SetText(t:sub(1, avant - 1)); end
				s:SetToolTipString("");
			end
		end
	end
	-- Pas d'événement à ajouter : bandeau et panneaux se redessinent déjà sur les événements de
	-- production, que le jeu envoie après la mise à jour du cache où le proxy relève la valeur.
end);
if not Debordement_Ok then
	print("Debordement echec " .. tostring(Debordement_Err));
end
end
