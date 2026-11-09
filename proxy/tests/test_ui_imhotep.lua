-- Test hors jeu de src/ui_imhotep.lua :
--   lua proxy/tests/test_ui_imhotep.lua
-- On charge comme le proxy (`patch_great_people`, `patch_pedia_great_person`) : un fichier qui
-- définit PopulateData ou GetModifierText, puis textes/bgd_textes.lua, textes/bgd_texte.lua et le
-- bloc, dans un environnement qui imite l'interface (GameInfo, Locale, GameConfiguration).

local dir = arg and arg[0] and arg[0]:match("^(.*)[/\\]") or ".";
local function ReadFile(path)
	local f = assert(io.open(path, "rb"));
	local s = f:read("*a");
	f:close();
	return s;
end
local BLOCK = ReadFile(dir .. "/../../textes/bgd_textes.lua") .. "\n"
	.. ReadFile(dir .. "/../../textes/bgd_texte.lua") .. "\n"
	.. ReadFile(dir .. "/../src/ui_imhotep.lua");

local TAG = "LOC_GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL";
local JEU_FR = "[ICON_Production] Production +175 pour la construction de merveilles, et valeur doublée si la merveille est de l'Antiquité ou de l'ère classique.";
local IMHOTEP, AUTRE = 42, 7;

local failures = 0;
local function check(name, got, want)
	if got ~= want then
		failures = failures + 1;
		print(("ÉCHEC %s\n  obtenu : %s\n  attendu : %s"):format(name, tostring(got), tostring(want)));
	else
		print("ok " .. name);
	end
end

-- Un contexte d'interface neuf : `fichier` définit PopulateData ou GetModifierText, comme le
-- fichier du jeu, puis le bloc est chargé derrière.
local function Contexte(opts)
	local env = setmetatable({}, { __index = _G });
	local args = {
		{ ModifierId = "AUTRE_CHOSE", Name = "Amount", Value = "999" },
		{ ModifierId = "GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL", Name = "Amount", Value = tostring(opts.ancien or 350) },
		{ ModifierId = "GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL", Name = "KeepOverflow", Value = "false" },
		{ ModifierId = "GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_OTHER", Name = "Amount", Value = tostring(opts.autre or 175) },
	};
	env.lues = 0;
	local individus = {};
	if not opts.sansImhotep then individus.GREAT_PERSON_INDIVIDUAL_IMHOTEP = { Index = IMHOTEP }; end
	env.GameInfo = {
		GreatPersonIndividuals = individus,
		GameSpeeds = { [123] = { CostMultiplier = 50 }, [456] = { CostMultiplier = 100 }, [789] = { CostMultiplier = 67 } },
		ModifierArguments = function()
			env.lues = env.lues + 1;
			local i = 0;
			return function() i = i + 1; return args[i]; end;
		end,
	};
	env.vitesse = opts.vitesse or 123;
	env.GameConfiguration = { GetGameSpeedType = function() return env.vitesse; end };
	env.Locale = {
		GetCurrentLanguage = function() return { Type = opts.langue or "fr_FR" }; end,
		Lookup = function(k) if k == TAG then return opts.texteJeu or JEU_FR; end return k; end,
	};
	local fichier = opts.fichier or [[
		function PopulateData(data, isPast)
			for _, e in ipairs(ENTREES) do
				table.insert(data.Timeline, { IndividualID = e[1], ActionEffectText = e[2] });
			end
		end
	]];
	env.ENTREES = opts.entrees or {};
	local load_ = loadstring or load;
	for _, src in ipairs({ fichier, BLOCK }) do
		local f = assert(load_(src));
		if setfenv then setfenv(f, env) else f = assert(load(src, "bloc", "t", env)); end
		f();
	end
	return env;
end

local ATTENDU_FR = "[ICON_Production] Production +87 pour la construction de merveilles, portée à 175 si la merveille est de l'Antiquité ou de l'ère classique.";

