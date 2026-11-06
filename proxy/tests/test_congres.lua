-- Test hors jeu des blocs du congrès, sur les vrais fichiers du jeu :
--   lua proxy/tests/test_congres.lua [WorldCongress.lua] [WorldCongressPopup.lua]
-- On patche comme le proxy, puis on exécute avec des bouchons du jeu :
-- - src/ui_congres_cibles.lua et src/ui_congres.lua : un spectateur (LEADER_SPECTATOR) et un votant,
--   correctifs POL actifs ou non ; liste des luxes recalculée si POL ;
-- - src/ui_congres_vote.lua : pas de fin de tour sur un cache vide, retour au vote et faveur ;
-- - src/ui_congres_attente.lua : bouton de retour au vote de l'écran d'attente ;
-- - src/ui_congres_fin_auto.lua et src/ui_congres_minuteur.lua.

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
	-- LuaEvents.X(…) déclenche l'événement, comme dans le jeu.
	return setmetatable(e, { __call = function(_, ...) e.Fire(...); end });
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
			IsAI = function() return false; end,
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
	.. ReadFile(CONGRESS_PATH):gsub("PlayerManager%.GetAliveMajors%(%)", "BGD_MajorsCibles()")
		:gsub("%(cachedLuxuryResources == nil%)", "(cachedLuxuryResources == nil or BGD_LuxesARecalculer())");
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

-- Luxes : un luxe apparu sur la carte entre deux sessions.
local function Luxes(pol)
	local env = NewEnv(pol, 0);
	env.GameEvents = Events();
	local surCarte = { [11] = 1, [12] = 0 };
	local resources = { { Hash = 11, ResourceClassType = "RESOURCECLASS_LUXURY", Frequency = 1, SeaFrequency = 0 },
		{ Hash = 12, ResourceClassType = "RESOURCECLASS_LUXURY", Frequency = 1, SeaFrequency = 0 } };
	env.GameInfo = setmetatable({ Resources = function() local i = 0; return function() i = i + 1; return resources[i]; end; end },
		{ __index = function() return nil; end });
	env.Map = { GetResourceCount = function(h) return surCarte[h] or 0; end };
	assert(load(Strip(cibles), "@WorldCongress.lua", "t", env))();
	local function Session()
		local options = {};
		env.GameEvents.WC_Validate_LuxuryBan.Fire(0, 0, options);
		return table.concat(options.ResolutionOptions or {}, ",");
	end
	local premiere = Session();
	surCarte[12] = 1;
	return premiere, Session();
end
local a, b = Luxes(1);
Check(a == "11" and b == "11,12", "luxes : recalculés à chaque session, POL actifs (" .. a .. " puis " .. b .. ")");
a, b = Luxes(0);
Check(a == "11" and b == "11", "luxes : cache du jeu, POL coupés (" .. a .. " puis " .. b .. ")");

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

-- ---------------------------------------------------------------------------------------------
-- Fenêtre du congrès : bloc du vote (ui_congres_vote.lua), ajouté après celui des spectateurs.
local popupVote = popup .. "\r\n" .. ReadFile(dir .. "/../src/ui_congres_vote.lua");
local function NewVoter()
	local env = NewEnv(1, 0);
	env.alloues = 1;
	env.resolutions = { Stage = 1, [1] = { Type = "WC_RES_X", FavoredPlayerIDs = {}, DisfavoredPlayerIDs = {}, PossibleTargets = {} } };
	env.favor = 100;
	env.lookups = {};
	env.hidden = 0;
	env.Locale = { Lookup = function(k, a) env.lookups[k] = a; return k; end, ToUpper = function(s) return s; end,
		GetCurrentLanguage = function() return { Type = "fr_FR" }; end };
	env.Players[0].GetFavor = function() return env.favor; end
	env.Game.GetWorldCongress = function()
		return setmetatable({
			GetResolutions = function() return env.resolutions; end,
			GetVotesandFavorCost = function() return { [0] = 0, 10, 30, 60, 100, MaxVotes = 5 }; end,
		}, mt);
	end
	env.GameInfo = Magic;
	env.Controls = Magic;
	env.ContextPtr = setmetatable({
		IsVisible = function() return true; end, IsHidden = function() return false; end,
		SetUpdate = function(_, f) env.update = f; end, ClearUpdate = function() env.update = nil; end,
	}, { __index = function() return Magic; end });
	env.InstanceManager = { new = function()
		return setmetatable({}, { __index = function(_, k)
			if k == "m_iAllocatedInstances" then return env.alloues; end
			return Magic;
		end });
	end };
	env.PopupDialog = { new = function() return Magic; end };
	env.Input = Magic;
	env.Mouse = Magic;
	env.InputContext = Magic;
	env.PopupPriority = Magic;
	assert(load(Strip(popupVote), "@WorldCongressPopup.lua", "t", env))();
	-- Les données de vote de la première résolution, telles que la fenêtre les lie à ses boutons.
	local UpdateVotingWidget = env.UpdateVotingWidget;
	env.UpdateVotingWidget = function(w, data, ...)
		env.voteA = env.voteA or data;
		return UpdateVotingWidget(w, data, ...);
	end
	env.LuaEvents.WorldCongressPopup_HideWorldCongressBetweenTurns.Add(function() env.hidden = env.hidden + 1; end);
	return env;
