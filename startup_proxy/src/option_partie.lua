-- ===========================================================================
-- BetterGameDLLs : option de partie « No First/Last moves (BGD) » (BGD_DEPART_SIMULTANE), voir
-- depart_tour/README.md, ajoutée à la fin de SetupParameters.lua (ou de son remplaçant du même
-- nom, celui de MPH) par le proxy de démarrage (startup_proxy/src/salon.rs). Fonction native :
--   Civ6Proxy_Journal(texte) -> écrit dans RolloverProxy.log
--
-- Les options de partie sont les lignes de la table Parameters de la base Configuration, lues par
-- Data_DiscoverParameters. Le proxy n'est pas un mod : avant chaque lecture, on y insère la ligne
-- si elle manque (la base est reconstruite à chaque changement de mods), puis on vide le cache des
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

	-- Une seule option pour le départ et la fin simultanés (ui_depart.lua).
	local OPTIONS = {
		{
			id = "BGD_DEPART_SIMULTANE", nom = "No First/Last moves (BGE)", ordre = 43,
			aide = "LOC_BGD_OPTION_PARTIE_AIDE",
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
			o.id, o.nom, BGD_Txt(o.aide), o.ordre);
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
