-- Test hors jeu de src/ui_congres_cibles.lua et src/ui_congres.lua, sur les vrais fichiers du jeu :
--   lua proxy/tests/test_congres.lua [WorldCongress.lua] [WorldCongressPopup.lua]
-- On patche comme le proxy (`patch_congres_cibles`, `patch_congres`), puis on exécute avec des
-- bouchons du jeu : un spectateur (LEADER_SPECTATOR) et un votant, correctifs POL actifs ou non.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local GAME = [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\]];
local CONGRESS_PATH = arg[1] or (GAME .. [[Scripts\WorldCongress.lua]]);
local POPUP_PATH = arg[2] or (GAME .. [[UI\Additions\WorldCongressPopup.lua]]);

-- HavokScript accepte des annotations de type que Lua 5.4 refuse : on les retire.
local function Strip(text)
	for _, t in ipairs({ "table", "number", "string", "boolean", "object", "userdata", "ifunction", "function" }) do
		text = text:gsub("([%w_%)%]])%s*:%s*" .. t .. "%f[^%w_(]", "%1");
	end
	return text;
end

local failures = 0;
local function Check(cond, what)
	print((cond and "ok    " or "ÉCHEC ") .. what);
	if not cond then failures = failures + 1; end
end

-- Objet passe-partout : tout champ, tout appel rend un autre passe-partout.
local Magic;
local mt = {};
mt.__index = function() return Magic; end
mt.__call = function() return Magic; end
mt.__concat = function(a, b) return tostring(a == Magic and "" or a) .. tostring(b == Magic and "" or b); end
mt.__lt = function() return false; end
mt.__le = function() return false; end
Magic = setmetatable({}, mt);

local function Event()
	local e = { handlers = {} };
	function e.Add(f) table.insert(e.handlers, f); end
	function e.Remove(f)
		for i, g in ipairs(e.handlers) do
			if g == f then table.remove(e.handlers, i); return; end
		end
	end
	function e.Fire(...) for _, f in ipairs(e.handlers) do f(...); end end
	return e;
end
local function Events()
	return setmetatable({}, { __index = function(t, k) local e = Event(); rawset(t, k, e); return e; end });
end

-- Monde : joueur 0 votant, joueur 1 spectateur, joueur 2 votant ; tous se sont rencontrés.
local LEADERS = { [0] = "LEADER_GANDHI", [1] = "LEADER_SPECTATOR", [2] = "LEADER_CLEOPATRA" };
local function Hash(s)
	local h = 0;
	for i = 1, #s do h = (h * 31 + s:byte(i)) % 2147483647; end
	return h;
end

local function NewEnv(pol, localPlayer)
	local env = setmetatable({}, { __index = _G });
	env.Civ6RolloverProxy_Pol = function() return pol; end
	env.GameEvents = Events();
	env.Events = Events();
	env.LuaEvents = Events();
	local players = {};
	for id = 0, 2 do
		players[id] = {
			GetID = function() return id; end,
			GetDiplomacy = function() return { HasMet = function() return true; end }; end,
			IsTurnActive = function() return env.turnActive; end,
			GetFavor = function() return 0; end,
			IsHuman = function() return true; end,
		};
	end
	env.Players = players;
	env.PlayerManager = { GetAliveMajors = function() return { players[0], players[1], players[2] }; end };
	env.PlayerConfigurations = {};
	for id = 0, 2 do
		env.PlayerConfigurations[id] = { GetLeaderTypeName = function() return LEADERS[id]; end, IsAlive = function() return true; end };
	end
	env.DB = { MakeHash = Hash };
	env.segment = Hash("TURNSEG_WORLDCONGRESS_1");
	env.turnActive = true;
	env.ops = {};
	env.Game = {
		GetLocalPlayer = function() return localPlayer; end,
		GetCurrentTurnSegment = function() return env.segment; end,
		GetCurrentGameTurn = function() return 100; end,
		GetWorldCongress = function() return Magic; end,
	};
	env.UI = setmetatable({
		RequestPlayerOperation = function(_, op) table.insert(env.ops, "op:" .. tostring(op)); end,
		RequestAction = function(a) table.insert(env.ops, "action:" .. tostring(a)); end,
	}, { __index = function() return Magic; end });
	env.PlayerOperations = setmetatable({}, { __index = function(_, k) return k; end });
	env.ActionTypes = setmetatable({}, { __index = function(_, k) return k; end });
	env.GameConfiguration = setmetatable({ IsPaused = function() return false; end, IsHotseat = function() return false; end },
		{ __index = function() return Magic; end });
	env.GameCapabilities = { HasCapability = function() return true; end };
	env.Locale = { Lookup = function(k) return k; end, GetCurrentLanguage = function() return { Type = "fr_FR" }; end };
	env.dequeued = 0;
	env.UIManager = setmetatable({ DequeuePopup = function() env.dequeued = env.dequeued + 1; end }, { __index = function() return Magic; end });
	env.include = function() end;
	env.PlayerTypes = { NONE = -1 };
	env.table = setmetatable({ count = function(t) local n = 0; for _ in pairs(t) do n = n + 1; end return n; end },
		{ __index = table });
	return env;
