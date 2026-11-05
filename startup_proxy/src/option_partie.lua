-- ===========================================================================
-- BetterGameDLLs : option de partie « No first moves (BGD) » (BGD_DEPART_SIMULTANE, voir
-- depart_tour/README.md), ajoutée à la fin de SetupParameters.lua (ou de son remplaçant du même
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

	local ID = "BGD_DEPART_SIMULTANE";

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

	local function Presente()
		local r = DB.ConfigurationQuery("SELECT ParameterId FROM Parameters WHERE ParameterId = ?1", ID);
		return r ~= nil and #r > 0;
	end

	local nom, aide = "No first moves (BGD)", nil;
	if FR() then
		aide = "Tous les joueurs commencent leur tour au même instant : à la fin du passage de tour, chacun attend que tous aient fini de charger. Pendant l'attente, on peut regarder la carte, mais les ordres sont retenus et le minuteur ne tourne pas. Il faut que tous les joueurs aient BetterGameDLLs, à la même version. Remplace la synchronisation du tour de MPH, qui est coupée.";
	else
		aide = "Every player starts their turn at the same moment: at the end of turn processing, everyone waits until all players have finished loading. While waiting, you can look at the map, but orders are held and the turn timer does not run. Every player needs BetterGameDLLs, at the same version. Replaces MPH's turn synchronisation, which is turned off.";
	end

	local journalise = false;
	local function Ajouter()
		if Presente() then
			return;
		end
		DB.ConfigurationQuery("INSERT INTO Parameters (ParameterId, Name, Description, Domain, DefaultValue, "
			.. "ConfigurationGroup, ConfigurationId, GroupId, SupportsSinglePlayer, SupportsHotSeat, "
			.. "SupportsPlayByCloud, ChangeableAfterGameStart, SortIndex) "
			.. "VALUES (?1, ?2, ?3, 'bool', 0, 'Game', ?1, 'GameOptions', 0, 0, 0, 1, 43)", ID, nom, aide);
		local ok = Presente();
		if ok then
			-- Le cache de CachedQuery ne voit pas forcément l'écriture : on le vide.
			QueryCache = {};
		end
		if ok or not journalise then
			journalise = true;
			J(ok and "ligne ajoutée à Parameters" or "écriture refusée par DB.ConfigurationQuery, option absente");
		end
	end

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
