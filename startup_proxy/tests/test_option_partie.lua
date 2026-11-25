-- Test hors jeu de src/option_partie.lua : les lignes des options « No First/Last moves (BGE) » et
-- « Disable Advanced UI indicators (BGE) » ajoutées à la table Parameters avant chaque lecture des
-- options de partie.
--   lua startup_proxy/tests/test_option_partie.lua
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end

local dir = (arg and arg[0] or ""):match("^(.*)[/\\]") or ".";
local BLOC = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/option_partie.lua");

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Base simulée : SELECT et INSERT sur Parameters, ou écriture refusée.
local function NewScreen(lang, refuse, rows, actif)
	local rows, journal, discovered = rows or {}, {}, 0;
	local DB = {
		ConfigurationQuery = function(sql, ...)
			local args = { ... };
			if sql:find("^INSERT") then
				if refuse then error("attempt to write a readonly database"); end
				table.insert(rows, { ParameterId = args[1], Name = args[2], Description = args[3], SortIndex = args[4], sql = sql,
					DefaultValue = tonumber(sql:match("'bool', (%d+),")),
					ChangeableAfterGameStart = tonumber(sql:match("'GameOptions', 0, 0, 0, (%d)")) });
				return {};
			end
			if sql:find("^UPDATE") then
				if refuse then error("attempt to write a readonly database"); end
				for _, row in ipairs(rows) do
					if row.ParameterId == args[1] then
						local key, value = sql:match("SET (%w+) = (%d+)");
						row[key] = tonumber(value);
					end
				end
				return {};
			end
			local r = {};
			for _, row in ipairs(rows) do
				if row.ParameterId == args[1] then
					-- Retail DB exposes this BOOLEAN column as a Lua boolean.
					if sql:find("SELECT ChangeableAfterGameStart", 1, true) then
						table.insert(r, {ChangeableAfterGameStart=row.ChangeableAfterGameStart == 1});
					else table.insert(r, row); end
				end
			end
			return r;
		end,
	};
	local SetupParameters = {};
	function SetupParameters:Data_DiscoverParameters() discovered = discovered + 1; return "liste"; end
	-- Règle du jeu simulée : le paramètre dit lui-même s'il est modifiable (client : false).
	function SetupParameters:Parameter_GetEnabled(p) return p.base ~= false; end
	local env = {
		-- Pas de rawget, rawset ni setfenv : HavokScript ne les a pas.
		pcall = pcall, print = function() end, tostring = tostring, tonumber = tonumber, type = type, string = string, ipairs = ipairs,
		DB = DB, SetupParameters = SetupParameters, QueryCache = { vieux = true },
		Locale = { GetCurrentLanguage = function() return { Type = lang }; end },
		Civ6Proxy_Journal = function(t) table.insert(journal, t); end,
		Civ6Proxy_Actif = actif ~= nil and function() return actif.valeur; end or nil,
	};
	assert(load(BLOC, "@option_partie.lua", "t", env))();
	env.Recharger = function()
		-- Nouvelle inclusion de SetupParameters.lua : la table est recréée, puis le bloc est relu.
		local S = {};
		function S:Data_DiscoverParameters() discovered = discovered + 1; return "liste"; end
		function S:Parameter_GetEnabled(p) return p.base ~= false; end
		env.SetupParameters = S;
		assert(load(BLOC, "@option_partie.lua", "t", env))();
	end;
	return env, rows, journal, function() return discovered; end;
end

