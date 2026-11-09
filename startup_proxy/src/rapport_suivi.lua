-- ===========================================================================
-- BetterGameExperience : suivi de l'envoi d'un rapport de bug (voir startup_proxy/src/rapport.rs).
-- Ajouté à la fin d'InGame.lua (ou de son remplaçant du même nom, celui de MPH), contexte toujours
-- affiché en partie : le menu Échap, lui, est caché dès que le joueur le ferme, et ses images
-- s'arrêtent. Le proxy a ajouté au XML un minuteur, BGD_RapportMinuteur (SlideAnim d'environ
-- 0,5 s, comme le minuteur de la minimap du jeu), qui tourne même partie en pause.
-- Natives : Civ6Proxy_RapportEtat() (voir rapport.lua) et Civ6Proxy_Journal(texte).
--   LuaEvents.BGD_RapportSuivre()                     lancé par le menu au début de l'envoi ;
--   LuaEvents.BGD_RapportFini(statut, texte, archive) rendu au menu à la fin, qui prévient le chat,
--                                                     affiche le résultat et se rouvre s'il le faut.
-- ===========================================================================
do
local BGD_RapportSuiviOk, BGD_RapportSuiviErr = pcall(function()
	if BGD_RapportSuivi ~= nil or Civ6Proxy_RapportEtat == nil then return; end

	local function Journal(texte)
		if Civ6Proxy_Journal ~= nil then pcall(Civ6Proxy_Journal, "rapport : " .. texte); end
	end

	local minuteur = Controls.BGD_RapportMinuteur;
	if minuteur == nil then
		Journal("minuteur absent du XML d'InGame, suivi par le menu seul");
		return;
	end

	local S = { actif = false };
	BGD_RapportSuivi = S;

	local function Relancer()
		minuteur:SetToBeginning();
		minuteur:Play();
	end

	minuteur:RegisterEndCallback(function()
		if not S.actif then return; end
		local ok, statut, texte, archive = pcall(Civ6Proxy_RapportEtat);
		if not ok then
			Journal("erreur du suivi : " .. tostring(statut));
		elseif statut ~= nil and statut ~= "" then
			S.actif = false;
			LuaEvents.BGD_RapportFini(statut, texte, archive);
			return;
		end
		Relancer();
	end);

	LuaEvents.BGD_RapportSuivre.Add(function()
		if S.actif then return; end
		S.actif = true;
		Relancer();
	end);
	-- Le menu a reçu le résultat lui-même (il était affiché) : plus rien à suivre.
	LuaEvents.BGD_RapportFini.Add(function() S.actif = false; end);
end);
if not BGD_RapportSuiviOk and Civ6Proxy_Journal ~= nil then
	pcall(Civ6Proxy_Journal, "rapport : erreur du suivi : " .. tostring(BGD_RapportSuiviErr));
end
end
