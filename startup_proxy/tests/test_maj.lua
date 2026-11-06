-- Test hors jeu de src/maj.lua :
--   lua startup_proxy/tests/test_maj.lua
-- Un menu principal simulé (OnShow, OnUpdate, ContextPtr, g_PopupDialog) et une native
-- Civ6Proxy_MiseAJour pilotée par le test : vérification en cours, nouvelle version, à jour, popup
-- déjà ouvert, boutons, langue, erreur.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local f = assert(io.open(dir .. "/../src/maj.lua", "rb"));
local BLOCK = f:read("a");
f:close();

local failures = 0;
local function check(cond, what)
	if not cond then
		failures = failures + 1;
		print("ÉCHEC : " .. what);
	end
end

-- `etat` : "" (en cours), nil (rien), ou { "0.1.2", "0.1.1"[, "prerelease"] } ; `installeur` : 1 ou 0.
local function NewMenu(options)
	options = options or {};
	local m = { etat = options.etat, calls = {}, journal = {}, includes = {}, shown = 0, updates = 0, exited = false };
	local popup = { open = options.popupOuvert or false, buttons = {}, texts = {} };
	function popup:IsOpen() return self.open; end
	function popup:Reset() self.buttons = {}; self.texts = {}; self.title = nil; end
	function popup:AddTitle(t) self.title = t; end
	function popup:AddText(t) table.insert(self.texts, t); end
	function popup:AddButton(label, cb, cmd) table.insert(self.buttons, { label = label, cb = cb, cmd = cmd }); end
	function popup:Open() self.open = true; m.opened = (m.opened or 0) + 1; end
	function popup:ShowOkDialog(t) self:Reset(); self:AddText(t); self.open = true; m.ok = t; end
	function popup:Click(label)
		for _, b in ipairs(self.buttons) do
			if b.label == label then self.open = false; if b.cb then b.cb(); end return true; end
		end
		return false;
	end
	m.popup = popup;

	local env = {
		string = string, type = type, pcall = pcall, error = error, tostring = tostring, table = table, ipairs = ipairs,
		g_PopupDialog = popup,
		PopupDialog = { COMMAND_CONFIRM = "confirm", COMMAND_CANCEL = "cancel" },
		Locale = { GetCurrentLanguage = function() return { Type = options.lang or "fr_FR" }; end },
		-- UI.ExitGame ne ferme pas le jeu depuis le menu principal : le bloc ne doit pas s'en servir.
		UI = { ExitGame = function() m.exitGame = true; end },
		Events = { UserConfirmedClose = function() m.exited = true; end },
		Network = { GetFriends = function() return { ClearRichPresence = function() m.presence = true; end }; end },
		OnShow = function() m.shown = m.shown + 1; m.env.ContextPtr:SetUpdate(m.env.OnUpdate); end,
		OnUpdate = function() m.updates = m.updates + 1; end,
		Civ6Proxy_Journal = function(t) table.insert(m.journal, t); end,
		Controls = {},
	};
	-- MainMenu de MPH : ni g_PopupDialog, ni PopupDialog, ni OnUpdate ; le XML a l'instance du popup
	-- (ajoutée par le proxy), et include("PopupDialog") définit la classe.
	if options.mph then
		env.g_PopupDialog = nil;
		env.PopupDialog = nil;
		env.OnUpdate = nil;
		env.OnShow = function() m.shown = m.shown + 1; end
		if not options.sansInstance then env.Controls.PopupRoot = {}; end
		env.include = function(name)
			table.insert(m.includes, name);
			env.PopupDialog = { COMMAND_CONFIRM = "confirm", COMMAND_CANCEL = "cancel" };
			function env.PopupDialog:new(id) m.popupId = id; return popup; end
		end
	end
	if not options.sansProxy then
		env.Civ6Proxy_MiseAJour = function(action)
			table.insert(m.calls, action or "?");
			if options.erreur then error("boum"); end
			if action == "plus_tard" then m.etat = nil; return; end
			if action == "installer" then m.etat = nil; return options.installeur or 1; end
			if type(m.etat) == "table" then return m.etat[1], m.etat[2], m.etat[3] or "release"; end
			return m.etat;
		end
	end
	env.ContextPtr = {
		SetShowHandler = function(_, h) m.showHandler = h; end,
		SetUpdate = function(_, u) m.update = u; end,
	};
	m.env = env;
	-- Le menu du jeu pose OnShow comme gestionnaire, puis le bloc est exécuté à la suite.
	m.showHandler = env.OnShow;
	assert(load(BLOCK, "maj.lua", "t", env))();
	m.Show = function() m.showHandler(); end
	m.Frame = function() if m.update then m.update(0.016); end end
	return m;
end