-- Écran des personnages illustres, vitesse en ligne.
do
	local env = Contexte({ entrees = {
		{ IMHOTEP, JEU_FR },
		{ AUTRE, "Production +175 [ICON_Production]" },
		{ IMHOTEP, "[ICON_Bullet]" .. JEU_FR .. "[NEWLINE]Autre effet" },
	} });
	local data = { Timeline = {} };
	env.PopulateData(data, false);
	check("en ligne : texte remplacé", data.Timeline[1].ActionEffectText, ATTENDU_FR);
	check("autre personnage intact", data.Timeline[2].ActionEffectText, "Production +175 [ICON_Production]");
	check("texte autour gardé", data.Timeline[3].ActionEffectText, "[ICON_Bullet]" .. ATTENDU_FR .. "[NEWLINE]Autre effet");
	env.vitesse = 456;
	data = { Timeline = {} };
	env.PopulateData(data, false);
	check("vitesse normale", data.Timeline[1].ActionEffectText,
		"[ICON_Production] Production +175 pour la construction de merveilles, portée à 350 si la merveille est de l'Antiquité ou de l'ère classique.");
	env.vitesse = 789;
	data = { Timeline = {} };
	env.PopulateData(data, false);
	check("vitesse rapide (67 %)", data.Timeline[1].ActionEffectText:match("%+(%d+)") .. "/" .. data.Timeline[1].ActionEffectText:match("à (%d+)"), "117/234");
	check("arguments lus une fois", env.lues, 1);
end

-- Texte du jeu construit autrement : remplacé en entier s'il porte encore 175 hors vitesse normale.
do
	local env = Contexte({ entrees = { { IMHOTEP, "+175 [ICON_Production] (x2 Antiquité)" }, { IMHOTEP, "+87, 175 en vitesse en ligne" } } });
	local data = { Timeline = {} };
	env.PopulateData(data, true);
	check("texte inconnu avec 175", data.Timeline[1].ActionEffectText, ATTENDU_FR);
	env.vitesse = 456;
	data = { Timeline = {} };
	env.PopulateData(data, true);
	check("texte inconnu, vitesse normale : intact", data.Timeline[1].ActionEffectText, "+175 [ICON_Production] (x2 Antiquité)");
end

-- Anglais, et montants changés par un mod.
do
	local env = Contexte({ langue = "en_US", autre = 200, ancien = 300,
		texteJeu = "Grants 175 [ICON_Production] Production towards wonder construction, doubled if the wonder is from the Ancient or Classical Era.",
		entrees = { { IMHOTEP, "Grants 175 [ICON_Production] Production towards wonder construction, doubled if the wonder is from the Ancient or Classical Era." } } });
	local data = { Timeline = {} };
	env.PopulateData(data, false);
	check("anglais, montants du mod", data.Timeline[1].ActionEffectText,
		"Grants 100 [ICON_Production] Production towards wonder construction, 150 if the wonder is from the Ancient or Classical Era.");
end

-- Sans Imhotep (pas de DLC) : rien ne change, pas d'erreur.
do
	local env = Contexte({ sansImhotep = true, entrees = { { IMHOTEP, JEU_FR } } });
	local data = { Timeline = {} };
	env.PopulateData(data, false);
	check("sans Imhotep", data.Timeline[1].ActionEffectText, JEU_FR);
end

-- Civilopédie : GetModifierText enveloppé pour le seul modificateur d'Imhotep.
do
	local env = Contexte({ fichier = [[
		function GetModifierText(modifierId, context)
			if modifierId == "GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL" then return Locale.Lookup("LOC_GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL"); end
			return "texte de " .. modifierId;
		end
	]] });
	check("Civilopédie : Imhotep", env.GetModifierText("GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL", "Summary"), ATTENDU_FR);
	check("Civilopédie : autre modificateur", env.GetModifierText("GREATPERSON_GRANT_PRODUCTION_IN_CITY_MEDIEVAL", "Summary"),
		"texte de GREATPERSON_GRANT_PRODUCTION_IN_CITY_MEDIEVAL");
	check("Civilopédie : pas de PopulateData créé", env.PopulateData, nil);
end

if failures > 0 then
	print(failures .. " échec(s)");
	os.exit(1);
end
print("tout est bon");