end

-- ---------------------------------------------------------------------------------------------
-- Cibles des résolutions (script de gameplay).
local cibles = ReadFile(dir .. "/../src/ui_congres_cibles.lua") .. "\r\n"
	.. ReadFile(CONGRESS_PATH):gsub("PlayerManager%.GetAliveMajors%(%)", "BGD_MajorsCibles()");
local function Targets(pol, event)
	local env = NewEnv(pol, 0);
	env.GameEvents = Events();
	local chunk = assert(load(Strip(cibles), "@WorldCongress.lua", "t", env));
	chunk();
	local options = {};
	env.GameEvents[event].Fire(0, 0, options);
	return table.concat(options.ResolutionOptions or {}, ",");
end
for _, event in ipairs({ "WC_Validate_BorderControl", "WC_Validate_MigrationTreaty", "WC_Validate_ArmsControlTreaty",
	"WC_Validate_TradeTreaty", "WC_Validate_DiploVictory" }) do
	Check(Targets(1, event) == "0,2", event .. " : spectateur retiré, POL actifs (" .. Targets(1, event) .. ")");
	Check(Targets(0, event) == "0,1,2", event .. " : liste du jeu, POL coupés (" .. Targets(0, event) .. ")");
end

-- ---------------------------------------------------------------------------------------------
-- Fenêtre du congrès.
local popup = ReadFile(POPUP_PATH) .. "\r\n" .. ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua")
	.. ReadFile(dir .. "/../src/ui_congres.lua");
local function NewPopup(pol, localPlayer)
	local env = NewEnv(pol, localPlayer);
	env.Controls = Magic;
	env.ContextPtr = setmetatable({ IsVisible = function() return true; end, IsHidden = function() return false; end },
		{ __index = function() return Magic; end });
	env.InstanceManager = { new = function() return setmetatable({ m_iAllocatedInstances = 1 }, mt); end };
	env.PopupDialog = { new = function() return Magic; end };
	env.Input = Magic;
	env.Mouse = Magic;
	env.InputContext = Magic;
	env.PopupPriority = Magic;
	local chunk = assert(load(Strip(popup), "@WorldCongressPopup.lua", "t", env));
	chunk();
	return env;
end

local function Votes(env)
	local data = { votes = 0, cost = 0, data = { Hash = 1 } };
	env.OnVoteResolution(1, 1, data, { [0] = 0, 0, MaxVotes = 5 }, true)();
	return data.votes;
end

-- Spectateur, POL actifs.
local spec = NewPopup(1, 1);
spec.Events.WorldCongressStage1.Fire(1);
Check(table.concat(spec.ops, " ") == "op:WORLD_CONGRESS_SUBMIT_TURN action:ACTION_ENDTURN",
	"spectateur : tour du congrès soumis à l'ouverture de la session (" .. table.concat(spec.ops, " ") .. ")");
spec.Events.LocalPlayerTurnBegin.Fire();
Check(#spec.ops == 2, "spectateur : soumis une seule fois par segment");
Check(Votes(spec) == 0, "spectateur : bouton de vote sans effet");
Check(spec.CanSubmit() == true, "spectateur : bouton de validation actif (Fermer)");
local before = spec.dequeued;
spec.Events.LocalPlayerTurnEnd.Fire();
Check(spec.dequeued == before, "spectateur : la fin de son tour ne ferme pas la fenêtre pendant le vote");
spec.segment = Hash("TURNSEG_SINGLEPHASE");
spec.Events.LocalPlayerTurnEnd.Fire();
Check(spec.dequeued == before + 1, "spectateur : hors du vote, la fin de tour ferme comme avant");

-- Votant, POL actifs : rien ne change.
local voter = NewPopup(1, 0);
voter.Events.WorldCongressStage1.Fire(0);
Check(#voter.ops == 0, "votant : pas de soumission d'office");
Check(Votes(voter) == 1, "votant : le vote compte");
local closed = voter.dequeued;
voter.Events.LocalPlayerTurnEnd.Fire();
Check(voter.dequeued == closed + 1, "votant : la fin de tour ferme la fenêtre");

-- Spectateur, POL coupés : comportement du jeu.
local off = NewPopup(0, 1);
off.Events.WorldCongressStage1.Fire(1);
Check(#off.ops == 0, "spectateur sans POL : pas de soumission d'office");
Check(Votes(off) == 1, "spectateur sans POL : le vote compte, comme dans le jeu");

os.exit(failures == 0 and 0 or 1);
