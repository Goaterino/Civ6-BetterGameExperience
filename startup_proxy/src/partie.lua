-- ===========================================================================
-- BetterGameDLLs : relais de l'hôte en cours de partie, ajouté à la fin de InGame.lua (ou de son
-- remplaçant du même nom, celui de MPH par exemple) par le proxy de démarrage
-- (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_PolActifs()     -> 1 si les correctifs POL sont actifs sur cette machine, sinon 0
--   Civ6Proxy_Journal(texte)  -> écrit dans RolloverProxy.log
--
-- Un joueur qui rejoint la partie en cours, ou s'y reconnecte, passe par le salon : il tire un
-- nouveau jeton et ne croit le verdict de l'hôte que s'il l'y trouve (salon.lua). Or l'hôte est en
-- jeu, son salon ne répond plus. Ce bloc le remplace : chaque fois qu'un humain connecté annonce la
-- version et le jeton de l'hôte, l'hôte ajoute son jeton au verdict de la partie, sans changer
-- l'état des correctifs POL (on ne peut pas les activer ou les couper en pleine partie sans
-- désynchroniser). Le nouveau venu conclut alors comme tout le monde.
-- Si les correctifs POL sont actifs et qu'un humain se met prêt sans s'être annoncé (sans le proxy
-- ou d'une autre version), l'hôte le dit dans le chat : sa partie va se désynchroniser. Les couper pour tous demanderait de
-- le faire au même point de la simulation sur chaque machine, ce qui n'est pas encore fait.
-- ===========================================================================

local BGD_PARTIE = { avertis = {} };

local function BGD_PartieJournal(texte)
	if Civ6Proxy_Journal ~= nil then
		Civ6Proxy_Journal("partie : " .. texte);
	end
end

local function BGD_PartieRelais()
	if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud()
		or not Network.IsGameHost() then
		return;
	end
	local localID = Network.GetLocalPlayerID();
	local cfg = PlayerConfigurations[localID];
	if cfg == nil then return; end
	-- Le verdict de la partie : "<version>@<jeton>:<1|0>:<jetons>", écrit par notre salon.
	local verdict = cfg:GetValue("BGD_VERDICT");
	local version, jeton, actif, liste = string.match(tostring(verdict), "^([^@#:]*)@([^@#:]*):([01]):(.*)$");
	local _, jetonAnnonce = string.match(tostring(cfg:GetValue("BGD_VERSION")), "^([^@#:]*)@([^@#:]*)#");
	if version == nil or jeton ~= jetonAnnonce then
		return;  -- pas de verdict de notre salon : partie lancée sans notre proxy
	end
	if (actif == "1") ~= (Civ6Proxy_PolActifs() == 1) then
		BGD_PartieJournal("verdict " .. verdict .. " différent de l'état local, verdict laissé tel quel");
		return;
	end
	local jetons = {};
	for j in string.gmatch(liste, "[^,]+") do jetons[j] = true; end
	local ajoutes, sans = {}, {};
	for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
		local pc = PlayerConfigurations[id];
		if id ~= localID and pc ~= nil and pc:IsHuman() and Network.IsPlayerConnected(id) then
			local v, jh, perso = string.match(tostring(pc:GetValue("BGD_VERSION")), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
			if v == version and jh == jeton then
				if not jetons[perso] then
					jetons[perso] = true;
					table.insert(ajoutes, perso);
				end
				BGD_PARTIE.avertis[id] = nil;
			elseif actif == "1" and pc:GetReady() and not BGD_PARTIE.avertis[id] then
				-- Prêt sans annonce : un joueur équipé s'annonce en ouvrant le salon, avant d'être prêt.
				BGD_PARTIE.avertis[id] = true;
				table.insert(sans, Locale.Lookup(pc:GetPlayerName()));
			end
		end
	end
	if #ajoutes > 0 then
		local nouveau = version .. "@" .. jeton .. ":" .. actif .. ":" .. liste
			.. ((liste ~= "") and "," or "") .. table.concat(ajoutes, ",");
		cfg:SetValue("BGD_VERDICT", nouveau);
		Network.BroadcastPlayerInfo();
		BGD_PartieJournal("verdict relayé : " .. nouveau);
	end
	if #sans > 0 then
		-- En anglais : chaque machine équipée l'affiche dans sa langue (ui_depart_chat.lua).
		local texte = "[COLOR_RED]BGD - " .. BGD_TxtAnglais("LOC_BGD_CHAT_ARRIVEE", table.concat(sans, ", ")) .. "[ENDCOLOR]";
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message : " .. texte);
	end
end

if Civ6Proxy_PolActifs ~= nil then
	local function protege()
		local ok, err = pcall(BGD_PartieRelais);
		if not ok then
			BGD_PartieJournal("erreur : " .. tostring(err));
		end
	end
	Events.PlayerInfoChanged.Add(protege);
	Events.MultiplayerPlayerConnected.Add(protege);
end
