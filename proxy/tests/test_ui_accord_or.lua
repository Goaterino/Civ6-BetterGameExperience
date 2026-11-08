-- Test hors jeu de src/ui_accord_or.lua :
--   lua proxy/tests/test_ui_accord_or.lua [dossier du jeu]
-- 1. Le vrai DiplomacyDealView.lua (Expansion2) suivi du bloc compile (annotations HavokScript
--    retirées), et PopulateAvailableGold y est globale.
-- 2. Avec le vrai InstanceManager.lua : deux ouvertures de l'écran avec des dispositions
--    différentes. Sans le bloc, l'or de l'autre joueur reprend un bouton désactivé par une
--    ressource stratégique (le défaut du jeu) ; avec le bloc, il est réactivé, et les ressources
--    désactivées le restent.
-- 3. Le bloc enveloppe aussi un PopulateAvailableGold déjà remplacé (MPH), et un échec ne
--    remonte pas.

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
local BLOCK = ReadFile(dir .. "/../src/ui_accord_or.lua");
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
	Check(view:find("\nfunction PopulateAvailableGold%(") ~= nil, "PopulateAvailableGold est globale");
	Check(view:find("\ng_IconOnlyIM%s*=") ~= nil and view:find("\ng_IconAndTextIM%s*=") ~= nil,
		"g_IconOnlyIM et g_IconAndTextIM sont globales");
else
	print("absent, partie 1 sautée");
end

-- 2. Le vrai InstanceManager.
local okIM, imText = pcall(ReadFile, GAME .. [[\Base\Assets\UI\InstanceManager.lua]]);
if not okIM then
	print("InstanceManager.lua absent, parties 2 et 3 sautées");
	os.exit(failures == 0 and 0 or 1);
end

local function NewControl()
	local c = { disabled = false, hidden = false };
	function c:SetHide(b) self.hidden = b; end
	function c:ChangeParent(p) self.parent = p; end
	function c:SetDisabled(b) self.disabled = b; end
	function c:IsDisabled() return self.disabled; end
	return c;
end

local function NewEnv()
	local env = setmetatable({}, { __index = _G });
	env.print = function() end;
	env.ContextPtr = {
		BuildInstanceForControl = function(_, _, controlTable, _)
			controlTable.SelectButton = NewControl();
		end,
	};
	assert(load(StripTypes(imText), "@InstanceManager.lua", "t", env))();
	env.g_IconOnlyIM = env.InstanceManager:new("IconOnly", "SelectButton", NewControl());
	env.g_IconAndTextIM = env.InstanceManager:new("IconAndText", "SelectButton", NewControl());
	-- Comme le jeu : l'or ponctuel et l'or par tour ne touchent pas à SetDisabled.
	env.PopulateAvailableGold = function(player, iconList)
		local oneTime = env.g_IconOnlyIM:GetInstance(iconList);
		local perTurn = env.g_IconAndTextIM:GetInstance(iconList);
		player.gold = { oneTime.SelectButton, perTurn.SelectButton };
		return 2;
	end
	-- Comme PopulateAvailableResources : désactive les ressources sans place sous le plafond.
	env.PopulateStrategic = function(player, n)
		player.strategic = {};
		for i = 1, n do
			local icon = env.g_IconOnlyIM:GetInstance(nil);
			icon.SelectButton:SetDisabled(true);
			player.strategic[i] = icon.SelectButton;
		end
	end
	-- Une ouverture de l'écran (OnShow) : réserves rendues, puis les deux colonnes.
	env.Open = function(myStrategic, theirStrategic)
		env.g_IconOnlyIM:ResetInstances();
		env.g_IconAndTextIM:ResetInstances();
		local me, them = {}, {};
		env.PopulateAvailableGold(me, "mine");
		env.PopulateStrategic(me, myStrategic);
		env.PopulateAvailableGold(them, "theirs");
		env.PopulateStrategic(them, theirStrategic);
		return me, them;
	end
	return env;
end

local function Disabled(buttons)
	for _, b in ipairs(buttons) do
		if b:IsDisabled() then return true; end
	end
	return false;
end

-- Sans le bloc : le défaut se reproduit.
local raw = NewEnv();
raw.Open(2, 0);
local _, them = raw.Open(0, 0);
Check(them.gold[1]:IsDisabled(), "sans le bloc : l'or ponctuel de l'autre reprend un bouton désactivé");

-- Avec le bloc.
local env = NewEnv();
assert(load(BLOCK, "@ui_accord_or.lua", "t", env))();
env.Open(2, 0);
local me1, them1 = env.Open(0, 0);
Check(not Disabled(me1.gold) and not Disabled(them1.gold), "avec le bloc : or réactivé après changement de disposition");
local me2, them2 = env.Open(1, 3);
Check(not Disabled(me2.gold) and not Disabled(them2.gold), "avec le bloc : or actif quand les deux ont des ressources");
local all = true;
for _, b in ipairs(me2.strategic) do all = all and b:IsDisabled(); end
for _, b in ipairs(them2.strategic) do all = all and b:IsDisabled(); end
Check(all, "les ressources désactivées le restent");

-- Bloc chargé deux fois : une seule enveloppe.
local wrapped = env.PopulateAvailableGold;
assert(load(BLOCK, "@ui_accord_or.lua", "t", env))();
Check(env.PopulateAvailableGold == wrapped, "pas deux enveloppes");

-- 3. MPH : PopulateAvailableGold déjà remplacée, qui rappelle celle du jeu.
local mph = NewEnv();
local base = mph.PopulateAvailableGold;
local allowed = true;
mph.PopulateAvailableGold = function(player, iconList)
	if not allowed then return 0; end
	return base(player, iconList);
end
assert(load(BLOCK, "@ui_accord_or.lua", "t", mph))();
mph.Open(3, 0);
local _, themMph = mph.Open(0, 0);
Check(not Disabled(themMph.gold), "MPH : or réactivé");
allowed = false;
Check(mph.PopulateAvailableGold({}, "x") == 0, "MPH : or interdit, 0 rendu sans erreur");

-- Réserve sans m_AllocatedInstances : rien ne remonte, la valeur passe.
local odd = NewEnv();
odd.g_IconOnlyIM = {};
odd.g_IconAndTextIM = nil;
odd.PopulateAvailableGold = function() return 5; end
assert(load(BLOCK, "@ui_accord_or.lua", "t", odd))();
local okOdd, n = pcall(odd.PopulateAvailableGold, {}, "x");
Check(okOdd and n == 5, "réserves inattendues : pas d'erreur, valeur rendue");

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
