-- ===========================================================================
-- Civ6RolloverProxy (src/ui_espions.lua), ajouté par le proxy à la fin de EspionageSupport.lua avec
-- `ui_espions=on`. EspionageSupport.lua est inclus par le choix de mission (EspionageChooser) et par
-- la fenêtre de mission (EspionagePopup) : les deux chiffres changent ensemble.
--
-- Le jeu affiche P(succès non détecté) + P(succès, doit s'échapper). Or l'issue « doit s'échapper »
-- n'est un succès que si l'espion s'évade : le butin (boost, or, œuvre) n'est versé qu'à l'évasion,
-- et l'espion des autres missions peut être capturé ou tué (analyses/espions/README.md). Le chiffre
-- devient :
--   P5 + P4 × P(évasion)
--   P(évasion) = p × évadé(E − POLICE) + (1 − p) × évadé(E)
--   E = ESPIONAGE_ESCAPE_BASE_CHANCE − ESPIONAGE_ESCAPE_LEVEL_BOOST × (niveau − 1) − bonus d'évasion
--   évadé(e) = part des jets de 3d6 ≥ e
--   p = poids de la meilleure sortie / somme des poids des sorties de la ville (la police la devine)
-- Les sorties sont les lignes de GameInfo.Districts de TravelTime positif dont la ville a un quartier
-- terminé, de ce type ou qui le remplace ; poids = (plus grand TravelTime) − TravelTime + 1. Le
-- contre-espion du défenseur n'est pas compté, comme dans le jeu : le joueur n'est pas censé savoir
-- s'il y en a un. Règles relevées dans GameCore (analyses/espions/plan_affichage.md).
--
-- Seul UnitManager.GetResultProbability change, et dans ce contexte seulement : le nom UnitManager
-- y est remplacé par une table qui renvoie tout le reste à l'original. La part retirée à « doit
-- s'échapper » rejoint d'elle-même « capturé ou tué » dans la fenêtre de mission, qui le calcule
-- par différence. Rien n'est envoyé au jeu : affichage seul. Tout passe sous pcall ; en cas
-- d'erreur, le chiffre du jeu reste et `Espions echec <erreur>` va dans Lua.log.
-- ===========================================================================
do
local Espions_Ok, Espions_Err = pcall(function()
	if BGD_Espions_Original ~= nil or UnitManager == nil or type(UnitManager.GetResultProbability) ~= "function" then
		return;
	end
	local Gestionnaire = UnitManager;
	local Original = Gestionnaire.GetResultProbability;
	BGD_Espions_Original = Original;

	-- Table de 3d6 de GameCore (DAT_180b64c70), indices 0 à 18, total 216.
	local DES = { [0] = 0, 0, 0, 1, 3, 6, 10, 15, 21, 25, 27, 27, 25, 21, 15, 10, 6, 3, 1 };

	local function Evade(e)
		local n = 0;
		for i = math.max(e, 3), 18 do
			n = n + DES[i];
		end
		return n / 216;
	end

	-- Fonction pure, testée hors jeu (proxy/tests/test_ui_espions.lua).
	function BGD_Espions_Affiche(p5, p4, niveau, bonus, poidsMin, poidsTotal, base, parNiveau, police)
		local e = base - parNiveau * (niveau - 1) - bonus;
		local p = (poidsTotal > 0) and (poidsMin / poidsTotal) or 1;
		local evasion = p * Evade(e - police) + (1 - p) * Evade(e);
		return p5 + p4 * evasion, evasion;
	end

	local function Parametre(nom, defaut)
		local row = GameInfo.GlobalParameters and GameInfo.GlobalParameters[nom];
		return (row ~= nil and tonumber(row.Value)) or defaut;
	end

	-- Bonus d'évasion de chaque promotion (effet EFFECT_ADJUST_UNIT_ESCAPE_BOOST), calculé une fois.
	local BonusPromotions = nil;
	local function Bonus(spy)
		if BonusPromotions == nil then
			BonusPromotions = {};
			local effets = {};
			for row in GameInfo.DynamicModifiers() do
				if row.EffectType == "EFFECT_ADJUST_UNIT_ESCAPE_BOOST" then
					effets[row.ModifierType] = true;
				end
			end
			local montants = {};
			for row in GameInfo.Modifiers() do
				if effets[row.ModifierType] then
					montants[row.ModifierId] = 0;
				end
			end
			for row in GameInfo.ModifierArguments() do
				if montants[row.ModifierId] ~= nil and row.Name == "Amount" then
					montants[row.ModifierId] = tonumber(row.Value) or 0;
				end
			end
			local attaches = {};
			for row in GameInfo.UnitPromotionModifiers() do
				local m = montants[row.ModifierId];
				if m ~= nil then
					local promo = GameInfo.UnitPromotions[row.UnitPromotionType];
					if promo ~= nil then
						BonusPromotions[promo.Index] = (BonusPromotions[promo.Index] or 0) + m;
						attaches[row.ModifierId] = true;
					end
				end
			end
			for id, _ in pairs(montants) do
				if not attaches[id] then
					print("BGD espions : bonus d'evasion " .. id .. " hors promotion, non compte");
				end
			end
		end
		local exp = spy:GetExperience();
		local total = 0;
		if exp ~= nil then
			for index, montant in pairs(BonusPromotions) do
				if exp:HasPromotion(index) then
					total = total + montant;
				end
			end
		end
		return total;
	end

	-- Poids de la meilleure sortie et somme des poids (180371b10).
	function BGD_Espions_Sorties(city)
		local max = 0;
		for row in GameInfo.Districts() do
			local t = tonumber(row.TravelTime) or 0;
			if t > max then
				max = t;
			end
		end
		local presents = {};
		for _, district in city:GetDistricts():Members() do
			if district:IsComplete() then
				local info = GameInfo.Districts[district:GetType()];
				if info ~= nil then
					presents[info.DistrictType] = true;
					local remplace = GameInfo.DistrictReplaces and GameInfo.DistrictReplaces[info.DistrictType];
					if remplace ~= nil then
						presents[remplace.ReplacesDistrictType] = true;
					end
				end
			end
		end
		local total, mini = 0, nil;
		for row in GameInfo.Districts() do
			local t = tonumber(row.TravelTime) or 0;
			if t > 0 and presents[row.DistrictType] then
				local poids = max - t + 1;
				total = total + poids;
				if mini == nil or poids < mini then
					mini = poids;
				end
			end
		end
		return mini or 1, total;
	end

	local Signale = false;
	local function Corrige(op, spy, plot)
		local r = Original(op, spy, plot);
		if type(r) ~= "table" then
			return r;
		end
		local ok, res = pcall(function()
			local p4 = r["ESPIONAGE_SUCCESS_MUST_ESCAPE"];
			if p4 == nil or p4 <= 0 or spy == nil or plot == nil then
				return r;
			end
			local city = Cities.GetPlotPurchaseCity(plot);
			if city == nil then
				return r;
			end
			local mini, total = BGD_Espions_Sorties(city);
			local niveau = spy:GetExperience():GetLevel();
			local _, evasion = BGD_Espions_Affiche(0, p4, niveau, Bonus(spy), mini, total,
				Parametre("ESPIONAGE_ESCAPE_BASE_CHANCE", 10), Parametre("ESPIONAGE_ESCAPE_LEVEL_BOOST", 1),
				Parametre("ESPIONAGE_ESCAPE_POLICE_CORRECT_MODIFIER", -4));
			local copie = {};
			for k, v in pairs(r) do
				copie[k] = v;
			end
			copie["ESPIONAGE_SUCCESS_MUST_ESCAPE"] = p4 * evasion;
			return copie;
		end);
		if ok then
			return res;
		end
		if not Signale then
			Signale = true;
			print("Espions echec " .. tostring(res));
		end
		return r;
	end

	UnitManager = setmetatable({ GetResultProbability = Corrige }, { __index = Gestionnaire });
end);
if not Espions_Ok then
	print("Espions echec " .. tostring(Espions_Err));
end
end
