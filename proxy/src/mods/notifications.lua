-- ===========================================================================
-- BetterGameDLLs (src/mods/notifications.lua), inséré par le proxy dans NotificationPanel.lua juste
-- après include("NotificationPanel_", true), là où le joker aurait chargé le mod : les variables
-- locales du fichier (m_notifications) restent visibles, et Initialize() suit.
-- D'après NotificationClear (336tarot) : un bouton du panneau efface d'un clic les notifications que
-- le joueur peut fermer lui-même (les accords et les choix à faire restent). Réécrit : la liste vient
-- du panneau (m_notifications), sans l'anneau de 100 identifiants du mod, qui pouvait en perdre.
-- Chaque fermeture est celle d'un clic droit sur la notification.
-- ===========================================================================
do
	local BGD_FR_Notif = false;
	do
		local l = Locale.GetCurrentLanguage and Locale.GetCurrentLanguage();
		BGD_FR_Notif = l ~= nil and type(l.Type) == "string" and string.sub(string.lower(l.Type), 1, 2) == "fr";
	end
	local BGD_AIDE = BGD_FR_Notif
		and "Efface les notifications qu'on peut fermer soi-même (les accords et les choix à faire restent)."
		or "Clears the notifications you can dismiss yourself (deals and pending choices stay).";

	local BGD_Add_jeu = OnDefaultAddNotification;
	local BGD_Release_jeu = ReleaseNotificationEntry;
	local m_bouton = nil;

	local function BGD_Afficher()
		if m_bouton ~= nil then
			-- Le bouton est lui-même une instance du panneau.
			m_bouton.Top:SetHide(#m_genericItemIM.m_AllocatedInstances <= 1);
		end
	end

	local function BGD_ToutEffacer()
		local ePlayer = Game.GetLocalPlayer();
		if ePlayer == nil or ePlayer < 0 then return; end
		local ids = {};
		for _, entry in pairs(m_notifications[ePlayer] or {}) do
			for _, id in ipairs(entry.m_IDs or {}) do table.insert(ids, id); end
		end
		for _, id in ipairs(ids) do
			local pNotification = NotificationManager.Find(ePlayer, id);
			if pNotification ~= nil and pNotification:CanUserDismiss() then
				NotificationManager.Dismiss(pNotification:GetPlayerID(), pNotification:GetID());
			end
		end
		BGD_Afficher();
	end

	local function BGD_Creer()
		local ePlayer = Game.GetLocalPlayer();
		if ePlayer == nil or ePlayer < 0 then return; end
		m_bouton = m_genericItemIM:GetInstance();
		if m_bouton == nil then return; end
		m_bouton.Icon:SetTexture("QueueTrash");
		m_bouton.Icon:SetToolTipString(BGD_AIDE);
		m_bouton.CountImage:SetHide(true);
		m_bouton.TitleCount:SetHide(true);
		m_bouton.LeftArrow:SetHide(true);
		m_bouton.RightArrow:SetHide(true);
		m_bouton.PagePipStack:SetHide(true);
		local function Clic() UI.PlaySound("Play_UI_Click"); BGD_ToutEffacer(); end
		m_bouton.MouseInArea:RegisterCallback(Mouse.eLClick, Clic);
		m_bouton.MouseInArea:RegisterCallback(Mouse.eRClick, Clic);
		m_bouton.MouseOutArea:RegisterCallback(Mouse.eLClick, Clic);
		m_bouton.MouseOutArea:RegisterCallback(Mouse.eRClick, Clic);
	end

	function OnDefaultAddNotification(pNotification)
		if m_bouton == nil then BGD_Creer(); end
		BGD_Add_jeu(pNotification);
		BGD_Afficher();
	end

	function ReleaseNotificationEntry(playerID, notificationID, isShuttingDown)
		BGD_Release_jeu(playerID, notificationID, isShuttingDown);
		BGD_Afficher();
	end
end
