-- Test hors jeu de src/ui_diag_accords.lua :
--   lua proxy/tests/test_ui_diag_accords.lua [DiplomacyDealView.lua]
-- 1. Le vrai DiplomacyDealView.lua (Expansion2) suivi du bloc, comme `patch_diag_accords`, compile
--    (annotations HavokScript retirées), et `ms_LastIncomingDealProposalAction` y est un local de
--    premier niveau, donc visible du bloc.
-- 2. Le bloc seul, dans un environnement qui imite l'écran d'accord : il écrit les lignes attendues
--    sur les événements, à l'ouverture et à la fermeture, et garde OnShow / OnHide.
-- 3. Un bloc qui échoue n'interrompt pas le script hôte.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_diag_accords.lua");
local failures = 0;
local function Check(cond, what)
	if cond then
		print("ok    " .. what);
	else
		failures = failures + 1;
		print("ECHEC " .. what);
	end
end

-- 1. Le vrai fichier.
local VIEW_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\UI\Replacements\DiplomacyDealView.lua]];
local okRead, view = pcall(ReadFile, VIEW_PATH);
if okRead then
	for _, t in ipairs({ "table", "number", "string", "boolean", "object", "userdata", "ifunction", "function" }) do
		view = view:gsub("([%w_%)%]])%s*:%s*" .. t .. "%f[^%w_(]", "%1");
	end
	local chunk, err = load(view .. "\r\n" .. BLOCK, "@DiplomacyDealView.lua");
	Check(chunk ~= nil, "fichier du jeu suivi du bloc : compile " .. tostring(err or ""));
	Check(view:find("\nlocal ms_LastIncomingDealProposalAction%s*=") ~= nil,
		"ms_LastIncomingDealProposalAction est un local de premier niveau");
	Check(view:find("ContextPtr:SetShowHandler%( OnShow %);") ~= nil and view:find("\nInitialize%(%);%s*$") ~= nil,
		"OnShow posé par Initialize, appelé en fin de fichier, avant le bloc");
else
	print("absent, partie 1 sautée : " .. VIEW_PATH);
end

-- 2. Le bloc seul.
local function NewEvent()
	local e = { handlers = {} };
	e.Add = function(h) e.handlers[#e.handlers + 1] = h; end
	e.Fire = function(...) for _, h in ipairs(e.handlers) do h(...); end end
	return e;
end

local function NewEnv(printed)
	local env = setmetatable({}, { __index = _G });
	env.print = function(s) printed[#printed + 1] = s; end
	env.DealProposalAction = { PENDING = 0, ACCEPTED = 1, REJECTED = 2, ADJUSTED = 3, PROPOSED = 4, CLOSED = 8 };
	env.DealDirection = { OUTGOING = 0, INCOMING = 1 };
	env.Events = {
		DiplomacyIncomingDeal = NewEvent(), DiplomacyStatement = NewEvent(),
		DiplomacySessionClosed = NewEvent(), DiplomacyDealEnacted = NewEvent(),
	};
	env.Game = { GetLocalPlayer = function() return 1; end };
	local pending = { ["1>2"] = false, ["2>1"] = true };
	env.DealManager = {
		HasPendingDeal = function(a, b) return pending[a .. ">" .. b]; end,
		GetWorkingDeal = function(direction, a, b)
			return {
				GetItemCount = function(self, x, y) return direction == 1 and (x == 2 and 2 or 0) or 1; end,
				IsValid = function() return true; end,
			};
		end,
	};
	env.session = nil;
	env.DiplomacyManager = {
		FindOpenSessionID = function(a, b) return env.session; end,
		GetSessionInfo = function(id) return { FromPlayer = 1 }; end,
	};
	env.hidden = true;
	env.ContextPtr = {
		IsHidden = function() return env.hidden; end,
		SetShowHandler = function(self, f) env.show = f; end,
		SetHideHandler = function(self, f) env.hide = f; end,
	};
	env.Controls = { AcceptDeal = { IsHidden = function() return true; end } };
	env.ms_OtherPlayerID = -1;
	env.ms_InitiatedByPlayerID = -1;
	env.ms_LastIncomingDealProposalAction = 0;
	env.calls = {};
	env.OnShow = function()
		env.calls[#env.calls + 1] = "OnShow";
		env.hidden = false;
		env.ms_OtherPlayerID = 2;
		env.session = 7;
		env.ms_InitiatedByPlayerID = 1;
	end
	env.OnHide = function() env.calls[#env.calls + 1] = "OnHide"; end
	return env;
end

local printed = {};
local env = NewEnv(printed);
assert(load(BLOCK, "@ui_diag_accords.lua", "t", env))();
Check(printed[1] == "BGD accords diagnostic pose", "bloc posé");
-- Un second passage (rechargement du contexte) ne repose rien.
assert(load(BLOCK, "@ui_diag_accords.lua", "t", env))();
Check(#printed == 1 and #env.Events.DiplomacyIncomingDeal.handlers == 1, "un seul passage par contexte");

env.Events.DiplomacyIncomingDeal.Fire(2, 1, 4);
Check(printed[2] ~= nil and printed[2]:find("^BGD accords entrant 2>1 PROPOSED | session=nil de=%- attente%(1>2%)=false attente%(2>1%)=true entrant=0%+2 valide=true sortant=1%+1 valide=true$") ~= nil,
	"proposition reçue, écran fermé : " .. tostring(printed[2]));
env.Events.DiplomacyIncomingDeal.Fire(3, 4, 4);
Check(#printed == 2, "accord entre deux autres joueurs ignoré");

env.show();
Check(env.calls[1] == "OnShow", "OnShow appelé à l'ouverture");
Check(printed[3] ~= nil and printed[3]:find("^BGD accords ouverture | session=7 de=1 .* ecran: initiateur=1 derniere=PENDING accepter=cache$") ~= nil,
	"ouverture sur sa propre session : " .. tostring(printed[3]));

env.Events.DiplomacyStatement.Fire(1, 2, { StatementType = -2065048438, SessionID = 7, Nested = {} });
Check(printed[4] ~= nil and printed[4]:find("^BGD accords declaration 1>2 SessionID=7,StatementType=%-2065048438 | ") ~= nil,
	"déclaration : " .. tostring(printed[4]));

env.Events.DiplomacySessionClosed.Fire(7);
Check(printed[5] ~= nil and printed[5]:find("^BGD accords session fermee 7 | session=7") ~= nil, "session fermée : " .. tostring(printed[5]));

env.hide("x");
Check(env.calls[2] == "OnHide" and printed[6] ~= nil and printed[6]:find("^BGD accords fermeture | ") ~= nil, "fermeture puis OnHide");

env.Events.DiplomacyDealEnacted.Fire();
Check(printed[7] == "BGD accords accord execute ", "accord exécuté : " .. tostring(printed[7]));

-- 3. Un bloc qui échoue (pas d'Events) écrit l'échec et rend la main.
local printed2 = {};
local env2 = NewEnv(printed2);
env2.Events = nil;
local ok = pcall(assert(load(BLOCK, "@ui_diag_accords.lua", "t", env2)));
Check(ok and printed2[1] ~= nil and printed2[1]:find("^BGD accords echec ") ~= nil, "échec signalé sans erreur : " .. tostring(printed2[1]));

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
