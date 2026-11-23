-- ===========================================================================
-- Civ6RolloverProxy (src/ui_luxes.lua), inserted by the proxy into NotificationPanel.lua right
-- after include("NotificationPanel_", true), like src/mods/notifications.lua, with `ui_luxes=on`
-- (off by default; and `ui_bandeau=on`, which computes the list): a notification for each player who has surplus
-- luxuries the local player does not own.
--
-- The top panel (src/mods/bandeau.lua) already marks its luxury button when another player has
-- such a luxury, and lists them in its tooltip. Trading one then takes: hover the button, read who
-- has what, find that player in the diplomacy ribbon, open the deal screen, pick the luxury. Here
-- each of these players gets a notification with the icon of the luxury button
-- (ICON_RESOURCE_TOYS): its text names the player and the luxuries, a click opens the deal screen
-- with these luxuries already on the other side of the deal, one copy each.
--
-- The list is not computed here: bandeau.lua publishes the one behind its tooltip
-- (LuaEvents.BGD_Luxes_Offres, { { joueur = player ID, luxes = { resource Index, ... } }, ... })
-- each time it changes, and this block asks for it once the panel is ready
-- (LuaEvents.BGD_Luxes_Demande). The notification and the tooltip therefore always agree, and a
-- notification is withdrawn as soon as the list no longer has its player: luxury traded away or
-- now owned, war, "no luxury trading" game modes.
--
-- These are not game notifications (NotificationManager has no entry, nothing is sent to the
-- other machines): each one is an ItemInstance of the panel, taken from m_genericItemIM like the
-- game's own and like the "clear" button of notifications.lua, and dressed as
-- OnDefaultAddNotification and RealizeStandardNotification dress theirs. They are kept out of
-- m_notifications, which the game walks expecting NotificationManager entries.
--   left click  : opens the deal. As EspionageOverview.lua does to trade a captured spy: the
--                 outgoing working deal is cleared, filled, then the session "MAKE_DEAL" is
--                 requested; DiplomacyDealView keeps a deal filled this way ("This allows other
--                 screens to pre-populate the deal"). With a deal already pending with that
--                 player, the pending deal is shown instead, as the game's "view deal" choice.
--   right click : dismisses. The luxuries dismissed stay so while that player keeps offering them;
--                 a new luxury from the same player brings the notification back. The "clear"
--                 button of notifications.lua dismisses them too, and stays below them (BGD_Bouton).
-- Display and the local player's own deal only. Everything runs under pcall; a failure writes
-- `Luxes echec <error>`.
-- ===========================================================================
do
local Luxes_Ok, Luxes_Err = pcall(function()
	if BGD_Luxes_Etat ~= nil or m_genericItemIM == nil or type(LateInitialize) ~= "function" then
		return;
	end
	BGD_Luxes_Etat = true;

	local ICONE = "ICON_RESOURCE_TOYS";
	-- The resource atlas has no 40 px sheet (38, 50, 64, 256): the 40 px slot is narrowed for it.
	local TAILLE = 38;
	local TAILLE_JEU = 40;
	-- Turns of the luxury in the deal: what the deal screen sets on a click (OnClickAvailableResource).
	local DUREE = 30;
	-- NotificationPanel.lua: MAX_WIDTH_INSTANCE, TITLE_OFFSET_NO_COUNT, and the spacing of
	-- RealizeNotificationSize.
	local LARGEUR_MAX = 500;
	local TITRE_Y = 5;
	local X_EXTRA, X_EXTRA_SORTIE, X_ZONE, Y_EXTRA = 20, 70, 215, 42;

	-- [player ID] = { joueur = player ID, luxes = { resource Index, ... }, instance = ItemInstance or nil }
	local m_offres = {};
	-- ["player:resource"] = true: dismissed by the user, until that player stops offering it.
	local m_fermees = {};

	local function Echec(err)
		print("Luxes echec " .. tostring(err));
	end

	local function Cle(joueur, luxe)
		return joueur .. ":" .. luxe;
	end

	-- No notification for an observer, a spectator (Better Spectator Mod), or without diplomacy.
	local function Actif()
		local id = Game.GetLocalPlayer();
		if id == nil or id < 0 or Players[id] == nil then return false; end
		local pc = PlayerConfigurations[id];
		if pc ~= nil and pc:GetLeaderTypeName() == "LEADER_SPECTATOR" then return false; end
		return GameCapabilities.HasCapability("CAPABILITY_DIPLOMACY");
	end

	-- Leader name, as in the tooltip of the top panel; the player's own name first for a human.
	local function Nom(joueur)
		local pc = PlayerConfigurations[joueur];
		if pc == nil then return tostring(joueur); end
		local nom = "";
		local leader = GameInfo.Leaders[pc:GetLeaderTypeName()];
		if leader ~= nil then nom = Locale.Lookup(leader.Name); end
		if pc:IsHuman() then
			local humain = pc:GetPlayerName();
			if type(humain) == "string" and humain ~= "" then
				humain = Locale.Lookup(humain);
				if nom == "" then
					nom = humain;
				elseif humain ~= nom then
					nom = humain .. " (" .. nom .. ")";
				end
			end
		end
		return nom;
	end

	local function Liste(luxes)
		local t = {};
		for _, luxe in ipairs(luxes) do
			local row = GameInfo.Resources[luxe];
			if row ~= nil then
				t[#t + 1] = "[ICON_" .. row.ResourceType .. "] " .. Locale.Lookup(row.Name);
			end
		end
		return table.concat(t, ", ");
	end

	-- An offer is shown while one of its luxuries has not been dismissed.
	local function Visible(offre)
		for _, luxe in ipairs(offre.luxes) do
			if not m_fermees[Cle(offre.joueur, luxe)] then return true; end
		end
		return false;
	end

	-- The panel is laid out again, as after a game notification. The "clear" button of
	-- notifications.lua shows when the panel has something to clear.
	local function Change()
		ProcessStackSizes();
		if type(BGD_Afficher) == "function" then pcall(BGD_Afficher); end
	end

	-- The instance goes back to the panel as the game expects to find it.
	local function Retirer(offre)
		local inst = offre.instance;
		if inst == nil then return; end
		offre.instance = nil;
		inst.MouseInArea:ClearMouseOverCallback();
		inst.MouseOutArea:ClearMouseExitCallback();
		inst.MouseOutArea:SetHide(true);
		inst.Icon:SetSizeVal(TAILLE_JEU, TAILLE_JEU);
		m_genericItemIM:ReleaseInstance(inst);
	end

	-- Left click: the deal screen, with the luxuries of the offer on the other player's side.
	local function Ouvrir(joueur)
		local offre = m_offres[joueur];
		local localID = Game.GetLocalPlayer();
		if offre == nil or not Actif() or Players[joueur] == nil then return; end
		UI.PlaySound("Play_UI_Click");
		if DealManager.HasPendingDeal(localID, joueur) then
			DealManager.ViewPendingDeal(localID, joueur);
			return;
		end
		DealManager.ClearWorkingDeal(DealDirection.OUTGOING, localID, joueur);
		local pDeal = DealManager.GetWorkingDeal(DealDirection.OUTGOING, localID, joueur);
		if pDeal ~= nil then
			for _, luxe in ipairs(offre.luxes) do
				local pItem = pDeal:AddItemOfType(DealItemTypes.RESOURCES, joueur);
				if pItem ~= nil then
					pItem:SetValueType(luxe);
					pItem:SetAmount(1);
					pItem:SetDuration(DUREE);
					-- Gone since the list was published: the deal opens without it.
					if not pItem:IsValid() then pDeal:RemoveItemByID(pItem:GetID()); end
				end
			end
		end
		DiplomacyManager.RequestSession(localID, joueur, "MAKE_DEAL");
	end

	-- Right click.
	local function Fermer(joueur)
		local offre = m_offres[joueur];
		if offre == nil then return; end
		for _, luxe in ipairs(offre.luxes) do
			m_fermees[Cle(joueur, luxe)] = true;
		end
		if offre.instance ~= nil then
			Retirer(offre);
			Change();
		end
	end

	local function Proteger(f, joueur)
		return function()
			local ok, err = pcall(f, joueur);
			if not ok then Echec(err); end
		end
	end

	-- As OnDefaultAddNotification: the icon is the mouse-in area, a larger rectangle tracks the
	-- mouse out once the text has slid out.
	local function Creer(offre)
		-- The "clear" button of notifications.lua takes its instance first: it stays at the bottom
		-- of the queue, below these notifications as below the game's.
		if type(BGD_Bouton) == "function" then pcall(BGD_Bouton); end
		local inst = m_genericItemIM:GetInstance();
		if inst == nil then return; end
		offre.instance = inst;
		inst.m_MouseIn = false;
		local ouvrir, fermer = Proteger(Ouvrir, offre.joueur), Proteger(Fermer, offre.joueur);
		inst.MouseInArea:RegisterCallback(Mouse.eLClick, ouvrir);
		inst.MouseInArea:RegisterCallback(Mouse.eRClick, fermer);
		inst.MouseOutArea:RegisterCallback(Mouse.eLClick, ouvrir);
		inst.MouseOutArea:RegisterCallback(Mouse.eRClick, fermer);
		inst.MouseInArea:RegisterMouseEnterCallback(function() OnMouseEnterNotification(inst); end);
		inst.MouseOutArea:RegisterMouseExitCallback(function() OnMouseExitNotification(inst); end);
		-- The game leaves the "wrong phase" tooltip of the previous notification on a reused instance.
		inst.MouseInArea:SetToolTipString("");

		local x, y, feuille = IconManager:FindIconAtlas(ICONE, TAILLE);
		if x ~= nil then
			inst.Icon:SetSizeVal(TAILLE, TAILLE);
			inst.Icon:SetTexture(x, y, feuille);
		end
		inst.IconBG:SetHide(false);
		inst.IconBGInvalidPhase:SetHide(true);

		-- One notification, not a stack.
		inst.CountImage:SetHide(true);
		inst.TitleCount:SetHide(true);
		inst.LeftArrow:SetHide(true);
		inst.RightArrow:SetHide(true);
		inst.PagePipStack:SetHide(true);
		inst.Pages:SetHide(true);
		inst.LeftArrow:ClearCallback(Mouse.eLClick);
		inst.RightArrow:ClearCallback(Mouse.eLClick);
		inst.TitleInfo:SetOffsetY(TITRE_Y);

		inst.NotificationSlide:Stop();
		inst.NotificationSlide:SetToBeginning();
	end

	-- Title, summary and sizes, as SetNotificationText, RealizeMaxWidth and RealizeNotificationSize.
	local function Ecrire(offre)
		local inst = offre.instance;
		if inst == nil then return; end
		inst.TitleInfo:SetString(Locale.ToUpper(BGD_Txt("LOC_BGD_LUXES_TITRE")));
		inst.Summary:SetString(BGD_Txt("LOC_BGD_LUXES_RESUME", Nom(offre.joueur), Liste(offre.luxes)));
		local ecranX = UIManager:GetScreenSizeVal();
		local retour = ecranX;
		inst.Summary:SetWrapWidth(retour);
		local largeur = inst.Summary:GetSizeX();
		if largeur > LARGEUR_MAX then
			retour = largeur / 1.7;
			inst.Summary:SetWrapWidth(retour);
		end
		largeur = math.max(inst.TitleInfo:GetSizeX(), inst.Summary:GetSizeX());
		if largeur > retour then inst.Summary:SetWrapWidth(largeur); end

		inst.ExpandedArea:SetSizeX(largeur + X_EXTRA);
		inst.NotificationSlide:SetEndVal((largeur - X_ZONE) + X_EXTRA, 0);
		inst.MouseOutArea:SetSizeX(largeur + X_EXTRA_SORTIE);
		if inst.m_MouseIn and inst.NotificationSlide:IsStopped() then
			inst.NotificationSlide:SetToEnd();
		end
		inst.ExpandedArea:SetSizeY(inst.Summary:GetSizeY() + inst.PagePipStack:GetSizeY() + Y_EXTRA);
	end

	-- The instances follow m_offres and m_fermees, in player order. `change`: an instance was
	-- already released by the caller.
	local function Synchroniser(change)
		local joueurs = {};
		for joueur in pairs(m_offres) do joueurs[#joueurs + 1] = joueur; end
		table.sort(joueurs);
		for _, joueur in ipairs(joueurs) do
			local offre = m_offres[joueur];
			if Visible(offre) then
				if offre.instance == nil then
					Creer(offre);
					change = true;
				end
				Ecrire(offre);
			elseif offre.instance ~= nil then
				Retirer(offre);
				change = true;
			end
		end
		if change then Change(); end
	end

	-- The list of bandeau.lua. It is copied: the table belongs to the top panel.
	local function Recevoir(offres)
		local nouvelles = {};
		if Actif() and type(offres) == "table" then
			for _, o in ipairs(offres) do
				if type(o) == "table" and type(o.joueur) == "number" and type(o.luxes) == "table" and #o.luxes > 0 then
					local luxes = {};
					for i, luxe in ipairs(o.luxes) do luxes[i] = luxe; end
					nouvelles[o.joueur] = { joueur = o.joueur, luxes = luxes };
				end
			end
		end
		-- A dismissal lasts as long as its luxury stays offered.
		local encore = {};
		for joueur, o in pairs(nouvelles) do
			for _, luxe in ipairs(o.luxes) do
				local cle = Cle(joueur, luxe);
				if m_fermees[cle] then encore[cle] = true; end
			end
		end
		m_fermees = encore;
		-- Withdrawn: the player is no longer in the list.
		local retire = false;
		for joueur, ancienne in pairs(m_offres) do
			local nouvelle = nouvelles[joueur];
			if nouvelle ~= nil then
				nouvelle.instance = ancienne.instance;
			elseif ancienne.instance ~= nil then
				Retirer(ancienne);
				retire = true;
			end
		end
		m_offres = nouvelles;
		Synchroniser(retire);
	end

	local function OnOffres(offres)
		local ok, err = pcall(Recevoir, offres);
		if not ok then Echec(err); end
	end

	-- The instances were destroyed with the panel's: nothing to release.
	local function Oublier()
		for _, offre in pairs(m_offres) do offre.instance = nil; end
	end

	-- "Clear" button of notifications.lua.
	function BGD_Luxes_ToutFermer()
		local ok, err = pcall(function()
			local joueurs = {};
			for joueur in pairs(m_offres) do joueurs[#joueurs + 1] = joueur; end
			for _, joueur in ipairs(joueurs) do Fermer(joueur); end
		end);
		if not ok then Echec(err); end
	end

	-- The game destroys every instance of the panel, then restores its own notifications.
	local RefreshRequested_jeu = OnNotificationRefreshRequested;
	function OnNotificationRefreshRequested(...)
		Oublier();
		RefreshRequested_jeu(...);
		local ok, err = pcall(Synchroniser, false);
		if not ok then Echec(err); end
	end

	local ObserverMode_jeu = OnStartObserverMode;
	function OnStartObserverMode(...)
		Oublier();
		m_offres, m_fermees = {}, {};
		ObserverMode_jeu(...);
	end

	-- Hotseat: the list is the one of the player who was playing.
	local LocalPlayerChanged_jeu = OnLocalPlayerChanged;
	function OnLocalPlayerChanged(...)
		LocalPlayerChanged_jeu(...);
		local ok, err = pcall(function()
			Recevoir({});
			LuaEvents.BGD_Luxes_Demande();
		end);
		if not ok then Echec(err); end
	end

	local LateInitialize_jeu = LateInitialize;
	function LateInitialize(...)
		LateInitialize_jeu(...);
		local ok, err = pcall(function()
			LuaEvents.BGD_Luxes_Offres.Add(OnOffres);
			-- The top panel may have published its list before this panel was loaded.
			LuaEvents.BGD_Luxes_Demande();
		end);
		if not ok then Echec(err); end
	end

	local Shutdown_jeu = OnShutdown;
	function OnShutdown(...)
		pcall(function() LuaEvents.BGD_Luxes_Offres.Remove(OnOffres); end);
		if Shutdown_jeu ~= nil then Shutdown_jeu(...); end
	end
end);
if not Luxes_Ok then
	print("Luxes echec " .. tostring(Luxes_Err));
end
end
