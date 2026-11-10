-- Test hors jeu de src/ui_avancement.lua :
--   lua proxy/tests/test_ui_avancement.lua
-- Un WorldTracker.lua réduit (ses locales m_researchInstance, m_currentResearchID… et les deux
-- fonctions de mise à jour), suivi du bloc dans le même texte, comme le proxy le compile. On vérifie
-- le « x/y » de la technologie et du dogme en cours, le nom raccourci, et les cas sans affichage.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../src/ui_avancement.lua");

local LOCAL = 0;
local CAR = 6; -- largeur d'un caractère visible, en pixels

local function Visible(t)
	return (t:gsub("%[[^%]]*%]", ""));
end

local function Label()
	local c = { text = "", hidden = false };
	function c:SetText(t) self.text = t or ""; end
	function c:GetText() return self.text; end
	function c:SetHide(h) self.hidden = h; end
	function c:GetSizeX() return #Visible(self.text) * CAR; end
	return c;
end

local function Instance(avecLabel)
	local inst = { MainPanel = { GetSizeX = function() return 296; end } };
	local titre = Label();
	titre.truncate = nil;
	titre.tooltip = nil;
	local texte = { SetTruncateWidth = function(_, w) titre.truncate = w; end };
	function titre:GetTextControl() return texte; end
	function titre:GetOffsetX() return 65; end
	function titre:SetToolTipString(s) self.tooltip = s; end
	inst.TitleButton = titre;
	if avecLabel then
		inst.BGD_Avancement = Label();
	end
	return inst;
end

local function NewMachine(opts)
	opts = opts or {};
	local m = {
		player = LOCAL, research = 1, progress = 37.6, cost = 120,
		civic = 0, cprogress = 55, ccost = 80, science = 15.25, culture = 7, appelsBase = 0, prints = {},
	};
	local env = setmetatable({}, { __index = _G });
	env.print = function(s) table.insert(m.prints, s); end
	env.PlayerTypes = { NONE = -1, OBSERVER = 63 };
	env.Game = { GetLocalPlayer = function() return m.player; end };
	m.config = {};
	env.GameConfiguration = { GetValue = function(k) return m.config[k]; end };
	env.Locale = { Lookup = function(s) return s; end, ToUpper = function(s) return string.upper(s); end };
	env.GameInfo = {
		Technologies = { [0] = { Name = "Pottery" }, [1] = { Name = "Scientific theory and much more stuff" } },
		Civics = { [0] = { Name = "Drama" } },
	};
	env.Players = { [LOCAL] = {
		GetTechs = function() return {
			GetResearchingTech = function() return m.research; end,
			GetResearchProgress = function(_, i) assert(i == m.research); if m.casse then error("boum"); end return m.progress; end,
			GetResearchCost = function(_, i) return m.cost; end,
		}; end,
		GetCulture = function() return {
			GetProgressingCivic = function() return m.civic; end,
			GetCulturalProgress = function(_, i) return m.cprogress; end,
			GetCultureCost = function(_, i) return m.ccost; end,
		}; end,
	} };
	if not opts.sansFonction then
		env.BGD_Excedent = function(j, quoi)
			assert(j == LOCAL);
			if m.sansExcedent then return nil; end
			if m.excedentCasse then error("boum"); end
			return quoi == 0 and m.science or m.culture;
		end
	end
	env.TruncateStringWithTooltip = function(control, w, s)
		control:GetTextControl():SetTruncateWidth(w);
		control:SetText(s);
		local coupe = #s * CAR > w;
		control:SetToolTipString(coupe and s or nil);
		return coupe;
	end
	env.BGD_M = m;
	local prefixe = [[
local m_researchInstance = BGD_Inst_R;
local m_civicsInstance = BGD_Inst_C;
local m_currentResearchID = BGD_M.suivi;
local m_currentCivicID = BGD_M.suiviDogme;
function UpdateResearchPanel(isHide)
	BGD_M.appelsBase = BGD_M.appelsBase + 1;
	m_researchInstance.TitleButton:SetText("BASE");
end
function UpdateCivicsPanel(hide)
	BGD_M.appelsBase = BGD_M.appelsBase + 1;
	m_civicsInstance.TitleButton:SetText("BASE");
end
function BGD_Suivre(r, c) m_currentResearchID = r; m_currentCivicID = c; end
]];
	m.suivi = opts.suivi == nil and 1 or opts.suivi;
	m.suiviDogme = opts.suiviDogme == nil and 0 or opts.suiviDogme;
	m.R = Instance(opts.label ~= false);
	m.C = Instance(opts.label ~= false);
	env.BGD_Inst_R = m.R;
	env.BGD_Inst_C = m.C;
	local chunk = assert(load(prefixe .. "\r\n" .. BLOCK, "WorldTracker.lua", "t", env));
	chunk();
	m.env = env;
	return m;
end

local failures = 0;
local function check(name, cond)
	if cond then
		print("ok   " .. name);
	else
		failures = failures + 1;
		print("FAIL " .. name);
	end
end

-- 0. Option de partie « Disable Advanced UI indicators (BGE) » : lue à chaque redessin, d'une
--    partie à l'autre sans redémarrer le jeu.
do
	local m = NewMachine();
	m.config.BGD_INDICATEURS_OFF = true;
	m.env.UpdateResearchPanel();
	m.env.UpdateCivicsPanel();
	check("0 option cochée : rien en recherche", m.R.BGD_Avancement.hidden == true and m.R.BGD_Avancement.text == "");
	check("0 option cochée : rien en dogme", m.C.BGD_Avancement.hidden == true and m.C.BGD_Avancement.text == "");
	check("0 option cochée : panneau du jeu dessiné", m.appelsBase == 2);
	m.config.BGD_INDICATEURS_OFF = false;
	m.env.UpdateResearchPanel();
	check("0 option décochée ensuite : x/y de retour", m.R.BGD_Avancement.hidden == false
		and m.R.BGD_Avancement.text == "[COLOR:ResScienceLabelCS]37/120 (+15.3)[ENDCOLOR]");
	m.config.BGD_INDICATEURS_OFF = 1;
	m.env.UpdateResearchPanel();
	check("0 option cochée ensuite (valeur 1) : x/y retiré, nom en entier", m.R.BGD_Avancement.hidden == true
		and m.R.TitleButton.truncate == 225);
