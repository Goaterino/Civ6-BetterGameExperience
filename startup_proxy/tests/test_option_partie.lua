-- Test hors jeu de src/option_partie.lua : les lignes des options « No first moves (BGD) » et
-- « No last moves (BGD) » ajoutées à la table Parameters avant chaque lecture des options de partie.
--   lua startup_proxy/tests/test_option_partie.lua
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end

local dir = (arg and arg[0] or ""):match("^(.*)[/\\]") or ".";
local BLOC = ReadFile(dir .. "/../src/option_partie.lua");

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

-- Base simulée : SELECT et INSERT sur Parameters, ou écriture refusée.
local function NewScreen(lang, refuse)
	local rows, journal, discovered = {}, {}, 0;
	local DB = {
		ConfigurationQuery = function(sql, ...)
			local args = { ... };
			if sql:find("^INSERT") then
				if refuse then error("attempt to write a readonly database"); end
				table.insert(rows, { ParameterId = args[1], Name = args[2], Description = args[3], SortIndex = args[4], sql = sql });
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
	local env = {
		pcall = pcall, print = function() end, tostring = tostring, type = type, string = string, ipairs = ipairs,
		DB = DB, SetupParameters = SetupParameters, QueryCache = { vieux = true },
		Locale = { GetCurrentLanguage = function() return { Type = lang }; end },
		Civ6Proxy_Journal = function(t) table.insert(journal, t); end,
	};
	assert(load(BLOC, "@option_partie.lua", "t", env))();
	return env, rows, journal, function() return discovered; end;
end

do
	local env, rows, journal, discovered = NewScreen("fr_FR");
	local r = env.SetupParameters.Data_DiscoverParameters({});
	Check("lecture d'origine appelée, résultat rendu", r == "liste" and discovered() == 1);
	Check("deux lignes ajoutées", #rows == 2 and rows[1].ParameterId == "BGD_DEPART_SIMULTANE"
		and rows[2].ParameterId == "BGD_FIN_SIMULTANEE");
	Check("libellé « No first moves (BGD) », aide en français", rows[1].Name == "No first moves (BGD)" and rows[1].Description:find("^Tous les joueurs commencent") ~= nil);
	Check("libellé « No last moves (BGD) », aide en français", rows[2].Name == "No last moves (BGD)" and rows[2].Description:find("^Tous les joueurs finissent") ~= nil);
	Check("« No last moves » juste sous « No first moves »", rows[1].SortIndex == 43 and rows[2].SortIndex == 44);
	Check("réseau seulement, option de partie booléenne",
		rows[1].sql:find("'bool', 0, 'Game', %?1, 'GameOptions', 0, 0, 0, 1, %?4") ~= nil);
	Check("cache des requêtes vidé", next(env.QueryCache) == nil);
	Check("journal", journal[1] == "option : BGD_DEPART_SIMULTANE ajoutée à Parameters"
		and journal[2] == "option : BGD_FIN_SIMULTANEE ajoutée à Parameters");
	env.SetupParameters.Data_DiscoverParameters({});
	Check("pas de doublon à la lecture suivante", #rows == 2 and discovered() == 2);
end

do
	local env, rows = NewScreen("en_US");
	env.SetupParameters.Data_DiscoverParameters({});
	Check("mêmes libellés, aides en anglais", rows[1].Name == "No first moves (BGD)" and rows[1].Description:find("^Every player starts") ~= nil
		and rows[2].Name == "No last moves (BGD)" and rows[2].Description:find("^Every player ends") ~= nil);
end

do
	local env, rows, journal, discovered = NewScreen("fr_FR", true);
	local r = env.SetupParameters.Data_DiscoverParameters({});
	Check("écriture refusée : l'écran se construit quand même", r == "liste" and discovered() == 1 and #rows == 0);
	Check("écriture refusée : journalisée une fois", #journal == 1 and journal[1]:find("erreur") ~= nil);
	env.SetupParameters.Data_DiscoverParameters({});
	Check("pas de répétition au journal", #journal == 1);
end

print(failures == 0 and "tout est bon" or (failures .. " échec(s)"));
os.exit(failures == 0 and 0 or 1);