end
local function Ops(env) return table.concat(env.ops, " "); end

-- Cache vide à l'ouverture, résolutions arrivées ensuite.
local v = NewVoter();
v.alloues = 0;
v.resolutions = { Stage = 1 };
v.SetupWorldCongress(1, true);
Check(#v.ops == 0, "cache vide : pas de fin de tour automatique (" .. Ops(v) .. ")");
Check(v.update ~= nil, "cache vide : attente des résolutions");
v.update(1);
Check(#v.ops == 0 and v.update ~= nil, "cache vide : on attend encore");
v.alloues = 1;
v.resolutions = { Stage = 1, [1] = { Type = "WC_RES_X", FavoredPlayerIDs = {}, DisfavoredPlayerIDs = {}, PossibleTargets = {} } };
v.update(0.5);
Check(#v.ops == 0 and v.update == nil, "résolutions arrivées : fenêtre remplie, toujours pas de fin de tour");

-- Cache toujours vide : comportement du jeu après l'attente.
v = NewVoter();
v.alloues = 0;
v.resolutions = { Stage = 1 };
v.SetupWorldCongress(1, true);
v.update(11);
Check(Ops(v) == "op:WORLD_CONGRESS_SUBMIT_TURN action:ACTION_ENDTURN", "cache vide 10 s : envoi automatique du jeu (" .. Ops(v) .. ")");

-- Session extraordinaire sans proposition : envoi automatique du jeu, sans attente.
v = NewVoter();
v.alloues = 0;
v.segment = Hash("TURNSEG_WORLDCONGRESS_2");
v.SetupWorldCongress(2, true);
Check(Ops(v) == "op:WORLD_CONGRESS_SUBMIT_TURN action:ACTION_ENDTURN" and v.update == nil,
	"session extraordinaire sans choix : comportement du jeu (" .. Ops(v) .. ")");

-- Retour au vote : 3 voix à 30 de faveur, envoyées, puis fin de tour annulée.
v = NewVoter();
v.SetupWorldCongress(1, true);
Check(v.voteA ~= nil, "votant : données de vote de la première résolution");
v.voteA.votes = 3;
v.voteA.cost = 30;
v.UpdateWorkingFavor();
Check(v.lookups.LOC_WORLD_CONGRESS_FAVOR_TITLE == 70, "votant : 100 - 30 de faveur avant l'envoi");
v.OnAccept();
Check(Ops(v) == "op:WORLD_CONGRESS_RESOLUTION_VOTE op:WORLD_CONGRESS_SUBMIT_TURN action:ACTION_ENDTURN", "votant : vote envoyé (" .. Ops(v) .. ")");
v.favor = 70;	-- débitée par GameCore
v.Events.LocalPlayerTurnEnd.Fire();
v.Events.LocalPlayerTurnUnready.Fire();
Check(v.hidden == 1, "retour au vote : écran d'attente fermé");
Check(v.voteA.votes == 3, "retour au vote : votes envoyés remis dans la fenêtre");
Check(v.lookups.LOC_WORLD_CONGRESS_FAVOR_TITLE == 70,
	"retour au vote : faveur affichée = celle d'avant l'envoi (" .. tostring(v.lookups.LOC_WORLD_CONGRESS_FAVOR_TITLE) .. ")");
v.ops = {};
v.OnAccept();
Check(Ops(v) == "op:WORLD_CONGRESS_RESOLUTION_VOTE op:WORLD_CONGRESS_SUBMIT_TURN action:ACTION_ENDTURN", "retour au vote : nouvel envoi");
v.Events.LocalPlayerTurnUnready.Fire();
Check(v.hidden == 2, "retour au vote : possible une seconde fois");
v.segment = Hash("TURNSEG_SINGLEPHASE");
v.OnAccept();
v.Events.LocalPlayerTurnUnready.Fire();
Check(v.hidden == 2, "hors du vote : une annulation ne rouvre rien");

-- ---------------------------------------------------------------------------------------------
-- Écran d'attente : bouton de retour au vote (ui_congres_attente.lua).
local ATTENTE_PATH = POPUP_PATH:gsub("WorldCongressPopup%.lua$", "WorldCongressBetweenTurns.lua");
local attente = ReadFile(ATTENTE_PATH) .. "\r\n" .. ReadFile(dir .. "/../../textes/bgd_textes.lua")
	.. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. ReadFile(dir .. "/../src/ui_congres_attente.lua");
local function NewAttente(revote)
	local env = NewEnv(1, 0);
	env.Civ6RolloverProxy_Revote = function() return revote; end
	env.turnActive = false;
	env.canUnready = true;
	env.elapsed = false;
	env.Players[0].CanUnreadyTurn = function() return env.canUnready; end
	env.UI.IsTurnTimerElapsed = function() return env.elapsed; end
	env.UI.PlaySound = function() end
	env.bouton = { hidden = true };
	function env.bouton:SetHide(h) self.hidden = h; end
	function env.bouton:RegisterCallback(_, f) self.click = f; end
	function env.bouton:SetText(t) self.text = t; end
	function env.bouton:SetToolTipString() end
	env.Controls = setmetatable({ BGD_Revoter = env.bouton }, { __index = function() return Magic; end });
	env.ContextPtr = setmetatable({ IsVisible = function() return true; end }, { __index = function() return Magic; end });
	env.InstanceManager = { new = function() return Magic; end };
	env.LeaderIcon = Magic;
	env.Mouse = Magic;
	env.PopupPriority = Magic;
	env.Keys = Magic;
	env.KeyEvents = Magic;
	assert(load(Strip(attente), "@WorldCongressBetweenTurns.lua", "t", env))();
	env.LuaEvents.WorldCongressPopup_ShowWorldCongressBetweenTurns.Fire(1);
	return env;
end
local att = NewAttente(1);
Check(att.bouton.hidden == false and att.bouton.text == "Revenir au vote", "attente : bouton « Revenir au vote » montré");
att.bouton.click();
Check(Ops(att) == "action:ACTION_UNREADYTURN", "attente : le bouton annule la fin de tour (" .. Ops(att) .. ")");
att.elapsed = true;
att.Events.TurnTimerUpdated.Fire(60, 60);
Check(att.bouton.hidden == true, "attente : caché quand le minuteur est écoulé");
att.elapsed = false;
att.canUnready = false;
att.Events.TurnTimerUpdated.Fire(10, 60);
Check(att.bouton.hidden == true, "attente : caché quand plus personne ne vote");
att.canUnready = true;
att.segment = Hash("TURNSEG_WORLDCONGRESS_RESOLUTION");
att.Events.TurnTimerUpdated.Fire(10, 60);
Check(att.bouton.hidden == true, "attente : caché hors du vote");
Check(NewAttente(0).bouton.hidden == true, "attente : caché sans le détour ou sans POL");

-- ---------------------------------------------------------------------------------------------
-- Fin de tour automatique (ui_congres_fin_auto.lua) et minuteur de MPH (ui_congres_minuteur.lua).
local finAuto = assert(load(Strip(ReadFile(dir .. "/../src/ui_congres_fin_auto.lua")), "@ActionPanel_Expansion2.lua", "t", (function()
	local env = NewEnv(1, 0);
	env.inSession = false;
	env.Game.GetWorldCongress = function() return { IsInSession = function() return env.inSession; end }; end
	env.XP1_AllowAutoEndTurn_Expansion = function() return true; end
	_G.finAutoEnv = env;
	return env;
end)()));
finAuto();
local fa = _G.finAutoEnv;
Check(fa.AllowAutoEndTurn_Expansion() == false, "fin auto : bloquée pendant le vote");
fa.segment = Hash("TURNSEG_SINGLEPHASE");
Check(fa.AllowAutoEndTurn_Expansion() == true, "fin auto : permise hors du congrès");
fa.inSession = true;
Check(fa.AllowAutoEndTurn_Expansion() == false, "fin auto : bloquée si le congrès est en session");

local function Minuteur(valeur, smart, segment)
	local env = NewEnv(1, 0);
	env.segment = Hash(segment);
	local conf = { TURN_TIMER_TIME = valeur, CPL_SMARTTIMER = smart };
	env.GameConfiguration = { IsNetworkMultiplayer = function() return true; end,
		GetValue = function(k) return conf[k]; end, SetValue = function(k, x) conf[k] = x; end };
	assert(load(Strip(ReadFile(dir .. "/../src/ui_congres_minuteur.lua")), "@TurnProcessing.lua", "t", env))();
	env.Events.LocalPlayerTurnBegin.Fire();
	return conf.TURN_TIMER_TIME;
end
Check(Minuteur(90, 0, "TURNSEG_WORLDCONGRESS_1") == 240, "minuteur : 90 s portées à 240 s pendant le vote");
Check(Minuteur(300, 0, "TURNSEG_WORLDCONGRESS_2") == 300, "minuteur : un minuteur plus long est gardé");
Check(Minuteur(90, 0, "TURNSEG_SINGLEPHASE") == 90, "minuteur : rien hors du vote");
Check(Minuteur(90, 1, "TURNSEG_WORLDCONGRESS_1") == 90, "minuteur : rien sans le minuteur intelligent de MPH");

os.exit(failures == 0 and 0 or 1);