do
	local env, rows, journal, discovered = NewScreen("fr_FR");
	local r = env.SetupParameters.Data_DiscoverParameters({});
	Check("lecture d'origine appelée, résultat rendu", r == "liste" and discovered() == 1);
	Check("trois lignes ajoutées, dans l'ordre", #rows == 3 and rows[1].ParameterId == "BGD_DEPART_SIMULTANE"
		and rows[2].ParameterId == "BGD_INDICATEURS_OFF");
	Check("libellé « No First/Last moves (BGE) », aide en français", rows[1].Name == "No First/Last moves (BGE)"
		and rows[1].Description:find("^Tous les joueurs commencent et finissent") ~= nil);
	Check("team pings: localized, disabled by default, network game option", rows[3].ParameterId == "BGD_TEAM_PINGS"
        and rows[3].SortIndex == 45 and rows[3].Name == "Signaux d’équipe (BGE)" and rows[3].DefaultValue == 0
        and rows[3].ChangeableAfterGameStart == 0);
    Check("position 43 dans les options", rows[1].SortIndex == 43);
	Check("réseau seulement, option de partie booléenne, cochée par défaut",
		rows[1].sql:find("'bool', 1, 'Game', %?1, 'GameOptions', 0, 0, 0, 1, %?4") ~= nil and rows[1].DefaultValue == 1);
	Check("cache des requêtes vidé", next(env.QueryCache) == nil);
	Check("libellé « Disable Advanced UI indicators (BGE) », aide en français, juste en dessous (44)",
		rows[2].Name == "Disable Advanced UI indicators (BGE)" and rows[2].SortIndex == 44
		and rows[2].Description:find("^Masque pour tous les joueurs les indicateurs avancés") ~= nil);
	Check("indicateurs : réseau seulement, booléenne, décochée par défaut",
		rows[2].sql:find("'bool', 0, 'Game', %?1, 'GameOptions', 0, 0, 0, 1, %?4") ~= nil and rows[2].DefaultValue == 0);
	Check("journal", #journal == 3 and journal[1] == "option : BGD_DEPART_SIMULTANE ajoutée à Parameters"
		and journal[2] == "option : BGD_INDICATEURS_OFF ajoutée à Parameters");
	env.SetupParameters.Data_DiscoverParameters({});
	Check("pas de doublon à la lecture suivante", #rows == 3 and discovered() == 2);
end

do
	local env, rows = NewScreen("en_US");
	env.SetupParameters.Data_DiscoverParameters({});
	Check("même libellé, aide en anglais", rows[1].Name == "No First/Last moves (BGE)" and rows[1].Description:find("^Every player starts and ends") ~= nil);
	Check("indicateurs : aide en anglais", rows[2].Description:find("^Hides BetterGameExperience's advanced indicators") ~= nil);
end

do
	-- Ligne d'une version précédente, décochée par défaut, restée dans la base.
	local env, rows, journal = NewScreen("fr_FR", false, { { ParameterId = "BGD_DEPART_SIMULTANE", DefaultValue = 0 },
		{ ParameterId = "BGD_INDICATEURS_OFF", DefaultValue = 1 }, { ParameterId = "BGD_TEAM_PINGS", DefaultValue = 1 } });
	env.SetupParameters.Data_DiscoverParameters({});
	Check("ancienne ligne : cochée par défaut, sans doublon", #rows == 3 and rows[1].DefaultValue == 1);
	Check("ancienne ligne des indicateurs : décochée par défaut", rows[2].DefaultValue == 0);
	Check("ancienne ligne des signaux d'équipe (pré-0.10.0, cochée) : décochée par défaut", rows[3].DefaultValue == 0);
	Check("ancienne ligne : cache vidé, journal", next(env.QueryCache) == nil
		and #journal == 3 and journal[1] == "option : BGD_DEPART_SIMULTANE : cochée par défaut"
		and journal[2] == "option : BGD_INDICATEURS_OFF : décochée par défaut"
		and journal[3] == "option : BGD_TEAM_PINGS : décochée par défaut");
	env.SetupParameters.Data_DiscoverParameters({});
	Check("ancienne ligne : une seule mise à jour", #journal == 3);
end

do
	local env, rows, journal, discovered = NewScreen("fr_FR", true);
	local r = env.SetupParameters.Data_DiscoverParameters({});
	Check("écriture refusée : l'écran se construit quand même", r == "liste" and discovered() == 1 and #rows == 0);
	Check("écriture refusée : journalisée une fois", #journal == 1 and journal[1]:find("erreur") ~= nil);
	env.SetupParameters.Data_DiscoverParameters({});
	Check("pas de répétition au journal", #journal == 1);
end

do
	-- Options grisées chez un hôte dont BetterGameExperience est désactivé.
	local actif = { valeur = 0 };
	local env, _, journal = NewScreen("fr_FR", false, nil, actif);
	local S = env.SetupParameters;
	Check("BGE désactivé : les trois options grisées",
		not S:Parameter_GetEnabled({ ParameterId = "BGD_DEPART_SIMULTANE" })
		and not S:Parameter_GetEnabled({ ParameterId = "BGD_INDICATEURS_OFF" })
		and not S:Parameter_GetEnabled({ ParameterId = "BGD_TEAM_PINGS" }));
	Check("BGE désactivé : les options du jeu restent modifiables", S:Parameter_GetEnabled({ ParameterId = "GameSpeeds" }) == true);
	Check("BGE désactivé : journalisé une fois", #journal == 1 and journal[1]:find("grisées") ~= nil);
	actif.valeur = 1;
	Check("BGE réactivé : options modifiables", S:Parameter_GetEnabled({ ParameterId = "BGD_TEAM_PINGS" }) == true
		and #journal == 2 and journal[2]:find("modifiables") ~= nil);
	Check("client : règle du jeu gardée", S:Parameter_GetEnabled({ ParameterId = "BGD_TEAM_PINGS", base = false }) == false);
	env.Recharger();
	actif.valeur = 0;
	Check("SetupParameters.lua inclus une deuxième fois : nouvelle table enveloppée aussi",
		env.SetupParameters:Parameter_GetEnabled({ ParameterId = "BGD_TEAM_PINGS" }) == false);
	local S = env.SetupParameters;
	assert(load(BLOC, "@option_partie.lua", "t", env))();
	Check("bloc relu sur la même table : une seule enveloppe", env.SetupParameters == S
		and S:Parameter_GetEnabled({ ParameterId = "BGD_TEAM_PINGS" }) == false);
	actif.valeur = nil;
	local env2 = NewScreen("fr_FR");
	Check("sans la native (ancien proxy) : options modifiables",
		env2.SetupParameters:Parameter_GetEnabled({ ParameterId = "BGD_DEPART_SIMULTANE" }) == true);
end

do
	-- Existing rows with the old flag are migrated even when the default is already correct.
	local env, rows = NewScreen("en_US", false, {
		{ ParameterId="BGD_DEPART_SIMULTANE", DefaultValue=1, ChangeableAfterGameStart=1 },
		{ ParameterId="BGD_INDICATEURS_OFF", DefaultValue=0, ChangeableAfterGameStart=1 },
		{ ParameterId="BGD_TEAM_PINGS", DefaultValue=0, ChangeableAfterGameStart=1 },
	});
	env.SetupParameters:Data_DiscoverParameters();
	Check("existing ping row locked after start; other settings unchanged", rows[3].ChangeableAfterGameStart == 0
		and rows[1].ChangeableAfterGameStart == 1 and rows[2].ChangeableAfterGameStart == 1);
	Check("old database metadata invalidates query cache", next(env.QueryCache) == nil);
	env.QueryCache = {unchanged=true};
	env.SetupParameters:Data_DiscoverParameters();
	Check("retail boolean columns do not trigger repeated writes/cache invalidation", env.QueryCache.unchanged);
end

do
	local env = NewScreen("en_US", false, nil, {valeur=1});
	local S = env.SetupParameters;
	local values = {};
	local pol, session, gameState = 0, true, 0;
	local peers = {
		[0]={announcement="current@lobby#host", status=1, human=true},
		[1]={announcement="current@lobby#peer", status=1, human=true},
		[2]={status=3, human=false}, -- Native observer, without BGE.
	};
	env.Network = {IsInSession=function() return session; end, GetGameHostPlayerID=function() return 0; end,
		IsPlayerConnected=function(id) return not peers[id].offline; end};
	env.GameStateTypes = {GAMESTATE_PREGAME=0};
	env.SlotStatus = {SS_TAKEN=1, SS_OBSERVER=3};
	env.GameConfiguration = {GetValue=function(k) return values[k]; end,
		GetGameState=function() return gameState; end, GetMultiplayerPlayerIDs=function() return {0,1,2}; end};
	env.Civ6Proxy_PolActifs = function() return pol; end;
	env.Civ6Proxy_Version = function() return "current"; end;
	env.PlayerConfigurations = {};
	for id, peer in pairs(peers) do
		env.PlayerConfigurations[id] = {GetValue=function() return peer.announcement; end,
			IsHuman=function() return peer.human; end, GetSlotStatus=function() return peer.status; end};
	end
	local function Enabled(id, base) return S:Parameter_GetEnabled({ParameterId=id, base=base}); end
	local ping, depart, ui = "BGD_TEAM_PINGS", "BGD_DEPART_SIMULTANE", "BGD_INDICATEURS_OFF";
	Check("pending/mixed POL cannot enable pings or simultaneous turns", not Enabled(ping) and not Enabled(depart));
	Check("advanced UI restriction and stock settings do not require POL", Enabled(ui) and Enabled("GameSpeeds"));
	values[ping], values[depart] = true, 1;
	Check("saved on choices can be disabled in mixed lobbies", Enabled(ping) and Enabled(depart));
	values[ping], values[depart] = false, false;
	Check("mixed lobby cannot re-enable a disabled choice", not Enabled(ping) and not Enabled(depart));
	pol = 1;
	Check("pings require observers before being selected; other POL does not", not Enabled(ping) and Enabled(depart));
	for _, bad in ipairs({"old@lobby#observer", "current@old#observer", "current@lobby#"}) do
		peers[2].announcement = bad;
		Check("incompatible/stale observer blocks pings: "..bad, not Enabled(ping));
	end
	peers[2].announcement = "current@lobby#observer";
	Check("compatible lobby can enable both POL settings", Enabled(ping) and Enabled(depart));
	peers[1].announcement = nil;
	Check("new incompatible human blocked before cached POL updates", not Enabled(ping) and not Enabled(depart));
	peers[1].offline = true;
	Check("disconnected seats ignored", Enabled(ping) and Enabled(depart));
	Check("client/stock restrictions preserved", not Enabled(ping, false) and not Enabled(ui, false));
	for _, state in ipairs({1,2}) do
		gameState = state; -- Running game and loaded-save lobby, regardless of cached DB metadata.
		for _, value in ipairs({false,true}) do
			values[ping] = value;
			Check("pings locked after start for either saved value", not Enabled(ping));
		end
		Check("UI restriction and simultaneous-turn option retain live behavior", Enabled(ui) and Enabled(depart));
	end
	gameState, session, pol = 0, false, 0;
	Check("before session creation host can prepare choices", Enabled(ping) and Enabled(depart));
	session = true;
	values[ping] = false;
	env.Civ6Proxy_PolActifs = nil;
	Check("missing POL native fails closed", not Enabled(ping) and not Enabled(depart) and Enabled(ui));
	env.Civ6Proxy_PolActifs = function() error("unavailable"); end;
	Check("failed POL native leaves normal controls intact", not Enabled(ping) and Enabled(ui));
end

-- Annonce restée sur un emplacement après un swap (rapport BGE-141E15) : son identité n'est pas
-- celle de l'occupant, ou, sans identité, son jeton personnel est aussi sur un autre emplacement.
do
	local env = NewScreen("en_US", false, nil, {valeur=1});
	local S = env.SetupParameters;
	local values = {};
	local seats = {
		[0]={BGD_VERSION="current@lobby#h~100", NETWORK_NAME="Host@100"},
		[1]={BGD_VERSION="current@lobby#m~111", NETWORK_NAME="Mateo@111"},
		[2]={BGD_VERSION="current@lobby#m~111", NETWORK_NAME="StulioS@222"},
	};
	env.Network = {IsInSession=function() return true; end, GetGameHostPlayerID=function() return 0; end,
		IsPlayerConnected=function() return true; end};
	env.GameStateTypes = {GAMESTATE_PREGAME=0};
	env.SlotStatus = {SS_TAKEN=1, SS_OBSERVER=3};
	env.GameConfiguration = {GetValue=function(k) return values[k]; end,
		GetGameState=function() return 0; end, GetMultiplayerPlayerIDs=function() return {0,1,2}; end};
	env.Civ6Proxy_PolActifs = function() return 1; end;
	env.Civ6Proxy_Version = function() return "current"; end;
	env.PlayerConfigurations = {};
	for id, seat in pairs(seats) do
		env.PlayerConfigurations[id] = {GetValue=function(_, k) return seat[k]; end,
			IsHuman=function() return true; end, GetSlotStatus=function() return 1; end};
	end
	local depart = "BGD_DEPART_SIMULTANE";
	values[depart] = false;
	Check("inherited announcement (other identity) blocks POL settings", not S:Parameter_GetEnabled({ParameterId=depart}));
	seats[2].BGD_VERSION = "current@lobby#s~222";
	Check("own announcements on every seat allow POL settings", S:Parameter_GetEnabled({ParameterId=depart}));
	for _, seat in pairs(seats) do seat.NETWORK_NAME = nil; seat.BGD_VERSION = seat.BGD_VERSION:gsub("~%d+$", ""); end
	seats[2].BGD_VERSION = seats[1].BGD_VERSION;
	Check("without identities, a personal token on two seats blocks POL settings", not S:Parameter_GetEnabled({ParameterId=depart}));
end

print(failures == 0 and "tout est bon" or (failures .. " échec(s)"));
os.exit(failures == 0 and 0 or 1);
