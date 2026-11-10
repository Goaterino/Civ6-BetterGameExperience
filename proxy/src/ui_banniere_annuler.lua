-- ===========================================================================
-- Civ6RolloverProxy (src/ui_banniere_annuler.lua), ajouté par le proxy à la fin de
-- CityBannerManager.lua (XP2 ou BSM) avec `ui_banniere_annuler=on` (BET-12) : un clic droit sur
-- l'icône de production du bandeau d'une de ses villes retire l'élément en tête de file, comme le
-- clic droit du panneau de production. Une commande ordinaire du jeu (BUILD, VALUE_REMOVE_AT), sur
-- la ville du bandeau et non sur la ville sélectionnée : rien d'autre ne change.
-- CityBanner:UpdateProduction est prévue pour être remplacée par un mod ; on l'enveloppe, et on
-- branche le clic droit sur le bouton que l'original vient de préparer.
-- ===========================================================================
do
local Annuler_Ok, Annuler_Err = pcall(function()
	if BGD_Annuler_Etat ~= nil or CityBanner == nil or type(CityBanner.UpdateProduction) ~= "function" then
		return;
	end
	BGD_Annuler_Etat = true;

	local function Retirer(playerID, cityID)
		if playerID ~= Game.GetLocalPlayer() then return; end
		local pPlayer = Players[playerID];
		local pCity = pPlayer ~= nil and pPlayer:GetCities():FindID(cityID) or nil;
		if pCity == nil or pCity:GetBuildQueue():GetSize() == 0 then return; end
		local p = {};
		p[CityOperationTypes.PARAM_INSERT_MODE] = CityOperationTypes.VALUE_REMOVE_AT;
		p[CityOperationTypes.PARAM_QUEUE_LOCATION] = 0;
		CityManager.RequestOperation(pCity, CityOperationTypes.BUILD, p);
		UI.PlaySound("Play_UI_Click");
	end

	local Update_jeu = CityBanner.UpdateProduction;
	CityBanner.UpdateProduction = function(self, pCity, ...)
		Update_jeu(self, pCity, ...);
		if self.m_StatProductionIM == nil or pCity == nil then return; end
		local inst = self.m_StatProductionIM:GetAllocatedInstance(1);
		if inst == nil or inst.Button == nil then return; end
		if pCity:GetOwner() == Game.GetLocalPlayer() then
			local owner, id = pCity:GetOwner(), pCity:GetID();
			inst.Button:RegisterCallback(Mouse.eRClick, function() Retirer(owner, id); end);
		else
			inst.Button:ClearCallback(Mouse.eRClick);
		end
	end
end);
if not Annuler_Ok then
	print("Annuler echec " .. tostring(Annuler_Err));
end
end
