-- ===========================================================================
-- BetterGameDLLs (src/mods/commun.lua) : fonctions communes aux mods d'interface repris par le proxy
-- (voir mods_ui/README.md). Chargé dans un contexte par Civ6Proxy_Charger("BGD_Commun"), fonction
-- native que le proxy pose dans la table globale du contexte avant de compiler son script :
--   Civ6Proxy_Charger(nom) -> fonction compilée du fichier embarqué `nom`, ou texte d'erreur.
-- Définit, dans la table globale du contexte :
--   BGD_Lookup(clé, …)  : Locale.Lookup, mais d'abord dans les textes des mods (textes.lua), qui ne
--                         sont plus dans la base sans le mod. Locale.Lookup met en forme un texte
--                         brut comme une clé ({1_Num}…).
--   BGD_Inclure(nom)    : exécute le fichier embarqué `nom` dans le contexte, comme include().
--   BGD_ChargerRMA()    : Real Modifier Analysis (de Better Report Screen), chargé une fois par
--                         contexte, dans celui du contexte (globales préfixées RMA_) ; rend sa
--                         table de fonctions.
-- ===========================================================================
if BGD_Lookup == nil then
	local function Erreur(nom, err)
		print("BetterGameExperience : " .. tostring(nom) .. " : " .. tostring(err));
	end

	local function Charger(nom)
		if Civ6Proxy_Charger == nil then
			Erreur(nom, "Civ6Proxy_Charger absent");
			return nil;
		end
		local f = Civ6Proxy_Charger(nom);
		if type(f) ~= "function" then
			Erreur(nom, f);
			return nil;
		end
		return f;
	end

	local f = Charger("BGD_Textes");
	if f ~= nil then f(); end
	if BGD_TEXTES == nil then BGD_TEXTES = {}; end

	local langue = "en_US";
	local l = Locale.GetCurrentLanguage and Locale.GetCurrentLanguage();
	if l ~= nil and type(l.Type) == "string" then langue = l.Type; end
	BGD_LANGUE = langue;
	BGD_FR = string.sub(string.lower(langue), 1, 2) == "fr";

	function BGD_Lookup(cle, ...)
		if type(cle) == "string" then
			local t = BGD_TEXTES[cle];
			if t ~= nil then
				local s = t[langue] or t.en_US;
				if s ~= nil then return Locale.Lookup(s, ...); end
			end
		end
		return Locale.Lookup(cle, ...);
	end

	-- Texte bilingue des blocs du proxy : { français, anglais }.
	function BGD_T(t)
		if BGD_FR then return t[1]; end
		return t[2];
	end

	function BGD_Inclure(nom)
		local fichier = Charger(nom);
		if fichier ~= nil then
			local ok, err = pcall(fichier);
			if not ok then Erreur(nom, err); end
		end
	end

	function BGD_ChargerRMA()
		if BGD_RMA ~= nil then return BGD_RMA; end
		local t = {};
		BGD_RMA = t;
		local fichier = Charger("RealModifierAnalysis");
		if fichier ~= nil then
			-- Dans l'environnement du contexte : HavokScript n'isole pas un chunk par setfenv (essai
			-- en jeu du 8 novembre 2026, GetCityData de CitySupport écrasé). Les globales de RMA sont
			-- donc toutes préfixées RMA_, et il garde sa propre copie de YieldTypes.
			BGD_RMA_Table = t;
			local ok, err = pcall(fichier);
			if not ok then Erreur("RealModifierAnalysis", err); end
		end
		return t;
	end
end
