
-- ===========================================================================
-- BetterGameDLLs (textes/bgd_texte.lua) : textes traduits des blocs Lua des proxys (voir
-- textes/README.md). Les proxys le placent, après la table BGD_TXT (textes/bgd_textes.lua), devant
-- chaque bloc qui affiche du texte. Chaque contexte d'interface a ses globales : le premier bloc
-- d'un contexte définit les fonctions, les suivants les trouvent déjà :
--   BGD_Txt(clé, …)         -> le texte dans la langue du jeu, {1}, {2}… remplacés par les arguments ;
--                              à défaut une langue de même base (pt_PT -> pt_BR), puis l'anglais,
--                              puis la clé elle-même (un texte brut, « x5 » par exemple, passe tel quel).
--   BGD_TxtAnglais(clé, …)  -> le texte anglais : les messages de chat envoyés aux autres joueurs
--                              partent en anglais, lisible même sans BetterGameDLLs.
--   BGD_TxtChat(texte)      -> un message de chat de BetterGameDLLs reçu en anglais
--                              (« [COLOR_…]BGD - <texte d'une clé LOC_BGD_CHAT_*>[ENDCOLOR] »),
--                              traduit dans la langue du jeu ; nil si ce n'en est pas un.
-- ===========================================================================
if type(BGD_Txt) ~= "function" then
	local BGD_TxtLangue = "en_US";
	do
		local ok, l = pcall(function() return Locale.GetCurrentLanguage(); end);
		if ok and l ~= nil and type(l.Type) == "string" then BGD_TxtLangue = l.Type; end
	end
	local BGD_TxtBase = string.sub(BGD_TxtLangue, 1, 2);

	local function Brut(cle, langue)
		local t = BGD_TXT ~= nil and BGD_TXT[cle];
		if not t then return nil; end
		local s = t[langue];
		if s == nil and langue == BGD_TxtLangue then
			for l, v in pairs(t) do
				if string.sub(l, 1, 2) == BGD_TxtBase then s = v; break; end
			end
		end
		return s or t.en_US;
	end

	-- {n} remplacé par le n-ième argument ; laissé tel quel s'il manque.
	local function Remplir(s, args)
		return (string.gsub(s, "{(%d+)}", function(n)
			local v = args[tonumber(n)];
			if v == nil then return nil; end
			return tostring(v);
		end));
	end

	function BGD_Txt(cle, ...)
		local s = Brut(cle, BGD_TxtLangue);
		if s == nil then return Remplir(tostring(cle), { ... }); end
		return Remplir(s, { ... });
	end

	function BGD_TxtAnglais(cle, ...)
		return Remplir(Brut(cle, "en_US") or tostring(cle), { ... });
	end

	-- Modèles des messages de chat, du plus long au plus court : un modèle court ne doit pas prendre
	-- le message d'un plus long qui commence de la même façon.
	local modeles = nil;
	local function Modeles()
		if modeles ~= nil then return modeles; end
		modeles = {};
		for cle, t in pairs(BGD_TXT or {}) do
			if string.sub(cle, 1, 13) == "LOC_BGD_CHAT_" and t.en_US ~= nil then
				local ordre = {};
				local motif = string.gsub(t.en_US, "[%^%$%(%)%%%.%[%]%*%+%-%?]", "%%%0");
				motif = string.gsub(motif, "{(%d+)}", function(n)
					table.insert(ordre, tonumber(n));
					return "(.-)";
				end);
				table.insert(modeles, { cle = cle, motif = "^" .. motif .. "$", ordre = ordre, longueur = string.len(t.en_US) });
			end
		end
		table.sort(modeles, function(a, b) return a.longueur > b.longueur; end);
		return modeles;
	end

	function BGD_TxtChat(texte)
		if type(texte) ~= "string" or BGD_TxtBase == "en" then return nil; end
		local debut, corps, fin = string.match(texte, "^(%[COLOR_[%w_]+%]BGD %- )(.*)(%[ENDCOLOR%])$");
		if debut == nil then return nil; end
		for _, m in ipairs(Modeles()) do
			local captures = { string.match(corps, m.motif) };
			if captures[1] ~= nil then
				local args = {};
				if #m.ordre > 0 then
					for i, n in ipairs(m.ordre) do args[n] = captures[i]; end
				end
				local s = Brut(m.cle, BGD_TxtLangue);
				if s == nil then return nil; end
				return debut .. Remplir(s, args) .. fin;
			end
		end
		return nil;
	end
end
