-- ===========================================================================
-- Civ6RolloverProxy (src/ui_congres_cibles.lua), ajouté par le proxy au début de WorldCongress.lua.
-- Les résolutions à cible « joueur » (contrôle des frontières, migrations, armement, commerce,
-- victoire diplomatique) proposent tous les majeurs vivants, spectateurs de BSM compris. Leurs
-- validateurs appellent BGD_MajorsCibles() à la place de PlayerManager.GetAliveMajors() : les mêmes
-- majeurs, sans les spectateurs, si les correctifs POL sont actifs (Civ6RolloverProxy_Pol, posée
-- par le proxy). Script de gameplay, exécuté par chaque machine : la liste doit être la même partout.
-- ===========================================================================
local function BGD_MajorsCibles()
	local aPlayers = PlayerManager.GetAliveMajors();
	if Civ6RolloverProxy_Pol == nil or Civ6RolloverProxy_Pol() ~= 1 then
		return aPlayers;
	end
	local aCibles = {};
	for _, pPlayer in ipairs(aPlayers) do
		local pConfig = pPlayer ~= nil and PlayerConfigurations[pPlayer:GetID()];
		if not (pConfig and pConfig:GetLeaderTypeName() == "LEADER_SPECTATOR") then
			table.insert(aCibles, pPlayer);
		end
	end
	return aCibles;
end
