-- ===========================================================================
-- Civ6RolloverProxy (src/ui_formations.lua), appended by the proxy to the end of
-- ProductionPanel.lua with `ui_formations=on`: in the production list, the main button of a unit
-- trains the highest formation the city can train (army, else corps, else the single unit; armada
-- and fleet at sea), and the drawer under it holds the lower ones.
--
-- The game does the opposite: the main button trains the single unit, and the corps and the army
-- sit in a drawer that the arrow on the right has to open first. A city only offers them with a
-- Military Academy (a Seaport at sea), which also makes them 25% cheaper to train, so they are
-- what one wants from such a city nearly every time: two clicks each time, one of them on a small
-- arrow.
--
--   the game:               this block, army offered:      corps only:
--   [ Infantry        v ]   [ Infantry (Army)       v ]    [ Infantry (Corps)  v ]
--     [ Infantry (Corps) ]    [ Infantry (Corps)      ]      [ Infantry          ]
--     [ Infantry (Army)  ]    [ Infantry              ]
--
-- Nothing is rebuilt: the game fills the row (PopulateUnits), then its controls are dressed again.
-- PopulateGenericBuildData, which PopulateUnits calls once per row, is wrapped to note each row and
-- its item; after PopulateUnits, each noted row that offers a corps or an army gets:
-- - on the main button, the highest formation that is not disabled: name with the game's own
--   suffix ("(Army[ICON_Army])", the text of the drawer), turns, tooltip, progress and click;
-- - in the drawer, the other formations, highest first, in the slots the game shows (the corps
--   slot, then the army slot): the same five things. The single unit has no suffix.
-- The clicks call the game's own BuildUnit, BuildUnitCorps and BuildUnitArmy: the commands sent
-- are the ones the game sends, nothing changes for the other players.
--
-- Left as the game draws them:
-- - a row whose corps and army are both disabled (strategic resources missing, for instance): the
--   single unit stays on the main button. With the army disabled and the corps not, the corps
--   takes the main button;
-- - the gold and faith purchase lists: a click there buys at once, for 2 to 3 times the price, and
--   the 25% discount does not apply to purchases;
-- - the tutorial.
--
-- The cost of the single unit is read from the build queue (GetUnitCost): the game overwrites
-- item.Cost with the cost of the corps when a corps is offered (ProductionPanel.lua, GetData).
-- Display and click target only. Everything runs under pcall; a failure writes
-- `Formations echec <error>` and leaves the row as the game drew it.
-- ===========================================================================
do
local Formations_Ok, Formations_Err = pcall(function()
	if BGD_Formations_Etat ~= nil or type(PopulateUnits) ~= "function" or type(PopulateGenericBuildData) ~= "function" then
		return;
	end
	-- lignes : the rows noted while PopulateUnits runs, nil outside of it.
	local S = { lignes = nil };
	BGD_Formations_Etat = S;

	-- The colour of the suffix in the drawer (ProductionPanel.xml, CorpsLabelIcon, Color0).
	local SUFFIXE_DEBUT = "[COLOR:217,180,119,255]";
	local SUFFIXE_FIN = "[ENDCOLOR]";

	-- The two slots of the drawer, in the order of its stack.
	local EMPLACEMENTS = {
		{ offert = "Corps", bouton = "TrainCorpsButton", suffixe = "CorpsLabelIcon", cout = "CorpsCostText",
			zone = "ProductionCorpsProgressArea", barre = "ProductionCorpsProgress", bloque = "CorpsDisabled" },
		{ offert = "Army", bouton = "TrainArmyButton", suffixe = "ArmyLabelIcon", cout = "ArmyCostText",
			zone = "ProductionArmyProgressArea", barre = "ProductionArmyProgress", bloque = "ArmyDisabled" },
	};

	local function CoutSimple(data, item)
		local row = GameInfo.Units[item.Type];
		local file = data.City ~= nil and data.City:GetBuildQueue() or nil;
		local cout = (row ~= nil and file ~= nil) and file:GetUnitCost(row.Index) or nil;
		if type(cout) ~= "number" then
			cout = item.Cost;
		end
		return cout;
	end

	-- The formations the row offers, highest first.
	local function Formations(data, item)
		local t = {};
		if item.Army then
			t[#t + 1] = { suffixe = item.ArmyName, tours = item.ArmyTurnsLeft, cout = item.ArmyCost,
				bulle = item.ArmyTooltip, bloque = item.ArmyDisabled == true, lancer = BuildUnitArmy };
		end
		if item.Corps then
			t[#t + 1] = { suffixe = item.CorpsName, tours = item.CorpsTurnsLeft, cout = item.CorpsCost,
				bulle = item.CorpsTooltip, bloque = item.CorpsDisabled == true, lancer = BuildUnitCorps };
		end
		t[#t + 1] = { suffixe = "", tours = item.TurnsLeft, cout = CoutSimple(data, item),
			bulle = item.ToolTip, bloque = item.Disabled == true, lancer = BuildUnit, simple = true };
		return t;
	end

	-- Same rule as the game's rows: a meter once production is placed, hidden when it covers the cost.
	local function Progression(zone, barre, progres, cout)
		if type(progres) == "number" and type(cout) == "number" and progres > 0 and cout > 0 and progres / cout < 1 then
			barre:SetPercent(progres / cout);
			zone:SetHide(false);
		else
			zone:SetHide(true);
		end
	end

	local function Tours(label, f)
		local texte, bulle = GetTurnsToCompleteStrings(f.tours);
		label:SetText(texte);
		label:SetToolTipString(bulle);
	end

	local function Clic(bouton, data, item, f)
		bouton:RegisterCallback(Mouse.eLClick, function()
			f.lancer(data.City, item);
			CloseAfterNewProduction();
		end);
	end

	local function Reordonner(data, ligne, item)
		local formations = Formations(data, item);
		local tete = nil;
		for i, f in ipairs(formations) do
			if not f.bloque then
				tete = i;
				break;
			end
		end
		-- Nothing higher than the single unit can be trained: the game's row is the right one.
		if tete == nil or formations[tete].simple then
			return;
		end
		local f = table.remove(formations, tete);

		-- Main button. The formation is not disabled: the button is shown and active, as the game
		-- leaves it for an item that can be produced (PopulateGenericItemData).
		local nom = Locale.Lookup(item.Name);
		if f.suffixe ~= nil and f.suffixe ~= "" then
			nom = nom .. "[NEWLINE]" .. SUFFIXE_DEBUT .. f.suffixe .. SUFFIXE_FIN;
		end
		ligne.LabelText:SetText(nom);
		ligne.Button:SetToolTipString(f.bulle);
		ligne.Disabled:SetToolTipString(f.bulle);
		ligne.Button:SetHide(false);
		ligne.Disabled:SetHide(true);
		ligne.Button:SetColor(UI.GetColorValue("COLOR_WHITE"));
		ligne.Button:SetDisabled(false);
		Tours(ligne.CostText, f);
		Progression(ligne.ProductionProgressArea, ligne.ProductionProgress, item.Progress, f.cout);
		Clic(ligne.Button, data, item, f);

		-- Drawer: the formations left, highest first, in the slots the game shows.
		local suivant = 1;
		for _, e in ipairs(EMPLACEMENTS) do
			local g = item[e.offert] and formations[suivant] or nil;
			if g ~= nil then
				suivant = suivant + 1;
				ligne[e.suffixe]:SetText(g.suffixe or "");
				ligne[e.bouton]:SetToolTipString(g.bulle);
				ligne[e.bloque]:SetToolTipString(g.bulle);
				ligne[e.bloque]:SetHide(not g.bloque);
				Tours(ligne[e.cout], g);
				Progression(ligne[e.zone], ligne[e.barre], item.Progress, g.cout);
				Clic(ligne[e.bouton], data, item, g);
			end
		end
	end

	local Remplir_jeu = PopulateGenericBuildData;
	function PopulateGenericBuildData(kInstance, kItem)
		Remplir_jeu(kInstance, kItem);
		if S.lignes ~= nil and kInstance ~= nil and kItem ~= nil and kInstance.ArmyCorpsDrawer ~= nil
			and (kItem.Corps or kItem.Army) then
			S.lignes[#S.lignes + 1] = { kInstance, kItem };
		end
	end

	local Unites_jeu = PopulateUnits;
	function PopulateUnits(data, listMode, listIM)
		if m_isTutorialRunning or (listMode ~= LISTMODE.PRODUCTION and listMode ~= LISTMODE.PROD_QUEUE) then
			return Unites_jeu(data, listMode, listIM);
		end
		S.lignes = {};
		local ok, err = pcall(Unites_jeu, data, listMode, listIM);
		local lignes = S.lignes;
		S.lignes = nil;
		if not ok then
			error(err, 0);
		end
		for _, l in ipairs(lignes) do
			local fait, echec = pcall(Reordonner, data, l[1], l[2]);
			if not fait then
				print("Formations echec " .. tostring(echec));
			end
		end
	end
end);
if not Formations_Ok then
	print("Formations echec " .. tostring(Formations_Err));
end
end
