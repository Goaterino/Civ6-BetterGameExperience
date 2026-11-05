-- ===========================================================================
-- BetterGameDLLs : options de partie « No first moves (BGD) » (BGD_DEPART_SIMULTANE) et « No last
-- moves (BGD) » (BGD_FIN_SIMULTANEE), voir depart_tour/README.md, ajoutées à la fin de SetupParameters.lua (ou de son remplaçant du même
-- nom, celui de MPH) par le proxy de démarrage (startup_proxy/src/salon.rs). Fonction native :
--   Civ6Proxy_Journal(texte) -> écrit dans RolloverProxy.log
--
-- Les options de partie sont les lignes de la table Parameters de la base Configuration, lues par
-- Data_DiscoverParameters. Le proxy n'est pas un mod : avant chaque lecture, on y insère les lignes
-- qui manquent (la base est reconstruite à chaque changement de mods), puis on vide le cache des
-- requêtes de SetupParameters. Multijoueur en réseau seulement. Une machine sans BetterGameDLLs n'a
-- pas la ligne : elle ne voit pas l'option et ignore la valeur, qui voyage dans la config de partie
-- comme toute clé de mod. La valeur seule n'active rien : il faut aussi que tous les humains aient
-- BetterGameDLLs à la même version (ui_depart.lua, en partie).
-- Tout passe par pcall : en cas d'échec, l'option manque et le reste de l'écran est intact.
-- ===========================================================================
do
local BGD_OptionOk, BGD_OptionErr = pcall(function()
	if BGD_OptionPartie ~= nil or SetupParameters == nil or SetupParameters.Data_DiscoverParameters == nil then
		return;
	end
	BGD_OptionPartie = true;

	local function J(texte)
		if Civ6Proxy_Journal ~= nil then
			Civ6Proxy_Journal("option : " .. texte);
		end
	end

	local function FR()
		local language = Locale.GetCurrentLanguage and Locale.GetCurrentLanguage();
		local code = language and language.Type;
		return type(code) == "string" and string.sub(string.lower(code), 1, 2) == "fr";
	end

	-- Les options, dans l'ordre de l'écran (SortIndex) : « No last moves » juste sous « No first moves ».
	local OPTIONS = {
		{
			id = "BGD_DEPART_SIMULTANE", nom = "No first moves (BGD)", ordre = 43,
			fr = "Tous les joueurs commencent leur tour au même instant : à la fin du passage de tour, chacun attend que tous aient fini de charger. Pendant l'attente, on peut regarder la carte, mais les ordres sont retenus et le minuteur ne tourne pas. Il faut que tous les joueurs aient BetterGameDLLs, à la même version. Remplace la synchronisation du tour de MPH, qui est coupée.",
			en = "Every player starts their turn at the same moment: at the end of turn processing, everyone waits until all players have finished loading. While waiting, you can look at the map, but orders are held and the turn timer does not run. Every player needs BetterGameDLLs, at the same version. Replaces MPH's turn synchronisation, which is turned off.",
		},
		{
			id = "BGD_FIN_SIMULTANEE", nom = "No last moves (BGD)", ordre = 44,
			fr = "Tous les joueurs finissent leur tour au même instant : quand le minuteur de l'hôte arrive à zéro, le tour se termine pour tout le monde, même si son minuteur affiche encore du temps (PC plus lent, départ plus tardif, durée différente). Sans effet sans minuteur de tour. Il faut que tous les joueurs aient BetterGameDLLs, à la même version.",
			en = "Every player ends their turn at the same moment: when the host's turn timer runs out, the turn ends for everyone, even if their own timer still shows time left (slower PC, later start, different duration). No effect without a turn timer. Every player needs BetterGameDLLs, at the same version.",
		},
	};

	local function Presente(id)
		local r = DB.ConfigurationQuery("SELECT ParameterId FROM Parameters WHERE ParameterId = ?1", id);
		return r ~= nil and #r > 0;
	end

	local refusJournalise = false;
	local function AjouterUne(o)
		if Presente(o.id) then
			return;
		end
		local avant = DB.ConfigurationChanges and DB.ConfigurationChanges();
		DB.ConfigurationQuery("INSERT INTO Parameters (ParameterId, Name, Description, Domain, DefaultValue, "
			.. "ConfigurationGroup, ConfigurationId, GroupId, SupportsSinglePlayer, SupportsHotSeat, "
			.. "SupportsPlayByCloud, ChangeableAfterGameStart, SortIndex) "
			.. "VALUES (?1, ?2, ?3, 'bool', 0, 'Game', ?1, 'GameOptions', 0, 0, 0, 1, ?4)",
			o.id, o.nom, FR() and o.fr or o.en, o.ordre);
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
