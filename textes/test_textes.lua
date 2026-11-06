-- Test hors jeu des textes traduits (textes/bgd_texte.lua) :
--   lua textes/test_textes.lua
-- Pour chaque langue : BGD_Txt, les replis (langue de même base, anglais, clé), et chaque message de
-- chat (LOC_BGD_CHAT_*) envoyé en anglais puis affiché dans la langue du joueur, arguments compris.
local dir = (arg and arg[0] or ""):match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("a");
	f:close();
	return s;
end
local CODE = ReadFile(dir .. "/bgd_textes.lua") .. ReadFile(dir .. "/bgd_texte.lua");

local echecs = 0;
local function Check(nom, ok)
	if not ok then
		echecs = echecs + 1;
		print("ECHEC   " .. nom);
	end
end

-- Un contexte d'interface neuf, dans la langue `langue`.
local function Contexte(langue)
	local env = setmetatable({
		Locale = { GetCurrentLanguage = function() return { Type = langue }; end },
	}, { __index = _G });
	assert(load(CODE, "=textes", "t", env))();
	return env;
end

local LANGUES = { "en_US", "fr_FR", "de_DE", "es_ES", "it_IT", "ja_JP", "ko_KR", "pl_PL", "pt_BR", "ru_RU", "zh_Hans_CN" };
local anglais = Contexte("en_US");

for _, langue in ipairs(LANGUES) do
	local env = Contexte(langue);
	local attendu = env.BGD_TXT.LOC_BGD_OPT_TITRE_GENERAL[langue] or env.BGD_TXT.LOC_BGD_OPT_TITRE_GENERAL.en_US;
	Check(langue .. " : texte simple", env.BGD_Txt("LOC_BGD_OPT_TITRE_GENERAL") == attendu);
	Check(langue .. " : paramètre", env.BGD_Txt("LOC_BGD_OPT_VALEUR_FICHIER", "7"):find("7", 1, true) ~= nil);
	Check(langue .. " : texte brut tel quel", env.BGD_Txt("x5") == "x5");
	for cle, t in pairs(env.BGD_TXT) do
		if cle:sub(1, 13) == "LOC_BGD_CHAT_" then
			local args = { "Bob, Zoé (0.1.0)", "0.5.0", "Léo" };
			local envoye = "[COLOR_RED]BGD - " .. anglais.BGD_TxtAnglais(cle, args[1], args[2], args[3]) .. "[ENDCOLOR]";
			local recu = env.BGD_TxtChat(envoye);
			if langue == "en_US" then
				Check(cle .. " : rien à traduire en anglais", recu == nil);
			else
				local voulu = "[COLOR_RED]BGD - " .. env.BGD_Txt(cle, args[1], args[2], args[3]) .. "[ENDCOLOR]";
				Check(langue .. " " .. cle .. " : " .. tostring(recu), recu == voulu);
			end
		end
	end
	Check(langue .. " : message d'un joueur laissé tel quel", env.BGD_TxtChat("salut") == nil);
	Check(langue .. " : message BGD inconnu laissé tel quel", env.BGD_TxtChat("[COLOR_RED]BGD - autre chose[ENDCOLOR]") == nil);
end

-- pt_PT n'existe pas : le portugais du Brésil.
local pt = Contexte("pt_PT");
Check("pt_PT : repli sur pt_BR", pt.BGD_Txt("LOC_BGD_OPT_TITRE_GENERAL") == (pt.BGD_TXT.LOC_BGD_OPT_TITRE_GENERAL.pt_BR or pt.BGD_TXT.LOC_BGD_OPT_TITRE_GENERAL.en_US));
-- Langue inconnue : l'anglais.
local xx = Contexte("xx_XX");
Check("langue inconnue : anglais", xx.BGD_Txt("LOC_BGD_OPT_TITRE_GENERAL") == "General");
Check("clé inconnue : la clé", xx.BGD_Txt("LOC_BGD_INCONNUE") == "LOC_BGD_INCONNUE");

if echecs > 0 then
	print(echecs .. " échec(s)");
	os.exit(1);
end
print("textes : tout est bon");
