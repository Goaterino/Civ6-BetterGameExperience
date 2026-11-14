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
					DefaultValue = tonumber(sql:match("'bool', (%d+),")) });
				return {};
			end
			if sql:find("^UPDATE") then
				for _, row in ipairs(rows) do
					if row.ParameterId == args[1] then row.DefaultValue = tonumber(sql:match("DefaultValue = (%d+)")); end
				end
				return {};
			end
			local r = {};
			for _, row in ipairs(rows) do
				if row.ParameterId == args[1] then table.insert(r, row); end
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
        and rows[3].sql:find("'GameOptions', 0, 0, 0, 1") ~= nil);
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

print(failures == 0 and "tout est bon" or (failures .. " échec(s)"));
os.exit(failures == 0 and 0 or 1);
