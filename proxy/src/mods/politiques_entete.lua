-- BetterGameDLLs (src/mods/politiques_entete.lua), ajouté par le proxy en tête de
-- GovernmentScreen.lua : Extended Policy Cards (Aristos), repris dans le proxy. Real Modifier
-- Analysis (de Better Report Screen, corrigé) est chargé à la première carte ; le mod en avait besoin
-- dans un autre contexte, le proxy l'apporte ici.
do
	local f = Civ6Proxy_Charger and Civ6Proxy_Charger("BGD_Commun");
	if type(f) == "function" then f(); end
end
