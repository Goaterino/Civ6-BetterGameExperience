-- Test hors jeu de src/ui_accord_fin.lua :
--   lua proxy/tests/test_ui_accord_fin.lua [dossier du jeu]
-- 1. Le vrai DiplomacyDealView.lua (Expansion2) suivi des textes et du bloc compile (annotations
--    HavokScript retirées) et appelle bien DealManager.SendWorkingDeal.
-- 2. Dans un environnement simulé : un accord vers un humain part en cours de tour, ne part pas
--    dans les MARGE_S dernières secondes, minuteur écoulé ou passage de tour commencé, et le joueur
--    est prévenu. Vers une IA, ou hors multijoueur en réseau, tout part. Le reste de DealManager
--    est celui du jeu.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local function StripTypes(text)
	for _, t in ipairs({ "table", "number", "string", "boolean", "object", "userdata", "ifunction", "function" }) do
		text = text:gsub("([%w_%)%]])%s*:%s*" .. t .. "%f[^%w_(]", "%1");
	end
	return text;
end
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. ReadFile(dir .. "/../../textes/bgd_texte.lua")
	.. ReadFile(dir .. "/../src/ui_accord_fin.lua");
local failures = 0;
local function Check(cond, what)
	if cond then
		print("ok    " .. what);
	else
		failures = failures + 1;
		print("ECHEC " .. what);
	end
end

local GAME = arg[1] or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI]];

-- 1. Le vrai fichier.
local okRead, view = pcall(ReadFile, GAME .. [[\DLC\Expansion2\UI\Replacements\DiplomacyDealView.lua]]);
if okRead then
	view = StripTypes(view);
	local chunk, err = load(view .. "\r\n" .. BLOCK, "@DiplomacyDealView.lua");
	Check(chunk ~= nil, "fichier du jeu suivi du bloc : compile " .. tostring(err or ""));
	Check(view:find("DealManager.SendWorkingDeal(", 1, true) ~= nil, "le fichier envoie par DealManager.SendWorkingDeal");
else
	print("DiplomacyDealView.lua absent, partie 1 sautée");
end

-- 2. Environnement simulé.
local function NewEvent()
	local e = { fns = {} };
	function e.Add(f) table.insert(e.fns, f); end
	function e.Fire(...) for _, f in ipairs(e.fns) do f(...); end end
	return e;
end

local function NewEnv(opts)
	local env = setmetatable({}, { __index = _G });
	env.print = function() end;
	env.clock = 100;
	env.os = { clock = function() return env.clock; end };
	env.sent, env.lines = {}, {};
	local jeu = {
		SendWorkingDeal = function(a, de, vers) table.insert(env.sent, { a, de, vers }); end,
		GetWorkingDeal = function() return "deal"; end,
	};
	env.jeu = jeu;
	env.DealManager = jeu;
	env.Events = { TurnTimerUpdated = NewEvent(), LocalPlayerTurnBegin = NewEvent() };
	env.LuaEvents = { BGD_DepartTexte = function(t) table.insert(env.lines, t); end };
	env.Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end };
	env.elapsed = false;
	env.UI = { IsTurnTimerElapsed = function() return env.elapsed; end };
	env.reseau = opts.reseau ~= false;
	env.GameConfiguration = {
		IsNetworkMultiplayer = function() return env.reseau; end,
		IsPlayByCloud = function() return false; end,
		IsHotseat = function() return false; end,
		GetMultiplayerPlayerIDs = function() return { 0, 1, 2 }; end,
	};
	-- 0 et 1 humains, 2 une IA.
	env.actifs = { [0] = true, [1] = true, [2] = true };
	env.PlayerConfigurations = {};
	env.Players = {};
	for id = 0, 2 do
		env.PlayerConfigurations[id] = { IsHuman = function() return id ~= 2; end };
		env.Players[id] = { IsAlive = function() return true; end, IsTurnActive = function() return env.actifs[id]; end };
	end
	local chunk = assert(load(BLOCK, "@bloc", "t", env));
	chunk();
	return env;
end

local env = NewEnv({});
Check(env.DealManager ~= env.jeu and env.DealManager.GetWorkingDeal() == "deal", "DealManager du jeu enveloppé, le reste inchangé");
env.Events.TurnTimerUpdated.Fire(10, 60);
env.DealManager.SendWorkingDeal(1, 0, 1);
Check(#env.sent == 1, "en cours de tour : l'accord part");
env.Events.TurnTimerUpdated.Fire(58, 60);
env.DealManager.SendWorkingDeal(1, 0, 1);
Check(#env.sent == 1 and #env.lines == 1, "2 s restantes : retenu, joueur prévenu");
Check(env.lines[1]:find("accord non envoyé", 1, true) ~= nil, "message dans la langue du jeu");
env.DealManager.SendWorkingDeal(1, 0, 2);
Check(#env.sent == 2, "vers une IA : part quand même");
env.clock = 110;
env.DealManager.SendWorkingDeal(2, 0, 1);
Check(#env.sent == 3, "valeur du minuteur périmée : ignorée");
env.elapsed = true;
env.DealManager.SendWorkingDeal(2, 0, 1);
Check(#env.sent == 3, "minuteur écoulé : retenu");
env.elapsed = false;
env.actifs[0], env.actifs[1] = false, false;
env.DealManager.SendWorkingDeal(2, 0, 1);
Check(#env.sent == 3, "plus aucun humain actif (passage de tour) : retenu");
env.actifs[1] = true;
env.DealManager.SendWorkingDeal(2, 0, 1);
Check(#env.sent == 4, "l'autre joue encore, sans minuteur proche : part");
env.Events.TurnTimerUpdated.Fire(59, 60);
env.Events.LocalPlayerTurnBegin.Fire();
env.DealManager.SendWorkingDeal(1, 0, 1);
Check(#env.sent == 5, "début du tour suivant : minuteur oublié, l'accord part");

local solo = NewEnv({ reseau = false });
solo.Events.TurnTimerUpdated.Fire(59, 60);
solo.DealManager.SendWorkingDeal(1, 0, 1);
Check(#solo.sent == 1, "hors multijoueur en réseau : rien ne change");

-- Pas deux fois dans un même contexte.
local w = env.DealManager;
assert(load(BLOCK, "@bloc", "t", env))();
Check(env.DealManager == w, "bloc chargé deux fois : une seule enveloppe");

print(failures == 0 and "tout est bon" or (failures .. " échec(s)"));
os.exit(failures == 0 and 0 or 1);
