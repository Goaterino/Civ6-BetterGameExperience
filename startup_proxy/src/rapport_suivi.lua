-- ===========================================================================
-- BetterGameExperience : suivi de l'envoi d'un rapport de bug (voir startup_proxy/src/rapport.rs).
-- Ajouté à la fin d'InGame.lua (ou de son remplaçant du même nom, celui de MPH), contexte toujours
-- affiché en partie : le menu Échap, lui, est caché dès que le joueur le ferme, et ses images
-- s'arrêtent. Le proxy a ajouté au XML un minuteur, BGD_RapportMinuteur (SlideAnim d'environ
-- 0,5 s, comme le minuteur de la minimap du jeu), qui tourne même partie en pause ; les événements
-- du jeu (GameCoreEventPublishComplete) aident hors pause.
-- Natives : Civ6Proxy_RapportEtat() (voir rapport.lua) et Civ6Proxy_Journal(texte).
--   LuaEvents.BGD_RapportSuivre()                         lancé par le menu au début de l'envoi ;
--   LuaEvents.BGD_RapportResultat(statut, texte, archive) le menu a lu le résultat (il était affiché) ;
--   la fin elle-même : BGD_RapportTerminer (rapport_fin.lua, placé devant ce bloc).
-- ===========================================================================
do
local BGD_RapportSuiviOk, BGD_RapportSuiviErr = pcall(function()
	if BGD_RapportSuivi ~= nil or Civ6Proxy_RapportEtat == nil then return; end

	local function Journal(texte)
		if Civ6Proxy_Journal ~= nil then pcall(Civ6Proxy_Journal, "rapport : " .. texte); end
	end

	local S = { actif = false, tics = 0 };
	BGD_RapportSuivi = S;
	local minuteur = Controls.BGD_RapportMinuteur;
	if minuteur == nil then
		Journal("minuteur absent du XML d'InGame, suivi par les événements du jeu seuls");
	end

	local function Terminer(statut, texte, archive)
		if not S.actif then return; end
		S.actif = false;
		local ok, err = pcall(BGD_RapportTerminer, statut, texte, archive, Journal);
		if not ok then Journal("erreur à la fin du rapport : " .. tostring(err)); end
	end

	local function Interroger(source)
		if not S.actif then return; end
		local ok, statut, texte, archive = pcall(Civ6Proxy_RapportEtat);
		if not ok then
			Journal("erreur du suivi : " .. tostring(statut));
		elseif statut ~= nil and statut ~= "" then
			Journal("résultat lu par le suivi d'InGame (" .. source .. ")");
			Terminer(statut, texte, archive);
		end
	end

	local function Relancer()
		if minuteur ~= nil then
			minuteur:SetToBeginning();
			minuteur:Play();
		end
	end

	if minuteur ~= nil then
		minuteur:RegisterEndCallback(function()
			if not S.actif then return; end
			S.tics = S.tics + 1;
			if S.tics == 1 then Journal("minuteur du suivi actif"); end
			Interroger("minuteur");
			if S.actif then Relancer(); end
		end);
	end
	Events.GameCoreEventPublishComplete.Add(function() Interroger("événements du jeu"); end);

	LuaEvents.BGD_RapportSuivre.Add(function()
		if S.actif then return; end
		S.actif = true;
		S.tics = 0;
		Journal("suivi de l'envoi lancé");
		Relancer();
	end);
	-- Le menu, affiché, a lu le résultat avant nous : c'est ce bloc qui termine quand même, pour que
	-- la fin passe toujours par le même chemin.
	LuaEvents.BGD_RapportResultat.Add(function(statut, texte, archive) Terminer(statut, texte, archive); end);
end);
if not BGD_RapportSuiviOk and Civ6Proxy_Journal ~= nil then
	pcall(Civ6Proxy_Journal, "rapport : erreur du suivi : " .. tostring(BGD_RapportSuiviErr));
end
end
