-- Test hors jeu du chargement de Real Modifier Analysis pour les cartes de politique
-- (src/mods/politiques_entete.lua, politiques_carte.lua, commun.lua) :
--   lua proxy/tests/test_mods_politiques.lua
-- Comme dans le jeu, une fonction garde les globales du contexte où elle a été compilée, et
-- Civ6Proxy_Charger compile dans celles du contexte qui tourne (le vrai luaL_loadbuffer). L'écran
-- du gouvernement peut dessiner ses cartes pendant qu'un autre contexte tourne (InGamePopup, essai
-- du 18 novembre 2026) : RMA compilé à ce moment-là s'installait dans l'autre contexte, et la carte
-- restait sans effet jusqu'au rechargement de la partie.
-- Le vrai RMA est remplacé par un RMA réduit qui fait comme lui : sa table vient de
-- BGD_RMA_Table, son texte de BGD_Lookup.

local dir = arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local ENTETE = ReadFile(dir .. "/../src/mods/politiques_entete.lua");
local CARTE = ReadFile(dir .. "/../src/mods/politiques_carte.lua");
local EMBARQUES = {
	BGD_Commun = ReadFile(dir .. "/../src/mods/commun.lua"),
	BGD_Textes = "BGD_TEXTES = {};",
	RealModifierAnalysis = [[
		local RMA = BGD_RMA_Table or {};
		function RMA_CalculateModifierEffect(kind, policy) return BGD_Lookup("effet de " .. policy); end
		RMA.CalculateModifierEffect = RMA_CalculateModifierEffect;
	]],
};
-- Rappel : la vraie tête de RMA lit bien BGD_RMA_Table, et ses fonctions BGD_Lookup.
local RMA_VRAI = ReadFile(dir .. "/../src/mods/rapports/RealModifierAnalysis.lua");

local failures = 0;
local function Check(name, ok)
	print((ok and "ok      " or "ECHEC   ") .. name);
	if not ok then failures = failures + 1; end
end

Check("RMA réel : sa table vient de BGD_RMA_Table", RMA_VRAI:find("local RMA = BGD_RMA_Table or {};", 1, true) ~= nil);
Check("RMA réel : textes par BGD_Lookup", RMA_VRAI:find("BGD_Lookup(", 1, true) ~= nil);

-- Un contexte : ses globales, sa sortie (préfixe du Lua.log). Le contexte courant est celui qui
-- tourne côté jeu.
local courant;
local compilations = 0;
local function Contexte(nom)
	local c = { nom = nom, sortie = {} };
	local env = setmetatable({}, { __index = _G });
	env.print = function(s) c.sortie[#c.sortie + 1] = tostring(s); end;
	env.Locale = { GetCurrentLanguage = function() return { Type = "fr_FR" }; end, Lookup = function(s) return s; end };
	env.Civ6Proxy_Charger = function(nom)
		local texte = EMBARQUES[nom];
		if texte == nil then return "fichier embarqué inconnu : " .. tostring(nom); end
		if nom == "RealModifierAnalysis" then compilations = compilations + 1; end
		local f, err = load(texte, "@BetterGameDLLs\\" .. nom .. ".lua", "t", courant.env);
		return f or err;
	end;
	c.env = env;
	return c;
end

-- Le script de l'écran : l'en-tête, puis la fonction qui remplit une carte, compilée avec lui.
local function Control()
	local c = { hidden = nil, text = nil, tip = nil };
	function c:SetHide(v) self.hidden = v; end
	function c:SetText(v) self.text = v; end
	function c:SetToolTipString(v) self.tip = v; end
	return c;
end
local function Ecran(avecEntete)
	local gouv = Contexte("GovernmentScreen");
	courant = gouv;
	if avecEntete then
		assert(load(ENTETE, "GovernmentScreen.lua", "t", gouv.env))();
	else
		-- Ancien en-tête : BGD_Commun seul, RMA à la première carte.
		gouv.env.Civ6Proxy_Charger("BGD_Commun")();
	end
	gouv.Carte = assert(load("return function(cardInstance, policyType, cardName, policy)\n" .. CARTE .. "\nend",
		"GovernmentScreen.lua", "t", gouv.env))();
	return gouv;
end
local function Dessiner(gouv)
	local carte = { Draggable = Control(), EffectContainer = Control(), Effect = Control() };
	gouv.Carte(carte, "POLICY_X", "Carte", { Description = "Description" });
	return carte;
end

-- Le cas du 18 novembre 2026 : la première carte est dessinée pendant qu'InGamePopup tourne.
do
	compilations = 0;
	local gouv = Ecran(true);
	local popup = Contexte("InGamePopup");
	courant = popup;
	local carte = Dessiner(gouv);
	Check("autre contexte à la première carte : effet affiché", carte.Effect.text == "effet de POLICY_X"
		and carte.EffectContainer.hidden == false);
	Check("autre contexte : effet dans l'infobulle", carte.Draggable.tip == "Carte[NEWLINE][NEWLINE]Description[NEWLINE][NEWLINE]effet de POLICY_X");
	Check("autre contexte : RMA dans celui de l'écran, rien chez l'autre",
		rawget(gouv.env, "RMA_CalculateModifierEffect") ~= nil and rawget(popup.env, "RMA_CalculateModifierEffect") == nil);
	Check("RMA compilé une fois, à la lecture du fichier", compilations == 1);
	Check("aucune trace", #gouv.sortie == 0 and #popup.sortie == 0);
	courant = gouv;
	Dessiner(gouv);
	Check("cartes suivantes : pas de nouvelle compilation", compilations == 1);
end

-- Sans le chargement en tête (ancien en-tête) : la table vide n'est plus gardée, la carte suivante
-- dessinée dans le bon contexte retrouve son effet.
do
	compilations = 0;
	local gouv = Ecran(false);
	local popup = Contexte("InGamePopup");
	courant = popup;
	local carte = Dessiner(gouv);
	-- (Pas de trace : BGD_PolTrace est défini par l'en-tête.)
	Check("ancien en-tête, autre contexte : pas d'effet", carte.Effect.text == "" and carte.EffectContainer.hidden == true);
	courant = gouv;
	carte = Dessiner(gouv);
	Check("ancien en-tête, puis bon contexte : effet retrouvé sans recharger la partie", carte.Effect.text == "effet de POLICY_X");
	Check("ancien en-tête : deux compilations", compilations == 2);
end

-- Toujours dans le mauvais contexte : RMA n'est pas recompilé à chaque carte.
do
	compilations = 0;
	local gouv = Ecran(false);
	courant = Contexte("InGamePopup");
	for _ = 1, 10 do Dessiner(gouv); end
	Check("échecs répétés : au plus 3 compilations", compilations == 3);
end

-- RMA absent : l'en-tête le signale dans Lua.log.
do
	local rma = EMBARQUES.RealModifierAnalysis;
	EMBARQUES.RealModifierAnalysis = nil;
	local gouv = Ecran(true);
	Check("RMA absent : trace de l'en-tête", gouv.sortie[#gouv.sortie] == "BetterGameExperience : cartes de politique : RMA chargé sans CalculateModifierEffect");
	EMBARQUES.RealModifierAnalysis = rma;
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