end

-- 1. Technologie en cours : « 37/120 » en couleur de science, nom raccourci à la place restante.
do
	local m = NewMachine();
	m.env.UpdateResearchPanel();
	local l = m.R.BGD_Avancement;
	check("1 base appelée", m.appelsBase == 1);
	check("1 texte", l.text == "[COLOR:ResScienceLabelCS]37/120 (+15.3)[ENDCOLOR]");
	check("1 visible", l.hidden == false);
	local place = 296 - 0 - #"37/120 (+15.3)" * CAR - 8 - 65;
	check("1 largeur du nom", m.R.TitleButton.truncate == place);
	check("1 nom complet", m.R.TitleButton.text == "SCIENTIFIC THEORY AND MUCH MORE STUFF");
	check("1 infobulle du nom raccourci", m.R.TitleButton.tooltip == "SCIENTIFIC THEORY AND MUCH MORE STUFF");
end

-- 2. Dogme en cours : couleur de culture ; nom court, pas d'infobulle.
do
	local m = NewMachine();
	m.env.UpdateCivicsPanel();
	local l = m.C.BGD_Avancement;
	check("2 texte", l.text == "[COLOR:ResCultureLabelCS]55/80 (+7)[ENDCOLOR]");
	check("2 visible", l.hidden == false);
	check("2 nom", m.C.TitleButton.text == "DRAMA" and m.C.TitleButton.tooltip == nil);
end

-- 3. Rien en cours (le panneau montre la dernière terminée) : caché, largeur du nom rendue.
do
	local m = NewMachine();
	m.env.UpdateResearchPanel();
	m.env.BGD_Suivre(-1, -1);
	m.env.UpdateResearchPanel();
	m.env.UpdateCivicsPanel();
	check("3 recherche cachée", m.R.BGD_Avancement.hidden == true and m.R.BGD_Avancement.text == "");
	check("3 dogme caché", m.C.BGD_Avancement.hidden == true);
	check("3 nom rendu par le jeu, pleine largeur", m.R.TitleButton.text == "BASE" and m.R.TitleButton.truncate == 225);
end

-- 4. Suivi du panneau illisible (nil) : ID du jeu.
do
	local m = NewMachine({ suivi = false });
	m.env.BGD_Suivre(nil, nil);
	m.research = 0;
	m.env.UpdateResearchPanel();
	check("4 ID du jeu", m.R.TitleButton.text == "POTTERY");
end

-- 5. Observateur : caché.
do
	local m = NewMachine();
	m.player = 63;
	m.env.UpdateResearchPanel();
	check("5 observateur", m.R.BGD_Avancement.hidden == true);
end

-- 6. Sans le libellé (XML non modifié) : rien, pas d'erreur, le jeu fait son travail.
do
	local m = NewMachine({ label = false });
	m.env.UpdateResearchPanel();
	m.env.UpdateCivicsPanel();
	check("6 base appelée", m.appelsBase == 2);
	check("6 nom intact", m.R.TitleButton.text == "BASE" and m.R.TitleButton.truncate == nil);
	check("6 pas d'erreur", #m.prints == 0);
end

-- 7. Erreur du jeu : journal, le panneau reste dessiné par le jeu.
do
	local m = NewMachine();
	m.casse = true;
	m.env.UpdateResearchPanel();
	check("7 base appelée", m.appelsBase == 1);
	check("7 journal", m.prints[1] ~= nil and m.prints[1]:find("^Avancement echec") ~= nil);
end

-- 8. Panneau XP1/XP2 qui enveloppe à son tour UpdateResearchPanel : le compteur suit.
do
	local m = NewMachine();
	local env = m.env;
	local base = env.UpdateResearchPanel;
	env.UpdateResearchPanel = function(h) base(h); end
	m.progress = 119.99;
	m.sansExcedent = true;
	env.UpdateResearchPanel();
	check("8 enveloppe XP1, excédent illisible omis", m.R.BGD_Avancement.text == "[COLOR:ResScienceLabelCS]119/120[ENDCOLOR]");
end

-- 9. Bloc ajouté deux fois : une seule enveloppe.
do
	local m = NewMachine();
	assert(load(BLOCK, "x", "t", m.env))();
	m.env.UpdateResearchPanel();
	check("9 une seule enveloppe", m.appelsBase == 1);
end

-- 10. Excédent nul ou presque : pas de parenthèses ; fonction absente ou en erreur : non plus.
do
	local m = NewMachine();
	m.science = 0.03;
	m.env.UpdateResearchPanel();
	check("10 excédent nul", m.R.BGD_Avancement.text == "[COLOR:ResScienceLabelCS]37/120[ENDCOLOR]");
	m.excedentCasse = true;
	m.env.UpdateResearchPanel();
	check("10 excédent en erreur", m.R.BGD_Avancement.text == "[COLOR:ResScienceLabelCS]37/120[ENDCOLOR]" and #m.prints == 0);
	local m2 = NewMachine({ sansFonction = true });
	m2.env.UpdateCivicsPanel();
	check("10 sans BGD_Excedent", m2.C.BGD_Avancement.text == "[COLOR:ResCultureLabelCS]55/80[ENDCOLOR]");
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout passe");
