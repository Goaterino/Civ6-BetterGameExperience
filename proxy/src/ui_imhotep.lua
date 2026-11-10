-- ===========================================================================
-- Civ6RolloverProxy (src/ui_imhotep.lua), ajouté par le proxy à la fin de GreatPeoplePopup.lua et
-- de CivilopediaPage_GreatPerson.lua (textes/bgd_textes.lua et textes/bgd_texte.lua devant).
-- Le texte de l'effet d'Imhotep (LOC_GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL)
-- écrit 175 en dur, là où les autres ingénieurs passent par {Amount}. Les deux modificateurs
-- (_OTHER 175, _ANCIENT_CLASSICAL 350) ont des arguments ScaleByGameSpeed : le jeu donne la moitié
-- en vitesse en ligne (CostMultiplier 50), et le texte reste à 175. On le remplace par
-- LOC_BGD_IMHOTEP_EFFET, rempli avec les deux valeurs lues dans la base et mises à l'échelle de la
-- vitesse de la partie.
-- ===========================================================================
do
	local MOD_AUTRE = "GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_OTHER";
	local MOD_ANCIEN = "GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL";
	local TAG = "LOC_GREAT_PERSON_INDIVIDUAL_IMHOTEP_PRODUCTION_ANCIENT_CLASSICAL";

	-- { jeu = texte du jeu, autre = montant _OTHER, ancien = montant _ANCIENT_CLASSICAL } à la
	-- vitesse normale, ou false si Imhotep ou ses modificateurs manquent (rien n'est alors touché).
	-- La vitesse est relue à chaque appel : la Civilopédie du menu principal sert à plusieurs parties.
	local cache = nil;

	local function Lire()
		if GameInfo.GreatPersonIndividuals["GREAT_PERSON_INDIVIDUAL_IMHOTEP"] == nil then return false; end
		local brut = {};
		for row in GameInfo.ModifierArguments() do
			if row.Name == "Amount" and (row.ModifierId == MOD_AUTRE or row.ModifierId == MOD_ANCIEN) then
				brut[row.ModifierId] = tonumber(row.Value);
			end
		end
		if brut[MOD_AUTRE] == nil or brut[MOD_ANCIEN] == nil then return false; end
		local jeu = Locale.Lookup(TAG);
		if jeu == nil or jeu == "" or jeu == TAG then return false; end
		return { jeu = jeu, autre = brut[MOD_AUTRE], ancien = brut[MOD_ANCIEN] };
	end

	local function Multiplicateur()
		local vitesse = GameConfiguration.GetGameSpeedType();
		local info = vitesse ~= nil and GameInfo.GameSpeeds[vitesse];
		return info and tonumber(info.CostMultiplier);
	end

	local function Donnees()
		if cache == nil then
			local ok, r = pcall(Lire);
			cache = ok and r or false;
		end
		if not cache then return nil; end
		local ok, mult = pcall(Multiplicateur);
		if not ok or mult == nil then return nil; end
		-- Mise à l'échelle entière du jeu : 175 en ligne -> 87.
		local autre = math.floor(cache.autre * mult / 100);
		local ancien = math.floor(cache.ancien * mult / 100);
		return cache, mult, BGD_Txt("LOC_BGD_IMHOTEP_EFFET", autre, ancien);
	end

	-- Le texte d'effet corrigé, ou le texte tel quel s'il ne contient pas celui d'Imhotep.
	function BGD_ImhotepEffet(texte)
		if type(texte) ~= "string" then return texte; end
		local d, mult, bon = Donnees();
		if d == nil then return texte; end
		local i, j = string.find(texte, d.jeu, 1, true);
		if i ~= nil then
			return string.sub(texte, 1, i - 1) .. bon .. string.sub(texte, j + 1);
		end
		-- Texte d'effet construit autrement par le jeu : remplacé en entier s'il annonce encore le
		-- montant de la vitesse normale alors que la partie n'est pas à cette vitesse.
		if mult ~= 100 and string.find(texte, "%f[%d]" .. d.autre .. "%f[%D]") ~= nil then
			return bon;
		end
		return texte;
	end

	-- Écran des personnages illustres : ActionEffectText vient du jeu (GetTimeline).
	if type(PopulateData) == "function" then
		local PopulateData_jeu = PopulateData;
		function PopulateData(data, isPast, ...)
			local r = PopulateData_jeu(data, isPast, ...);
			local imhotep = GameInfo.GreatPersonIndividuals["GREAT_PERSON_INDIVIDUAL_IMHOTEP"];
			if imhotep ~= nil and type(data) == "table" and type(data.Timeline) == "table" then
				for _, kPerson in ipairs(data.Timeline) do
					if kPerson.IndividualID == imhotep.Index then
						kPerson.ActionEffectText = BGD_ImhotepEffet(kPerson.ActionEffectText);
					end
				end
			end
			return r;
		end
	end

	-- Civilopédie : le texte vient de GetModifierText (GameEffectsText.lua).
	if type(GetModifierText) == "function" then
		local GetModifierText_jeu = GetModifierText;
		function GetModifierText(modifierId, context, ...)
			local texte = GetModifierText_jeu(modifierId, context, ...);
			if modifierId == MOD_ANCIEN then
				return BGD_ImhotepEffet(texte);
			end
			return texte;
		end
	end
end
