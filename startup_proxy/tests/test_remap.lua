-- Test hors jeu de src/remap.lua :
--   lua startup_proxy/tests/test_remap.lua
-- Le bloc est chargé seul, avec des bouchons du jeu et des fonctions natives du proxy. On vérifie
-- qu'un `.mph_ui_snap` de MPH arrivé entre son Network.RestartGame() et le redémarrage (OnRefresh
-- encore appelé en milieu de tour) ne demande pas d'instantané perdu avec le redémarrage, et rend
-- GAME_HOST_IS_JUST_RELOADING à la partie redémarrée.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local f = assert(io.open(dir .. "/../src/remap.lua", "rb"));
local BLOCK = f:read("a");
f:close();

local failures = 0;
local function check(cond, what)
	if not cond then
		failures = failures + 1;
		print("ÉCHEC : " .. what);
	end
end

local function NewEvent()
	local e = { handlers = {} };
	e.Add = function(h) table.insert(e.handlers, h); end
	return e;
end

-- Une interface du jeu : son propre environnement, le bloc chargé dedans.
local function Interface(state)
	local env = setmetatable({}, { __index = _G });
	env.Events = { GameCoreEventPublishComplete = NewEvent(), LoadScreenClose = NewEvent() };
	env.Network = {
		GetLocalPlayerID = function() return 0; end,
		GetGameHostPlayerID = function() return 0; end,
		IsPlayerResyncing = function() return false; end,
		IsPlayerHotJoining = function() return false; end,
		RequestSnapshot = function() state.snapshots = state.snapshots + 1; end,
		SendChat = function(text) table.insert(state.chat, text); end,
		BroadcastGameConfig = function() state.broadcasts = state.broadcasts + 1; end,
		RestartGame = function() state.restarts = state.restarts + 1; end,
	};
	env.GameConfiguration = {
		GetValue = function(k) return state.config[k]; end,
		SetValue = function(k, v) state.config[k] = v; end,
		IsNetworkMultiplayer = function() return true; end,
	};
	env.Civ6Proxy_Journal = function(text) table.insert(state.log, text); end;
	env.Civ6Proxy_RemapMark = function() state.marked = true; end;
	env.Civ6Proxy_RemapTake = function() local m = state.marked; state.marked = false; return m and 0 or -1; end;
	env.Civ6Proxy_RemapLeger = function() state.armed = true; end;
	env.Civ6Proxy_RemapLegerActif = function() return 0; end;
	local chunk = assert(load(BLOCK, "remap.lua", "t", env));
	chunk();
	return env;
end

local function State()
	return { snapshots = 0, chat = {}, broadcasts = 0, restarts = 0, config = {}, log = {}, marked = false, armed = false };
end

-- Cas normal : interface fraîche après le redémarrage, MPH dit `.mph_ui_snap`.
do
	local s = State();
	local ui = Interface(s);
	s.config.GAME_HOST_IS_JUST_RELOADING = "N";   -- remis par OnRefresh de MPH juste avant
	ui.BGD_RemapSnap();
	check(s.snapshots == 1, "sans redémarrage en cours, l'hôte charge sa propre partie");
	check(s.marked, "demande gardée dans le proxy");
end

-- La course : OnLocalHostRestart arme puis redémarre, et OnRefresh de MPH passe encore.
do
	local s = State();
	local ui = Interface(s);
	s.config.GAME_HOST_IS_JUST_RELOADING = "Y";
	ui.BGD_RemapLeger();
	ui.Network.RestartGame();
	check(s.armed, "premier chargement léger armé");
	s.config.GAME_HOST_IS_JUST_RELOADING = "N";   -- OnRefresh de MPH, avant son `.mph_ui_snap`
	ui.BGD_RemapSnap();
	check(s.snapshots == 0, "pas d'instantané demandé entre le redémarrage et son chargement");
	check(not s.marked, "pas de demande gardée pour la partie jetée");
	check(#s.chat == 0, "clients pas appelés");
	check(s.config.GAME_HOST_IS_JUST_RELOADING == "Y", "drapeau rendu à la partie redémarrée");
	check(s.broadcasts == 1, "configuration diffusée");

	-- La partie redémarrée : nouvelle interface, l'OnRefresh de MPH y voit « Y » et appelle le bloc.
	local ui2 = Interface(s);
	s.config.GAME_HOST_IS_JUST_RELOADING = "N";
	ui2.BGD_RemapSnap();
	check(s.snapshots == 1, "la partie redémarrée demande l'instantané");
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("test_remap : OK");
