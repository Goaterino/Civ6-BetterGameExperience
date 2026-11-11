-- ===========================================================================
-- BetterGameDLLs : relais de l'hôte en cours de partie, ajouté à la fin de InGame.lua (ou de son
-- remplaçant du même nom, celui de MPH par exemple) par le proxy de démarrage
-- (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_PolActifs()     -> 1 si les correctifs POL sont actifs sur cette machine, sinon 0
--   Civ6Proxy_Journal(texte)  -> écrit dans RolloverProxy.log
--   Civ6Proxy_GardeMessages() -> lectures corrigées par la garde des messages réseau depuis
--                                le dernier appel (voir plus bas)
--   Civ6Proxy_Relais()        -> paires de joueurs relayées par l'hôte (« 0 ms »), "a-b,c-d"
--   Civ6Proxy_RelaisReparer(partenaire, steamID) -> 1 si la tentative de lien direct est lancée
--   Civ6Proxy_RelaisEtat(partenaire) -> 0 en cours, 1 relié en direct, 2 échec
--   Civ6Proxy_RelaisRetirer(a, b) -> retire la paire de la table locale des relais (voir plus bas)
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

-- Liens à 0 ms (startup_proxy/src/reliaison.rs) : deux joueurs dont le lien direct a échoué à
-- l'arrivée de l'un d'eux passent par l'hôte pour toute la partie (« 0 ms » dans la liste des
-- joueurs), et sont déconnectés tous les deux si l'hôte part. Pendant le départ de tour simultané
-- (ui_depart.lua : tout le monde attend, les ordres sont retenus, rien n'est en vol entre eux), le
-- plus petit identifiant de chaque paire refait le lien direct : il le dit dans le chat, lance la
-- tentative, garde son signal « prêt » jusqu'au résultat (BGD_RelaisRetient, que ui_depart.lua
-- consulte avant d'envoyer `.bgd_pret` ; l'hôte attend au plus 60 s, une tentative dure au plus
-- 45 s), puis dit le résultat. Réussite : `.bgd_relais <a> <b>` à tous (caché par
-- ui_depart_chat.lua), et chaque machine équipée retire la paire de sa table des relais. Au plus
-- MAX_ESSAIS tentatives par paire et par partie. Le départ simultané exige que tous les humains
-- aient BGE à la même version : les deux joueurs de la paire l'ont.
if Civ6Proxy_Relais ~= nil and Civ6Proxy_RelaisReparer ~= nil and Civ6Proxy_RelaisEtat ~= nil
	and Civ6Proxy_RelaisRetirer ~= nil then
	local RELAIS = ".bgd_relais ";
	local MAX_ESSAIS = 3;
	local R = { essais = {}, tour = -1, file = {}, encours = nil };

	local function Nom(id)
		local pc = PlayerConfigurations[id];
		return pc and Locale.Lookup(pc:GetPlayerName()) or ("#" .. tostring(id));
	end

	-- « <joueur>: BGE - … » à tous, en anglais (chaque machine équipée le traduit à l'arrivée).
	local function Dire(couleur, cle, p, ...)
		local texte = couleur .. "BGE - " .. BGD_TxtAnglais(cle, Nom(p.a), Nom(p.b), ...) .. "[ENDCOLOR]";
		Network.SendChat(texte, ChatTargetTypes.CHATTARGET_ALL, -1);
		BGD_PartieJournal("message : " .. texte);
	end

	local function Cle(p)
		return p.a .. "-" .. p.b;
	end

	-- Les paires de la table locale dont ce joueur est le plus petit identifiant, partenaire connecté.
	local function MesPaires()
		local moi = Network.GetLocalPlayerID();
		local t = {};
		for a, b in string.gmatch(tostring(Civ6Proxy_Relais() or ""), "(%d+)-(%d+)") do
			a, b = tonumber(a), tonumber(b);
			if a == moi and Network.IsPlayerConnected(b) then
				table.insert(t, { a = a, b = b });
			end
		end
		return t;
	end

	local function Lancer(p)
		R.essais[Cle(p)] = (R.essais[Cle(p)] or 0) + 1;
		local pc = PlayerConfigurations[p.b];
		local steam = pc and tostring(pc:GetNetworkIdentifer() or "") or "";
		if not string.match(steam, "^%d+$") or Civ6Proxy_RelaisReparer(p.b, steam) ~= 1 then
			BGD_PartieJournal("relais : tentative " .. Cle(p) .. " pas lancée (voir reliaison)");
			return false;
		end
		R.encours = p;
		BGD_PartieJournal("relais : tentative " .. Cle(p) .. " (" .. R.essais[Cle(p)] .. "/" .. MAX_ESSAIS .. ")");
		Dire("[COLOR_Civ6Yellow]", "LOC_BGD_CHAT_RELAIS_DETECTE", p);
		return true;
	end

	-- Vrai tant qu'une tentative est en cours : ui_depart.lua n'envoie pas encore `.bgd_pret`.
	-- Appelée quand la machine serait prête, puis deux fois par seconde tant qu'elle retient.
	function BGD_RelaisRetient()
		local ok, retenir = pcall(function()
			if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsPlayByCloud() then
				return false;
			end
			if R.encours ~= nil then
				local p = R.encours;
				local etat = Civ6Proxy_RelaisEtat(p.b);
				if etat == 0 then
					return true;
				end
				R.encours = nil;
				if etat == 1 then
					Network.SendChat(RELAIS .. p.a .. " " .. p.b, ChatTargetTypes.CHATTARGET_ALL, -1);
					Dire("[COLOR_GREEN]", "LOC_BGD_CHAT_RELAIS_REPARE", p);
				elseif (R.essais[Cle(p)] or 0) >= MAX_ESSAIS then
					Dire("[COLOR_RED]", "LOC_BGD_CHAT_RELAIS_ABANDON", p, MAX_ESSAIS);
				else
					Dire("[COLOR_RED]", "LOC_BGD_CHAT_RELAIS_ECHEC", p);
				end
			end
			local tour = Game.GetCurrentGameTurn();
			if R.tour ~= tour then
				R.tour = tour;
				R.file = {};
				for _, p in ipairs(MesPaires()) do
					if (R.essais[Cle(p)] or 0) < MAX_ESSAIS then
						table.insert(R.file, p);
					end
				end
			end
			while #R.file > 0 do
				if Lancer(table.remove(R.file, 1)) then
					return true;
				end
			end
			return false;
		end);
		if not ok then
			BGD_PartieJournal("relais : erreur : " .. tostring(retenir));
			R.encours = nil;
			return false;
		end
		return retenir;
	end

	-- Réussite annoncée par l'initiateur : la paire sort de la table locale (l'hôte compris).
	Events.MultiplayerChat.Add(function(fromPlayer, toPlayer, text, eTargetType)
		local ok, err = pcall(function()
			if type(text) ~= "string" or string.sub(text, 1, #RELAIS) ~= RELAIS then
				return;
			end
			local a, b = string.match(text, "^%.bgd_relais (%d+) (%d+)");
			if a ~= nil and Civ6Proxy_RelaisRetirer(tonumber(a), tonumber(b)) == 1 then
				BGD_PartieJournal("relais : paire " .. a .. "-" .. b .. " retirée (reliée en direct)");
			end
		end);
		if not ok then
			BGD_PartieJournal("relais : erreur : " .. tostring(err));
		end
	end);
end
