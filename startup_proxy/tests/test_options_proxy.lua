-- Test hors jeu de src/options_proxy.lua :
--   lua startup_proxy/tests/test_options_proxy.lua [Options.lua]
-- On charge comme le proxy (`patch_options_lua`) : Options.lua du jeu, puis le bloc, dans un
-- environnement qui imite l'interface. Les deux fonctions natives écrivent dans une table au lieu
-- du fichier. On vérifie l'onglet, les listes, l'enregistrement à la confirmation et le message de
-- redémarrage du jeu.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/options_proxy.lua");
local OPTIONS_PATH = arg[1]
	or [[C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Assets\UI\Options.lua]];
-- HavokScript accepte des annotations de type (`local x:number`) que Lua 5.4 refuse : on les retire.
local OPTIONS = ReadFile(OPTIONS_PATH);
for _, t in ipairs({ "table", "number", "string", "boolean", "object" }) do
	OPTIONS = OPTIONS:gsub("%s*:%s*" .. t .. "%f[^%w_(]", "");
end

-- Tout ce que l'écran appelle sans qu'on s'y intéresse : chaque champ et chaque appel rendent le
-- même objet, sauf les index numériques (ipairs s'arrête) et les comparaisons numériques.
local Any;
Any = setmetatable({}, {
	__index = function(_, k) if type(k) == "number" then return nil; end return Any; end,
	__call = function() return Any; end,
	__lt = function() return false; end,
	__le = function() return false; end,
	__mul = function() return 0; end, __div = function() return 0; end,
	__add = function() return 0; end, __sub = function() return 0; end,
	__concat = function() return ""; end,
});

-- Un contrôle : garde sa visibilité, son texte, ses rappels et, pour une liste, ses entrées.
local function NewControl()
	local c = { hidden = false, text = nil, callbacks = {}, entries = {}, disabled = false };
	return setmetatable(c, { __index = function(_, k)
		if k == "SetHide" then return function(self, h) self.hidden = h; end; end
		if k == "IsHidden" then return function(self) return self.hidden; end; end
		if k == "SetText" or k == "LocalizeAndSetText" then return function(self, t) self.text = t; end; end
		if k == "SetToolTipString" then return function(self, t) self.tooltip = t; end; end
		if k == "SetDisabled" then return function(self, d) self.disabled = d; end; end
		if k == "RegisterCallback" then return function(self, ev, f) self.callbacks[ev] = f; end; end
		if k == "RegisterSelectionCallback" then return function(self, f) self.selection = f; end; end
		if k == "ClearEntries" then return function(self) self.entries = {}; end; end
		if k == "BuildEntry" then return function(self, _, inst)
			inst.Button = NewControl();
			table.insert(self.entries, inst);
		end; end
		if k == "GetButton" then return function(self)
			self.button = rawget(self, "button") or NewControl();
			return self.button;
		end; end
		if k == "IsSelected" then return function() return false; end; end
		if type(k) == "string" and k:match("^Get") then return function() return 0; end; end
		return function() return Any; end;
	end });
end

-- `loaded` : le proxy de GameCore a déjà lu le fichier (une partie a été lancée) ; `lang` : langue
-- du jeu (fr_FR par défaut) ; `diag` : proxy compilé avec la feature `diag` (builds locaux), qui
-- montre les réglages de diagnostic. Sans, l'onglet est celui de la release.
local function NewScreen(cfgText, installed, loaded, lang, diag)
	local m = { written = {}, popupTexts = {}, built = {}, loaded = loaded };
	local env = setmetatable({}, { __index = function(_, k) if _G[k] ~= nil then return _G[k]; end return Any; end });
	env.include = function() end;
	env.Controls = setmetatable({}, { __index = function(t, k) local c = NewControl(); rawset(t, k, c); return c; end });
	env.ContextPtr = setmetatable({
		SetShowHandler = function(_, f) m.show = f; end,
		SetInputHandler = function() end,
		BuildInstanceForControl = function(_, name, inst, parent)
			if name == "KeyBindingCategory" then
				inst.CategoryName = NewControl();
			else
				inst.StringName = NewControl();
				inst.PullDown = NewControl();
			end
			table.insert(m.built, { name = name, inst = inst, parent = parent });
		end,
	}, { __index = Any });
	env.Locale = setmetatable({
		Lookup = function(s) return s; end,
		ToUpper = function(s) return s; end,
		GetCurrentLanguage = function() return { Type = lang or "fr_FR" }; end,
	}, { __index = Any });
	env.Mouse = { eLClick = "LClick", eMouseEnter = "Enter" };
	env.PopupDialog = { new = function()
		return setmetatable({
			AddText = function(_, t) table.insert(m.popupTexts, t); end,
		}, { __index = function() return function() end; end });
	end };
	env.InstanceManager = { new = function() return Any; end };
	env.UIManager = setmetatable({ GetScreenSizeVal = function() return 1920, 1080; end }, { __index = Any });
	-- Les options du jeu lues par OnShow : des nombres ; les listes (adaptateurs…) : vides.
	env.Options = setmetatable({}, { __index = function(_, k)
		if k:match("^Get%a+Option$") then return function() return 0; end; end
		if k:match("^GetAvailable") then return function() return {}; end; end
		return Any;
	end });
	-- Le fichier : les lignes écrites s'ajoutent à la fin, la dernière d'une clé l'emporte.
	m.cfg = cfgText;
	env.Civ6Proxy_LireReglages = function() return m.cfg, installed, m.loaded; end;
	env.Civ6Proxy_EcrireReglages = function(lines)
		table.insert(m.written, lines);
		m.cfg = m.cfg .. "\n" .. lines .. "\n";
		return 1, m.loaded;
	end;
	-- IsInGame, Options, UserConfiguration… : Any.
	-- Comme `lua_block` (src/options.rs).
	local block = diag and BLOCK:gsub("local PROXY_DIAG = false;", "local PROXY_DIAG = true;", 1) or BLOCK;
	local f = assert(load(OPTIONS .. "\r\n" .. block, "Options.lua", "t", env));
	f();
	m.env = env;
	-- La ligne de réglage d'une clé : sa liste et son libellé.
	function m.Row(name)
		for _, b in ipairs(m.built) do
			if b.inst.StringName and b.inst.StringName.text == name then return b.inst; end
		end
	end
	-- Le message de redémarrage du jeu a-t-il été affiché ?
	function m.RestartShown()
		for _, t in ipairs(m.popupTexts) do
			if t == "LOC_OPTIONS_CHANGES_REQUIRE_APP_RESTART" then return true; end
		end
		return false;
	end
	-- Ce que fait le jeu quand on choisit l'entrée `label` d'une liste.
	function m.Choose(row, label)
		for i, e in ipairs(row.PullDown.entries) do
			if e.Button.text == label then
				row.PullDown.selection(i, 0, row.PullDown);
				return true;
			end
		end
		return false;
	end
	return m;
end

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

do
	-- Une partie a déjà été lancée : le proxy de GameCore a lu le fichier.
	local m = NewScreen("mode=coalesce\r\nui_boutons=on\r\nwmd_accel=7\r\n", 1, 1, nil, true);
	local C = m.env.Controls;
	Check("onglet visible", C.ProxyTab.hidden == false and C.ProxyTab.callbacks.LClick ~= nil);
	local rows, titles = 0, 0;
	local stacks = { [C.ProxyBasicStack] = true, [C.ProxyAdvancedStack] = true, [C.ProxyOptionsStack] = true };
	for _, b in ipairs(m.built) do
		if not stacks[b.parent] then rows = -100; end
		if b.name == "PullDownParameterInstance" then rows = rows + 1; else titles = titles + 1; end
	end
	Check("24 réglages et 7 titres (crédits compris), dans le panneau", rows == 24 and titles == 7);

	-- Options avancées : performances et diagnostic, cachées par défaut derrière un bouton.
	local function StackOf(name)
		for _, b in ipairs(m.built) do
			if (b.inst.StringName and b.inst.StringName.text == name)
				or (b.inst.CategoryName and b.inst.CategoryName.text == name) then return b.parent; end
		end
	end
	Check("avancées : cachées par défaut", C.ProxyAdvancedStack.hidden == true
		and C.ProxyAdvancedToggle.text == "Afficher les options avancées");
	Check("avancées : performances et diagnostic", StackOf("Performances du passage de tour") == C.ProxyAdvancedStack
		and StackOf("Cache des améliorations constructibles") == C.ProxyAdvancedStack
		and StackOf("Diagnostic") == C.ProxyAdvancedStack and StackOf("Journal des scripts et XML chargés") == C.ProxyAdvancedStack);
	Check("visibles : général, interface, correctifs, mods", StackOf("Proxys BetterGameDLLs") == C.ProxyBasicStack
		and StackOf("Recherche des mises à jour") == C.ProxyBasicStack
		and StackOf("Boutons d'attaque au-dessus des drapeaux d'unités") == C.ProxyBasicStack
		and StackOf("Merveille gardée dans la file après un échange") == C.ProxyBasicStack
		and StackOf("Écran des rapports amélioré") == C.ProxyBasicStack);
	Check("crédits : à la fin, hors des options avancées", StackOf("Crédits") == C.ProxyOptionsStack);
	C.ProxyAdvancedToggle.callbacks.LClick();
	Check("bouton : montre les avancées", C.ProxyAdvancedStack.hidden == false
		and C.ProxyAdvancedToggle.text == "Masquer les options avancées");
	m.show();
	Check("réouverture : état du bouton gardé", C.ProxyAdvancedStack.hidden == false);
	C.ProxyAdvancedToggle.callbacks.LClick();
	Check("bouton : cache à nouveau", C.ProxyAdvancedStack.hidden == true
		and C.ProxyAdvancedToggle.text == "Afficher les options avancées");
	Check("avertissement : redémarrage du jeu", (C.ProxyWarning.text or ""):find("redémarrage du jeu") ~= nil);
	Check("état : proxy installé", C.ProxyStatus.text == "Proxy de GameCore installé.");

	local boutons = m.Row("Boutons d'attaque au-dessus des drapeaux d'unités");
	Check("valeur du fichier affichée", boutons.PullDown.button.text == "Activé");
	local nuke = m.Row("Accélération de l'animation des armes nucléaires");
	Check("valeur hors liste montrée telle quelle", nuke.PullDown.button.text == "7 (valeur du fichier)");
	local amel = m.Row("Cache des améliorations constructibles");
	Check("clé absente : valeur par défaut", amel.PullDown.button.text == "Désactivé");

	-- Un changement, puis retour à la valeur d'origine : rien à écrire.
	C.ConfirmButton.disabled = true;
	Check("choix d'une valeur", m.Choose(boutons, "Désactivé"));
	Check("confirmer activé", C.ConfirmButton.disabled == false);
	m.Choose(boutons, "Activé");
	C.ConfirmButton.callbacks.LClick();
	Check("rien d'écrit sans changement réel", #m.written == 0 and not m.env._PromptRestartApp);

	-- Deux changements confirmés : écrits, et le message de redémarrage du jeu s'affiche.
	m.Choose(boutons, "Désactivé");
	m.Choose(amel, "Activé");
	C.ConfirmButton.callbacks.LClick();
	local w = m.written[1] or "";
	Check("écrit à la confirmation", #m.written == 1 and w:find("ui_boutons=off", 1, true) and w:find("ameliorations=quiet", 1, true));
	Check("redémarrage demandé", m.env._PromptRestartApp == true);
	Check("message de redémarrage du jeu", m.RestartShown());

	-- Réouverture : choix en attente abandonnés, fichier relu.
	m.Choose(boutons, "Activé");
	m.show();
	C.ConfirmButton.callbacks.LClick();
	Check("annulé par la réouverture", #m.written == 1);
end

do
	-- Onglet de la release (sans la feature `diag`) : ni les réglages de diagnostic ni leur effet,
	-- mais le départ simultané garde son réglage d'affichage des signaux.
	local m = NewScreen("ui_diag_accords=on\r\n", 1, 1);
	local rows = 0;
	for _, b in ipairs(m.built) do
		if b.name == "PullDownParameterInstance" then rows = rows + 1; end
	end
	Check("release : 20 réglages", rows == 20);
	Check("release : réglages de diagnostic absents", m.Row("Journal des scripts et XML chargés") == nil
		and m.Row("Mesure du coût des épingles") == nil and m.Row("Diagnostic des accords entre joueurs") == nil
		and m.Row("Profil des zones de l'exe") == nil);
	Check("release : signaux du départ simultané gardés", m.Row("Afficher les signaux du départ simultané") ~= nil);
end

do
	local m = NewScreen("", 0, 0);
	Check("état : proxy non installé", (m.env.Controls.ProxyStatus.text or ""):find("non installé") ~= nil);
end

do
	-- Aucune partie lancée depuis le démarrage : la prochaine lira le fichier, pas de redémarrage.
	local m = NewScreen("ui_boutons=on\r\n", 1, 0);
	local C = m.env.Controls;
	Check("avertissement : prochaine partie", (C.ProxyWarning.text or ""):find("prochaine partie") ~= nil);
	m.Choose(m.Row("Boutons d'attaque au-dessus des drapeaux d'unités"), "Désactivé");
	C.ConfirmButton.callbacks.LClick();
	Check("écrit sans partie lancée", m.written[1] == "ui_boutons=off");
	Check("pas de message de redémarrage", not m.env._PromptRestartApp and not m.RestartShown());
	-- Une partie lancée entre-temps : la réouverture met l'avertissement à jour.
	m.loaded = 1;
	m.show();
	Check("réouverture : avertissement de redémarrage", (C.ProxyWarning.text or ""):find("redémarrage du jeu") ~= nil);
end

do
	-- Réglage lu par le proxy de démarrage au lancement du jeu : redémarrage demandé même sans partie lancée.
	local m = NewScreen("ui_boutons=on\r\n", 1, 0, nil, true);
	local C = m.env.Controls;
	m.Choose(m.Row("Profil des zones de l'exe"), "Activé");
	C.ConfirmButton.callbacks.LClick();
	Check("profil_zones=on écrit", m.written[1] == "profil_zones=on");
	Check("profil_zones : redémarrage demandé sans partie lancée", m.env._PromptRestartApp == true);
end

do
	-- Jeu en anglais (comme toute autre langue que le français).
	for _, lang in ipairs({ "en_US", "de_DE" }) do
		local m = NewScreen("ui_boutons=on\r\nwmd_accel=7\r\n", 1, 0, lang);
		local C = m.env.Controls;
		local row = m.Row("Attack buttons above unit flags");
		Check(lang .. " : libellés en anglais", row ~= nil and row.PullDown.button.text == "Enabled"
			and (rawget(row.StringName, "tooltip") or ""):find("stay clickable", 1, true) ~= nil);
		Check(lang .. " : titres en anglais", m.built[1].inst.CategoryName.text == "General");
		Check(lang .. " : état et avertissement en anglais", C.ProxyStatus.text == "GameCore proxy installed."
			and (C.ProxyWarning.text or ""):find("next game starts", 1, true) ~= nil);
		local nuke = m.Row("Nuclear strike animation speed-up");
		Check(lang .. " : valeur hors liste", nuke.PullDown.button.text == "7 (value from file)");
	end
	local fr = NewScreen("", 1, 0, "fr_CA");
	Check("fr_CA : en français", fr.Row("Épingles de carte allégées") ~= nil);
end

do
	-- Interrupteur général.
	local m = NewScreen("ui_boutons=off\r\n", 1, 0);
	local C = m.env.Controls;
	local actif = m.Row("Proxys BetterGameDLLs");
	local boutons = m.Row("Boutons d'attaque au-dessus des drapeaux d'unités");
	Check("général : absent du fichier, activé", actif.PullDown.button.text == "Activé" and not actif.PullDown.disabled);
	Check("général : autres réglages modifiables", not boutons.PullDown.disabled);
	m.Choose(actif, "Désactivé");
	Check("général off : autres réglages grisés, valeur gardée", boutons.PullDown.disabled
		and boutons.PullDown.button.text == "Désactivé" and not actif.PullDown.disabled);
	C.ConfirmButton.callbacks.LClick();
	Check("général off : seul actif=off écrit", m.written[1] == "actif=off");
	m.show();
	Check("général off relu : grisés", boutons.PullDown.disabled);
	local maj = m.Row("Recherche des mises à jour");
	Check("général off : recherche des mises à jour jamais grisée", maj ~= nil and not maj.PullDown.disabled);
	m.Choose(maj, "Désactivé");
	C.ConfirmButton.callbacks.LClick();
	Check("verif_maj=off écrit", m.written[#m.written] == "verif_maj=off");
	m.Choose(actif, "Activé");
	Check("général on : modifiables à nouveau", not boutons.PullDown.disabled);
end

do
	-- Options.xml non modifié (remplacé par un mod) : le bloc ne fait rien.
	local env = setmetatable({}, { __index = function(_, k) if _G[k] ~= nil then return _G[k]; end return Any; end });
	env.Controls = {};
	local ok = pcall(assert(load(BLOCK, "bloc", "t", env)));
	Check("sans le XML : rien d'ajouté, pas d'erreur", ok);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