do
	local m = NewMenu({ etat = "" });
	m.Show();
	check(m.shown == 1, "OnShow d'origine appelé");
	m.Frame();
	check(m.updates == 1, "OnUpdate d'origine appelé");
	check(not m.popup.open, "en cours : pas de popup");
	m.etat = { "0.1.2", "0.1.1" };
	m.Frame();
	check(m.popup.open and m.opened == 1, "nouvelle version : popup ouvert");
	check(m.popup.title == "BetterGameDLLs : nouvelle release", "titre : release");
	check(m.popup.texts[1]:find("La release 0.1.2", 1, true) and m.popup.texts[1]:find("installée : 0.1.1", 1, true), "texte en français avec les versions");
	check(not m.popup.texts[1]:find("pré-release", 1, true), "release : pas de mention de pré-release");
	check(#m.popup.buttons == 2 and m.popup.buttons[1].label == "Mettre à jour" and m.popup.buttons[2].label == "Plus tard", "deux boutons");
	check(m.popup.buttons[2].cmd == "cancel", "Plus tard sur Échap");
	m.Frame();
	check(m.opened == 1, "une seule fois");
	check(m.popup:Click("Mettre à jour"), "clic Mettre à jour");
	check(m.calls[#m.calls] == "installer" and m.exited and not m.exitGame, "installeur lancé, jeu fermé comme Quitter vers le bureau");
	check(m.presence, "présence Steam effacée");
end

do
	local m = NewMenu({ etat = { "0.1.2", "0.1.1" } });
	m.Show(); m.Frame();
	m.popup:Click("Plus tard");
	check(m.calls[#m.calls] == "plus_tard" and not m.exited, "Plus tard : jeu gardé");
	m.Show(); m.Frame();
	check(m.opened == 1, "Plus tard : pas de nouveau popup en revenant au menu");
end

do
	local m = NewMenu({ etat = { "0.1.2", "0.1.1" }, installeur = 0 });
	m.Show(); m.Frame();
	m.popup:Click("Mettre à jour");
	check(not m.exited and m.ok and m.ok:find("navigateur", 1, true), "sans installeur : page ouverte, message, jeu gardé");
end

do
	local m = NewMenu({ etat = { "0.1.2", "0.1.1" }, popupOuvert = true });
	m.Show(); m.Frame();
	check(m.opened == nil and #m.calls == 0, "autre popup ouvert : on attend");
	m.popup.open = false;
	m.Frame();
	check(m.opened == 1, "autre popup fermé : notification");
end

do
	local m = NewMenu({ etat = nil });
	m.Show(); m.Frame(); m.Frame();
	check(m.opened == nil and #m.calls == 1, "à jour : rien, une seule question");
end

do
	local m = NewMenu({ etat = { "0.1.3", "0.1.2", "prerelease" } });
	m.Show(); m.Frame();
	check(m.popup.title == "BetterGameDLLs : nouvelle pré-release", "titre : pré-release");
	check(m.popup.texts[1]:find("La pré-release 0.1.3", 1, true) and m.popup.texts[1]:find("case « Pré-release »", 1, true), "pré-release : dite et expliquée");
	check(m.journal[#m.journal]:find("pré-release 0.1.3", 1, true), "pré-release : au journal");
end

do
	local m = NewMenu({ etat = { "0.1.3", "0.1.2", "prerelease" }, lang = "en_US" });
	m.Show(); m.Frame();
	check(m.popup.title == "BetterGameDLLs: new pre-release" and m.popup.texts[1]:find("pre-release 0.1.3", 1, true), "anglais : pré-release");
end

do
	local m = NewMenu({ etat = { "0.1.2", "0.1.1" }, lang = "en_US" });
	m.Show(); m.Frame();
	check(m.popup.buttons[1].label == "Update" and m.popup.buttons[2].label == "Later", "anglais : boutons");
	check(m.popup.texts[1]:find("installed: 0.1.1", 1, true), "anglais : texte");
end

do
	local m = NewMenu({ etat = { "0.1.2", "0.1.1" }, erreur = true });
	m.Show(); m.Frame(); m.Frame();
	check(m.updates == 2 and m.journal[#m.journal]:find("erreur", 1, true), "erreur : au journal, menu intact");
	check(m.opened == nil, "erreur : pas de popup");
end

do
	local m = NewMenu({ sansProxy = true });
	check(m.showHandler == m.env.OnShow, "sans native : menu laissé tel quel");
end

do
	-- Menu de MPH : popup créé par le bloc.
	local m = NewMenu({ etat = "", mph = true });
	m.Show(); m.Frame();
	check(m.shown == 1 and m.updates == 0, "MPH : OnShow appelé, pas d'OnUpdate");
	check(m.opened == nil and #m.includes == 0, "MPH : en cours, rien de créé");
	m.etat = { "0.1.2", "0.1.1" };
	m.Frame();
	check(m.includes[1] == "PopupDialog" and m.popupId == "BGD_MajPopup", "MPH : PopupDialog chargé, popup créé");
	check(m.opened == 1 and #m.popup.buttons == 2, "MPH : notification ouverte");
	m.popup:Click("Mettre à jour");
	check(m.exited, "MPH : installeur lancé, jeu fermé");
end

do
	-- Menu sans popup ni instance : erreur au journal, pas d'attente silencieuse.
	local m = NewMenu({ etat = { "0.1.2", "0.1.1" }, mph = true, sansInstance = true });
	m.Show(); m.Frame();
	check(m.opened == nil and m.journal[#m.journal]:find("pas de popup", 1, true), "sans instance : erreur au journal");
end

if failures == 0 then
	print("maj.lua : tous les tests passent");
else
	print(failures .. " échec(s)");
	os.exit(1);
end
