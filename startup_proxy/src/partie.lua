-- ===========================================================================
-- BetterGameDLLs : relais de l'hôte en cours de partie, ajouté à la fin de InGame.lua (ou de son
-- remplaçant du même nom, celui de MPH par exemple) par le proxy de démarrage
-- (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_PolActifs()     -> 1 si les correctifs POL sont actifs sur cette machine, sinon 0
--   Civ6Proxy_Journal(texte)  -> écrit dans RolloverProxy.log
--   Civ6Proxy_GardeMessages() -> reads fixed by the network message guard since the last call
--                                (see below)
--   Civ6Proxy_RelayedPairs()  -> pairs of players relayed through the host, "a-b,c-d"
--   Civ6Proxy_RelinkStart(partner, steamID) -> 1 if the direct link attempt started
--   Civ6Proxy_RelinkState(partner) -> 0 in progress, 1 linked directly, 2 failed
--   Civ6Proxy_RelayForgetPair(a, b) -> removes the pair from the local relay table (see below)
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
	-- Extra parentheses: GetValue of a missing key returns no value at all, and tostring() fails
	-- when given none (a player without BGE in a new game).
	local _, jetonAnnonce = string.match(tostring((cfg:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#");
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
			local v, jh, perso = string.match(tostring((pc:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
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
		local texte = "[COLOR_RED]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_ARRIVEE", table.concat(sans, ", ")) .. "[ENDCOLOR]";
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

-- Garde des messages réseau (startup_proxy/src/messages.rs) : quand elle a jeté un message abîmé
-- qui aurait gelé le jeu, le joueur le dit à tous dans le chat, en anglais (chaque machine équipée
-- l'affiche dans sa langue, ui_depart_chat.lua) : « <joueur>: BGE - Blocked a damaged network
-- message… ». Une correction faite avant l'arrivée en jeu (salon, chargement) est dite à la
-- première occasion. Au plus une ligne par minute : le reste est dans le journal. Sans chat (solo,
-- hotseat, PlayByCloud), le journal seulement.
if Civ6Proxy_GardeMessages ~= nil then
	local dernier = nil;
	local function BGD_PartieGarde()
		local n = Civ6Proxy_GardeMessages();
		if n == nil or n <= 0 then
			return;
		end
		BGD_PartieJournal("garde : " .. tostring(n) .. " lecture(s) corrigée(s)");
		local maintenant = os.clock();
		if dernier ~= nil and maintenant - dernier < 60 then
			return;
		end
		dernier = maintenant;
		if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud() then
			return;
		end
		local texte = "[COLOR_RED]BGE - " .. BGD_TxtAnglais("LOC_BGD_CHAT_GARDE") .. "[ENDCOLOR]";
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message : " .. texte);
	end
	Events.GameCoreEventPublishComplete.Add(function()
		local ok, err = pcall(BGD_PartieGarde);
		if not ok then
			BGD_PartieJournal("erreur : " .. tostring(err));
		end
	end);
end

-- Links relayed through the host (startup_proxy/src/relink.rs): two players whose direct link
-- failed when one of them arrived go through the host for the rest of the game, and both are
-- disconnected if the host leaves. During the simultaneous turn start (ui_depart.lua: everyone
-- waits, orders are held, nothing is in flight between them), the lower player ID of each pair
-- rebuilds the direct link: it says so in the chat, starts the attempt, holds its "ready" signal
-- until the result (BGD_RelinkHolding, which ui_depart.lua checks before sending `.bgd_pret`; the
-- host waits at most 60 s, an attempt lasts at most 45 s), then says the result. On success,
-- `.bgd_relinked <a> <b>` goes to everyone (hidden by ui_depart_chat.lua), and every machine with
-- BGE removes the pair from its relay table. At most MAX_TRIES attempts per pair per game. The
-- simultaneous turn start requires every human to have the same BGE version, so both players of
-- the pair have it.
if Civ6Proxy_RelayedPairs ~= nil and Civ6Proxy_RelinkStart ~= nil and Civ6Proxy_RelinkState ~= nil
	and Civ6Proxy_RelayForgetPair ~= nil then
	local SIGNAL = ".bgd_relinked ";
	local MAX_TRIES = 3;
	local R = { tries = {}, turn = -1, queue = {}, current = nil };

	local function Name(id)
		local pc = PlayerConfigurations[id];
		return pc and Locale.Lookup(pc:GetPlayerName()) or ("#" .. tostring(id));
	end

	-- "<player>: BGE - …" to everyone, in English (each machine with BGE translates it on arrival).
	local function Say(color, key, p, ...)
		local text = color .. "BGE - " .. BGD_TxtAnglais(key, Name(p.a), Name(p.b), ...) .. "[ENDCOLOR]";
		Network.SendChat(text, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message: " .. text);
	end

	local function Key(p)
		return p.a .. "-" .. p.b;
	end

	-- The pairs of the local table where this player has the lower ID, with the partner connected.
	local function MyPairs()
		local me = Network.GetLocalPlayerID();
		local t = {};
		for a, b in string.gmatch(tostring(Civ6Proxy_RelayedPairs() or ""), "(%d+)-(%d+)") do
			a, b = tonumber(a), tonumber(b);
			if a == me and Network.IsPlayerConnected(b) then
				table.insert(t, { a = a, b = b });
			end
		end
		return t;
	end

	local function Start(p)
		R.tries[Key(p)] = (R.tries[Key(p)] or 0) + 1;
		local pc = PlayerConfigurations[p.b];
		local steam = pc and tostring(pc:GetNetworkIdentifer() or "") or "";
		if not string.match(steam, "^%d+$") or Civ6Proxy_RelinkStart(p.b, steam) ~= 1 then
			BGD_PartieJournal("relink: attempt " .. Key(p) .. " not started (see the relink lines)");
			return false;
		end
		R.current = p;
		BGD_PartieJournal("relink: attempt " .. Key(p) .. " (" .. R.tries[Key(p)] .. "/" .. MAX_TRIES .. ")");
		Say("[COLOR_Civ6Yellow]", "LOC_BGD_CHAT_RELINK_DETECTED", p);
		return true;
	end

	-- True while an attempt is in progress: ui_depart.lua does not send `.bgd_pret` yet.
	-- Called when this machine would be ready, then twice a second while it holds.
	function BGD_RelinkHolding()
		local ok, holding = pcall(function()
			if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud() then
				return false;
			end
			if R.current ~= nil then
				local p = R.current;
				local state = Civ6Proxy_RelinkState(p.b);
				if state == 0 then
					return true;
				end
				R.current = nil;
				if state == 1 then
					Network.SendChat(SIGNAL .. p.a .. " " .. p.b, ChatTargetTypes.CHATTARGET_ALL, -1);
					Say("[COLOR_GREEN]", "LOC_BGD_CHAT_RELINK_FIXED", p);
				elseif (R.tries[Key(p)] or 0) >= MAX_TRIES then
					Say("[COLOR_RED]", "LOC_BGD_CHAT_RELINK_GAVE_UP", p, MAX_TRIES);
				else
					Say("[COLOR_RED]", "LOC_BGD_CHAT_RELINK_FAILED", p);
				end
			end
			local turn = Game.GetCurrentGameTurn();
			if R.turn ~= turn then
				R.turn = turn;
				R.queue = {};
				for _, p in ipairs(MyPairs()) do
					if (R.tries[Key(p)] or 0) < MAX_TRIES then
						table.insert(R.queue, p);
					end
				end
			end
			while #R.queue > 0 do
				if Start(table.remove(R.queue, 1)) then
					return true;
				end
			end
			return false;
		end);
		if not ok then
			BGD_PartieJournal("relink: error: " .. tostring(holding));
			R.current = nil;
			return false;
		end
		return holding;
	end

	-- Success announced by the initiator: the pair leaves the local table (the host's included).
	Events.MultiplayerChat.Add(function(fromPlayer, toPlayer, text, eTargetType)
		local ok, err = pcall(function()
			if type(text) ~= "string" or string.sub(text, 1, #SIGNAL) ~= SIGNAL then
				return;
			end
			local a, b = string.match(text, "^%.bgd_relinked (%d+) (%d+)");
			if a ~= nil and Civ6Proxy_RelayForgetPair(tonumber(a), tonumber(b)) == 1 then
				BGD_PartieJournal("relink: pair " .. a .. "-" .. b .. " removed (linked directly)");
			end
		end);
		if not ok then
			BGD_PartieJournal("relink: error: " .. tostring(err));
		end
	end);
end
