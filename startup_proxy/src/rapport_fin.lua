-- ===========================================================================
-- BetterGameExperience : fin de l'envoi d'un rapport de bug (voir startup_proxy/src/rapport.rs).
-- Placé devant rapport_suivi.lua (InGame) et devant rapport.lua (menu Échap) : celui des deux qui
-- reçoit le résultat termine le rapport, une seule fois.
--   BGD_RapportTerminer(statut, texte, archive, journal)
--     - en multijoueur, un message à tous : envoi terminé, ou en échec (la pause peut être levée) ;
--     - une ligne dans le chat que seul le joueur local voit : numéro du rapport, ou chemin du zip,
--       et le contact (@goaterino en message privé Discord). LuaEvents.BGD_DepartTexte, le canal
--       des lignes locales du chat (proxy de GameCore, ui_depart_chat.lua) ;
--     - LuaEvents.BGD_RapportFini(statut) : le menu remet son état à zéro et ferme son popup.
-- ===========================================================================
if type(BGD_RapportTerminer) ~= "function" then
	function BGD_RapportTerminer(statut, texte, archive, journal)
		local ok = statut == "ok";
		local enLigne = GameConfiguration.IsNetworkMultiplayer() and not GameConfiguration.IsPlayByCloud();
		if enLigne then
			local nom = "?";
			local cfg = PlayerConfigurations[Network.GetLocalPlayerID()];
			if cfg ~= nil then nom = Locale.Lookup(cfg:GetPlayerName()); end
			local couleur = ok and "COLOR_Civ6Green" or "COLOR_Civ6Red";
			local cle = ok and "LOC_BGD_CHAT_RAPPORT_FIN" or "LOC_BGD_CHAT_RAPPORT_ECHEC";
			local message = "[" .. couleur .. "]BGE - " .. BGD_TxtAnglais(cle, nom) .. "[ENDCOLOR]";
			Network.SendChat(message, ChatTargetTypes.CHATTARGET_ALL, -1);
			journal("message : " .. message);
		end
		local ligne = ok and BGD_Txt("LOC_BGD_RAPPORT_LOCAL_OK", texte) or BGD_Txt("LOC_BGD_RAPPORT_LOCAL_ECHEC", archive);
		ligne = "[" .. (ok and "COLOR_Civ6Green" or "COLOR_Civ6Red") .. "]BGE - " .. ligne .. "[ENDCOLOR]";
		LuaEvents.BGD_DepartTexte(ligne);
		journal((ok and "rapport envoyé, numéro " or "rapport non envoyé : ") .. tostring(texte)
			.. ", archive : " .. tostring(archive) .. " ; ligne locale : " .. ligne);
		LuaEvents.BGD_RapportFini(statut);
	end
end
