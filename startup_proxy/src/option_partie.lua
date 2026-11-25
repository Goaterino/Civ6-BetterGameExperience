-- ===========================================================================
-- BetterGameDLLs : options de partie « No First/Last moves (BGE) » (BGD_DEPART_SIMULTANE, voir
-- depart_tour/README.md) et « Disable Advanced UI indicators (BGE) » (BGD_INDICATEURS_OFF),
-- ajoutées à la fin de SetupParameters.lua (ou de son remplaçant du même nom, celui de MPH) par le
-- proxy de démarrage (startup_proxy/src/salon.rs). Fonctions natives :
--   Civ6Proxy_Journal(texte) -> écrit dans RolloverProxy.log
--   Civ6Proxy_Actif() -> 1 si BetterGameExperience est actif sur cette machine, sinon 0 ; les options
--     sont alors grisées (hôte qui a désactivé BGE)
--   Civ6Proxy_PolActifs() -> verdict POL courant ; Civ6Proxy_Version() -> version active locale
--
-- Les options de partie sont les lignes de la table Parameters de la base Configuration, lues par
-- Data_DiscoverParameters. Le proxy n'est pas un mod : avant chaque lecture, on y insère la ligne
-- si elle manque (la base est reconstruite à chaque changement de mods), puis on vide le cache des
-- requêtes de SetupParameters. Multijoueur en réseau seulement. Une machine sans BetterGameDLLs n'a
-- pas la ligne : elle ne voit pas l'option et ignore la valeur, qui voyage dans la config de partie
-- comme toute clé de mod. Pour BGD_DEPART_SIMULTANE, la valeur seule n'active rien : il faut aussi
-- que tous les humains aient BetterGameDLLs à la même version (ui_depart.lua, en partie).
-- BGD_INDICATEURS_OFF s'applique seule : elle ne fait que masquer des affichages de BetterGameDLLs.
-- Tout passe par pcall : en cas d'échec, l'option manque et le reste de l'écran est intact.
-- ===========================================================================
do
local BGD_OptionOk, BGD_OptionErr = pcall(function()
	-- Le garde est posé sur la table SetupParameters, pas dans une globale : chaque inclusion de
	-- SetupParameters.lua recrée la table (« SetupParameters = {} »), sans nos enveloppes. Avec un
	-- drapeau global, la deuxième inclusion s'arrêtait là et la table utilisée par le salon restait
	-- sans enveloppe (options jamais grisées). Pas de rawget : HavokScript ne l'a pas (tout le bloc
	-- échouait, options absentes) ; SetupParameters n'a pas de métatable, l'accès simple suffit.
	if SetupParameters == nil or SetupParameters.Data_DiscoverParameters == nil or SetupParameters.BGD_OptionPartie ~= nil then
		return;
	end
	SetupParameters.BGD_OptionPartie = true;

	local function J(texte)
		if Civ6Proxy_Journal ~= nil then
			Civ6Proxy_Journal("option : " .. texte);
		end
	end

	-- Départ et fin simultanés (ui_depart.lua), cochée par défaut : l'hôte peut la décocher.
	-- Indicateurs avancés coupés pour tous (BGD_INDICATEURS_OFF : x/y (+z) de la recherche et du
	-- dogme, production en réserve ; ui_avancement.lua, ui_debordement.lua), décochée par défaut,
	-- juste en dessous. Signaux d'équipe (BGD_TEAM_PINGS, proxy/src/mods/pings), décochée par
	-- défaut : l'hôte l'active.
	local OPTIONS = {
		{
			id = "BGD_DEPART_SIMULTANE", nom = "No First/Last moves (BGE)", ordre = 43, defaut = 1,
			aide = "LOC_BGD_OPTION_PARTIE_AIDE",
		},
		{
			id = "BGD_INDICATEURS_OFF", nom = "Disable Advanced UI indicators (BGE)", ordre = 44, defaut = 0,
			aide = "LOC_BGD_OPTION_INDICATEURS_AIDE",
		},
		{
			id = "BGD_TEAM_PINGS", nom = "LOC_BGD_PINGS_NAME", ordre = 45, defaut = 0,
			aide = "LOC_BGD_PINGS_HELP", avantPartie = true,
		},
	};

	local function Presente(id)
		local r = DB.ConfigurationQuery("SELECT ParameterId FROM Parameters WHERE ParameterId = ?1", id);
		return r ~= nil and #r > 0;
	end

	local refusJournalise = false;
	local function AjouterUne(o)
		local modifiable = o.avantPartie and 0 or 1;
		if Presente(o.id) then
			-- Ligne écrite par une version précédente avec une autre valeur par défaut, dans une base pas
			-- encore reconstruite : on remet celle-ci.
			local r = DB.ConfigurationQuery("SELECT DefaultValue FROM Parameters WHERE ParameterId = ?1", o.id);
			if r ~= nil and r[1] ~= nil and tostring(r[1].DefaultValue) ~= tostring(o.defaut) then
				pcall(DB.ConfigurationQuery, "UPDATE Parameters SET DefaultValue = " .. o.defaut .. " WHERE ParameterId = ?1", o.id);
				QueryCache = {};
				J(o.id .. (o.defaut == 1 and " : cochée par défaut" or " : décochée par défaut"));
			end
			-- Les bases déjà ouvertes peuvent encore porter l'ancien drapeau des signaux.
			local r = DB.ConfigurationQuery("SELECT ChangeableAfterGameStart FROM Parameters WHERE ParameterId = ?1", o.id);
			local v = r and r[1] and r[1].ChangeableAfterGameStart;
			-- DB.ConfigurationQuery peut rendre les colonnes BOOLEAN comme booléens Lua.
			if r ~= nil and r[1] ~= nil and (v == true or v == 1 or v == "1") ~= (modifiable == 1) then
				DB.ConfigurationQuery("UPDATE Parameters SET ChangeableAfterGameStart = " .. modifiable .. " WHERE ParameterId = ?1", o.id);
				QueryCache = {};
			end
			return;
		end
		local avant = DB.ConfigurationChanges and DB.ConfigurationChanges();
		DB.ConfigurationQuery("INSERT INTO Parameters (ParameterId, Name, Description, Domain, DefaultValue, "
			.. "ConfigurationGroup, ConfigurationId, GroupId, SupportsSinglePlayer, SupportsHotSeat, "
			.. "SupportsPlayByCloud, ChangeableAfterGameStart, SortIndex) "
			.. "VALUES (?1, ?2, ?3, 'bool', " .. o.defaut .. ", 'Game', ?1, 'GameOptions', 0, 0, 0, " .. modifiable .. ", ?4)",
			o.id, BGD_Txt(o.nom), BGD_Txt(o.aide), o.ordre);
		local ok = Presente(o.id);
		if ok then
			-- Le cache de CachedQuery ne voit pas forcément l'écriture : on le vide.
			QueryCache = {};
		end
		if ok or not refusJournalise then
			refusJournalise = refusJournalise or not ok;
			local diag = "";
			if avant ~= nil then
				-- Diagnostic du bouton Prêt grisé au chargement d'une partie (salon.lua).
				diag = " (base " .. tostring(avant) .. " -> " .. tostring(DB.ConfigurationChanges()) .. ", état "
					.. tostring(GameConfiguration and GameConfiguration.GetGameState and GameConfiguration.GetGameState()) .. ")";
			end
			J((ok and (o.id .. " ajoutée à Parameters") or "écriture refusée par DB.ConfigurationQuery, options absentes") .. diag);
		end
	end

	local function Ajouter()
		for _, o in ipairs(OPTIONS) do
			AjouterUne(o);
		end
	end

	-- Options grisées chez un hôte dont BetterGameExperience est désactivé (actif=off, ou proxy de
	-- GameCore absent) : elles n'auraient pas d'effet chez lui, et il ne doit pas les régler pour les
	-- autres. Les clients ne peuvent de toute façon pas changer une option de partie (règle du jeu).
	-- La valeur reste celle de la config de partie. Native absente (ancien proxy) : rien ne change.
	local NOTRE = {};
	for _, o in ipairs(OPTIONS) do
		NOTRE[o.id] = o;
	end

	-- Disponibilité pour l'hôte, sans écrire ni le choix ni le verdict. Avant de créer le salon,
	-- il peut préparer ses choix. Dans le salon, le verdict doit être acquis et les annonces de
	-- tous les participants encore valides. Pour activer les signaux, vérifier les observateurs
	-- même quand l'option est décochée (le verdict ordinaire ne les inclut pas encore).
	function SetupParameters.BGD_PolDisponible(observateurs)
		if Network == nil or not Network.IsInSession() then return true; end
		if Civ6Proxy_PolActifs == nil or Civ6Proxy_PolActifs() ~= 1 or Civ6Proxy_Version == nil then return false; end
		local version = Civ6Proxy_Version();
		local hote = Network.GetGameHostPlayerID();
		-- Format et règle de salon.lua (BGD_LireAnnonce, BGD_AnnonceDe, BGD_AutresHumains) : changer
		-- les deux ensemble. Une annonce compte si son identité est celle de l'occupant actuel de
		-- l'emplacement (un swap laisse l'annonce sur l'emplacement) ; sans identité, si son jeton
		-- personnel n'est sur aucun autre emplacement.
		local function Brute(id)
			local pc = PlayerConfigurations[id];
			local v = pc and pc:GetValue("BGD_VERSION");
			if type(v) ~= "string" then return nil; end
			return string.match(v, "^([^@#:]*)@([^@#:]*)#([^@#:,]+)$");
		end
		local function Identite(id)
			local pc = PlayerConfigurations[id];
			if pc == nil then return ""; end
			local nom = pc:GetValue("NETWORK_NAME");
			local ident = type(nom) == "string" and string.match(nom, "@(%w+)$") or nil;
			if ident == nil and pc.GetNetworkIdentifer ~= nil then
				local ok, v = pcall(pc.GetNetworkIdentifer, pc);
				ident = ok and v ~= nil and tostring(v) or nil;
			end
			return string.sub(string.gsub(ident or "", "[^%w]", ""), 1, 32);
		end
		local occurrences = {};
		for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
			local _, _, perso = Brute(id);
			if perso ~= nil then occurrences[perso] = (occurrences[perso] or 0) + 1; end
		end
		local function Annonce(id)
			local v, j, perso = Brute(id);
			if v == nil then return nil; end
			local ident = string.match(perso, "~(%w*)$");
			if ident ~= nil and ident ~= Identite(id) then return nil; end
			if (ident == nil or ident == "") and occurrences[perso] > 1 then return nil; end
			return v, j;
		end
		local vh, jeton = Brute(hote);
		if version == nil or vh ~= version or jeton == nil or jeton == "" then return false; end
		for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
			local pc = PlayerConfigurations[id];
			if id ~= hote and pc ~= nil and Network.IsPlayerConnected(id)
				and ((pc:IsHuman() and pc:GetSlotStatus() == SlotStatus.SS_TAKEN)
					or (observateurs and pc:GetSlotStatus() == SlotStatus.SS_OBSERVER)) then
				local v, j = Annonce(id);
				if v ~= version or j ~= jeton then return false; end
			end
		end
		return true;
	end

	local griseesJournal = nil;
	local enabledBase = SetupParameters.Parameter_GetEnabled;
	if enabledBase ~= nil then
		SetupParameters.Parameter_GetEnabled = function(self, parameter, ...)
			local enabled = enabledBase(self, parameter, ...);
			local o = parameter and NOTRE[parameter.ParameterId];
			if not enabled or o == nil then return enabled; end
			-- Aussi vérifier ici : un paramètre déjà en cache peut porter l'ancien drapeau.
			if o.avantPartie and GameConfiguration ~= nil and GameStateTypes ~= nil
				and GameConfiguration.GetGameState() ~= GameStateTypes.GAMESTATE_PREGAME then return false; end
			if Civ6Proxy_Actif ~= nil then
				local ok, actif = pcall(Civ6Proxy_Actif);
				local grisee = ok and actif == 0;
				if grisee ~= griseesJournal then
					griseesJournal = grisee;
					J(grisee and "BetterGameExperience désactivé : options de partie BGE grisées"
						or "BetterGameExperience actif : options de partie BGE modifiables");
				end
				if grisee then
					return false;
				end
			end
			if o.id ~= "BGD_INDICATEURS_OFF" then
				local ok, disponible = pcall(SetupParameters.BGD_PolDisponible, o.id == "BGD_TEAM_PINGS");
				if not ok or not disponible then
					-- Un choix conservé (préréglage, ou joueur incompatible arrivé ensuite) peut être
					-- décoché, jamais recoché. Sinon les signaux cochés bloqueraient les autres POL
					-- avec un observateur incompatible, sans moyen de les couper.
					local v = GameConfiguration.GetValue(o.id);
					return v == true or v == 1;
				end
			end
			return enabled;
		end
	end

	local journalise = false;
	local base = SetupParameters.Data_DiscoverParameters;
	SetupParameters.Data_DiscoverParameters = function(self, ...)
		local ok, err = pcall(Ajouter);
		if not ok and not journalise then
			journalise = true;
			J("erreur : " .. tostring(err));
		end
		return base(self, ...);
	end
end);
if not BGD_OptionOk then
	print("BGD option echec " .. tostring(BGD_OptionErr));
	if Civ6Proxy_Journal ~= nil then
		Civ6Proxy_Journal("option : erreur au chargement : " .. tostring(BGD_OptionErr));
	end
end
end
