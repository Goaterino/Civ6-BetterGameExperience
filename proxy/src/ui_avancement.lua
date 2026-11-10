-- ===========================================================================
-- Civ6RolloverProxy (src/ui_avancement.lua), ajouté par le proxy à la fin de WorldTracker.lua (jeu
-- de base, ou celui de BSM) avec `ui_avancement=on` : « x/y » à droite du nom de la technologie et
-- du dogme en cours, dans le panneau en haut à gauche, suivi de « (+z) » s'il y a un excédent.
-- x est la science (la culture) déjà placée, y le coût total, z la science (la culture) gardée en
-- excédent par le jeu, versée plus tard dans la recherche en cours. Le libellé `BGD_Avancement` est
-- ajouté par le proxy à WorldTrackerResearch.xml et WorldTrackerCivic.xml ; sans lui, rien ne
-- s'affiche. L'excédent est lu dans GameCore par `BGD_Excedent(joueur, 0 science | 1 culture)`
-- (src/avancement.rs) ; sans elle, pas de « (+z) ».
--
-- UpdateResearchPanel et UpdateCivicsPanel sont enveloppées : elles redessinent le panneau à chaque
-- changement de recherche, de rendement ou de tour (et le panneau XP1 / XP2 les enveloppe à son tour).
-- Rien n'est montré pour une technologie ou un dogme terminé, ni quand rien n'est en cours. Le nom
-- est raccourci (infobulle du nom complet) s'il chevaucherait le compteur.
-- Tout s'exécute sous pcall ; un échec écrit `Avancement echec <erreur>`.
-- ===========================================================================
do
local Avancement_Ok, Avancement_Err = pcall(function()
	if BGD_Avancement_Pose ~= nil or type(UpdateResearchPanel) ~= "function" or type(UpdateCivicsPanel) ~= "function" then
		return;
	end
	BGD_Avancement_Pose = true;
	-- Marge à droite du libellé (XML), écart avec le nom, largeur du nom dans le jeu (MAX_BEFORE_TRUNC_TITLE).
	local MARGE, ECART, TITRE_MAX = 0, 8, 225;

	-- Une décimale, sans « .0 ».
	local function Points(v)
		local s = string.format("%.1f", math.floor(v * 10 + 0.5) / 10);
		return (s:gsub("%.0$", ""));
	end

	-- « x/y (+z) » coloré (« (+z) » seulement si l'excédent arrondi dépasse 0), ou nil sans coût.
	function BGD_TexteAvancement(progres, cout, couleur, excedent)
		if type(progres) ~= "number" or type(cout) ~= "number" or cout <= 0 then
			return nil;
		end
		local t = couleur .. tostring(math.floor(progres)) .. "/" .. tostring(math.floor(cout + 0.5));
		if type(excedent) == "number" and excedent >= 0.05 then
			t = t .. " (+" .. Points(excedent) .. ")";
		end
		return t .. "[ENDCOLOR]";
	end

	-- Excédent du joueur (0 science, 1 culture), nil s'il est illisible.
	local function Excedent(ePlayer, quoi)
		if type(BGD_Excedent) ~= "function" then
			return nil;
		end
		local ok, v = pcall(BGD_Excedent, ePlayer, quoi);
		return ok and v or nil;
	end

	-- Avant le jeu : le nom reprend toute sa largeur (un raccourci précédent resterait sinon).
	local function Elargir(inst)
		if inst == nil or inst.BGD_Avancement == nil or inst.TitleButton == nil or inst.TitleButton.GetTextControl == nil then
			return;
		end
		local t = inst.TitleButton:GetTextControl();
		if t ~= nil and t.SetTruncateWidth ~= nil then
			t:SetTruncateWidth(TITRE_MAX);
		end
	end

	local function Afficher(inst, texte, nom)
		local label = inst.BGD_Avancement;
		if texte == nil then
			label:SetText("");
			label:SetHide(true);
			return;
		end
		label:SetText(texte);
		label:SetHide(false);
		local titre = inst.TitleButton;
		if titre ~= nil and nom ~= nil and type(TruncateStringWithTooltip) == "function" then
			local place = inst.MainPanel:GetSizeX() - MARGE - label:GetSizeX() - ECART - titre:GetOffsetX();
			TruncateStringWithTooltip(titre, math.min(place, TITRE_MAX), nom);
		end
	end

	-- Joueur local et son objet, ou nil (observateur, hors partie).
	local function Joueur()
		local ePlayer = Game.GetLocalPlayer();
		if ePlayer == nil or ePlayer == PlayerTypes.NONE or ePlayer == PlayerTypes.OBSERVER then
			return nil;
		end
		local pPlayer = Players[ePlayer];
		if pPlayer == nil then
			return nil;
		end
		return ePlayer, pPlayer;
	end

	-- ID en cours : celui que suit le panneau s'il est lisible, sinon celui du jeu.
	local function EnCours(suivi, duJeu)
		if type(suivi) == "number" then
			return suivi;
		end
		return duJeu;
	end

	local function MajRecherche()
		local inst = m_researchInstance;
		if inst == nil or inst.BGD_Avancement == nil then
			return;
		end
		local ePlayer, pPlayer = Joueur();
		if pPlayer == nil then
			Afficher(inst, nil);
			return;
		end
		local techs = pPlayer:GetTechs();
		local i = EnCours(m_currentResearchID, techs:GetResearchingTech());
		local row = (i ~= nil and i >= 0) and GameInfo.Technologies[i] or nil;
		if row == nil then
			Afficher(inst, nil);
			return;
		end
		Afficher(inst, BGD_TexteAvancement(techs:GetResearchProgress(i), techs:GetResearchCost(i), "[COLOR:ResScienceLabelCS]",
			Excedent(ePlayer, 0)), Locale.ToUpper(Locale.Lookup(row.Name)));
	end

	local function MajDogme()
		local inst = m_civicsInstance;
		if inst == nil or inst.BGD_Avancement == nil then
			return;
		end
		local ePlayer, pPlayer = Joueur();
		if pPlayer == nil then
			Afficher(inst, nil);
			return;
		end
		local culture = pPlayer:GetCulture();
		local i = EnCours(m_currentCivicID, culture:GetProgressingCivic());
		local row = (i ~= nil and i >= 0) and GameInfo.Civics[i] or nil;
		if row == nil then
			Afficher(inst, nil);
			return;
		end
		Afficher(inst, BGD_TexteAvancement(culture:GetCulturalProgress(i), culture:GetCultureCost(i), "[COLOR:ResCultureLabelCS]",
			Excedent(ePlayer, 1)), Locale.ToUpper(Locale.Lookup(row.Name)));
	end

	local function Proteger(f)
		local ok, err = pcall(f);
		if not ok then
			print("Avancement echec " .. tostring(err));
		end
	end

	local BaseRecherche = UpdateResearchPanel;
	function UpdateResearchPanel(isHideResearch)
		Proteger(function() Elargir(m_researchInstance); end);
		BaseRecherche(isHideResearch);
		Proteger(MajRecherche);
	end

	local BaseDogme = UpdateCivicsPanel;
	function UpdateCivicsPanel(hideCivics)
		Proteger(function() Elargir(m_civicsInstance); end);
		BaseDogme(hideCivics);
		Proteger(MajDogme);
	end
end);
if not Avancement_Ok then
	print("Avancement echec " .. tostring(Avancement_Err));
end
end
