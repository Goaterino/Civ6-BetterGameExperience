-- BetterGameDLLs (src/mods/politiques_entete.lua), ajouté par le proxy en tête de
-- GovernmentScreen.lua : Extended Policy Cards (Aristos), repris dans le proxy. Real Modifier
-- Analysis (de Better Report Screen, corrigé) est chargé à la première carte ; le mod en avait besoin
-- dans un autre contexte, le proxy l'apporte ici.
-- BGD_PolTrace(cle, texte) : écrit texte dans Lua.log une seule fois par clé et par contexte. Les
-- chemins où l'effet des cartes manque s'y signalent (rapport BGE-BFBBC3 : effets perdus après le
-- rechargement d'une partie, sans aucune trace). Globale, pour n'ajouter aucune locale au chunk.
BGD_POL_TRACES = {};
function BGD_PolTrace(cle, texte)
	if BGD_POL_TRACES[cle] then return; end
	BGD_POL_TRACES[cle] = true;
	print("BetterGameExperience : cartes de politique : " .. tostring(texte));
end
do
	local f = Civ6Proxy_Charger and Civ6Proxy_Charger("BGD_Commun");
	if type(f) == "function" then
		f();
	elseif Civ6Proxy_Charger == nil then
		BGD_PolTrace("commun", "Civ6Proxy_Charger absent, BGD_Commun non chargé");
	else
		BGD_PolTrace("commun", "BGD_Commun non chargé : " .. tostring(f));
	end
end
