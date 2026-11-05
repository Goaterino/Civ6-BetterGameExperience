-- ===========================================================================
-- Civ6RolloverProxy (src/ui_depart_chat.lua) : signaux du départ de tour simultané cachés dans le
-- chat (depart_tour/README.md). Ajouté par le proxy de GameCore à la fin de ChatPanel.lua, que le
-- remplaçant de MPH inclut avant de reprendre OnChat : il hérite donc de ce filtre.
-- - Les messages qui commencent par `.bgd_` (`.bgd_pret`, `.bgd_depart`) ne s'affichent pas, sauf
--   avec le réglage « Afficher les signaux du départ simultané » (depart_signaux=on).
-- - LuaEvents.BGD_DepartTexte(texte) ajoute une ligne locale, que ui_depart.lua emploie pour
--   prévenir le joueur ; rien n'est envoyé.
-- ===========================================================================
do
local BGD_DepartChatOk, BGD_DepartChatErr = pcall(function()
	if BGD_DepartChat ~= nil or OnChat == nil then
		return;
	end
	BGD_DepartChat = true;
	local BGD_OnChatBase = OnChat;
	OnChat = function(fromPlayer, toPlayer, text, eTargetType, playSounds)
		if type(text) == "string" and string.sub(text, 1, 5) == ".bgd_"
			and not (Civ6RolloverProxy_DepartSignaux ~= nil and Civ6RolloverProxy_DepartSignaux() == 1) then
			return;
		end
		return BGD_OnChatBase(fromPlayer, toPlayer, text, eTargetType, playSounds);
	end
	LuaEvents.BGD_DepartTexte.Add(function(texte)
		local ok, err = pcall(AddChatEntry, texte, Controls.ChatEntryStack, m_ChatInstances, Controls.ChatLogPanel);
		if not ok then
			print("BGD depart chat echec " .. tostring(err));
		end
	end);
end);
if not BGD_DepartChatOk then
	print("BGD depart chat echec " .. tostring(BGD_DepartChatErr));
end
end
