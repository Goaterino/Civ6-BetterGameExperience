-- ===========================================================================
-- BetterGameDLLs (src/mods/notifications.lua), inséré par le proxy dans NotificationPanel.lua juste
-- après include("NotificationPanel_", true), là où le joker aurait chargé le mod : les variables
-- locales du fichier (m_notifications) restent visibles, et Initialize() suit.
-- D'après NotificationClear (336tarot) : un bouton du panneau efface d'un clic les notifications que
-- le joueur peut fermer lui-même (les accords et les choix à faire restent). Réécrit : la liste vient
-- du panneau (m_notifications), sans l'anneau de 100 identifiants du mod, qui pouvait en perdre.
-- Chaque fermeture est celle d'un clic droit sur la notification.
-- With `ui_luxes` (src/ui_luxes.lua, inserted after this block), the luxury trade notifications
-- are instances of the panel too: the button dismisses them as well (BGD_Luxes_ToutFermer), and
-- that block calls BGD_Afficher when it adds or removes one.
-- The button stays at the bottom of the queue: the panel's stack grows upwards (StackGrowth="Top")
-- and InstanceManager moves an instance to the end of the stack each time it is taken or released,
-- so the button, taken once and never released, is the bottom one as long as it is taken before
-- every other instance. Whoever takes an instance of the panel without going through
-- OnDefaultAddNotification (ui_luxes.lua) calls BGD_Bouton first. When the game destroys the
-- instances of the panel (refresh, observer mode), the button is forgotten and made again.
-- ===========================================================================
do
	local BGD_AIDE = BGD_Txt("LOC_BGD_NOTIF_AIDE");

	local BGD_Add_jeu = OnDefaultAddNotification;
	local BGD_Release_jeu = ReleaseNotificationEntry;
	local m_bouton = nil;

	function BGD_Afficher()
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
		if type(BGD_Luxes_ToutFermer) == "function" then BGD_Luxes_ToutFermer(); end
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

	-- Before any other instance of the panel is taken: the button is the bottom one.
	function BGD_Bouton()
		if m_bouton == nil then BGD_Creer(); end
	end

	function OnDefaultAddNotification(pNotification)
		BGD_Bouton();
		BGD_Add_jeu(pNotification);
		BGD_Afficher();
	end

	-- The game destroys every instance of the panel, the button with them.
	local BGD_Refresh_jeu = OnNotificationRefreshRequested;
	if BGD_Refresh_jeu ~= nil then
		function OnNotificationRefreshRequested(...)
			m_bouton = nil;
			BGD_Refresh_jeu(...);
		end
	end
	local BGD_Observateur_jeu = OnStartObserverMode;
	if BGD_Observateur_jeu ~= nil then
		function OnStartObserverMode(...)
			m_bouton = nil;
			BGD_Observateur_jeu(...);
		end
	end

	function ReleaseNotificationEntry(playerID, notificationID, isShuttingDown)
		BGD_Release_jeu(playerID, notificationID, isShuttingDown);
		BGD_Afficher();
	end
end
