-- ===========================================================================
-- BetterGameDLLs (src/mods/rapports/liste.lua), ajouté par le proxy à la fin de ReportsList.lua :
-- la liste des rapports de Better Report Screen (Infixo), d'après ReportsList Loader (RLL, du même
-- auteur). RLL lit la liste dans une table de la base (RLLReports) que le mod crée ; ici elle est
-- écrite en dur. Si la table existe (RLL fourni par BRS ou par un autre mod), ses lignes s'ajoutent
-- à la nôtre, ou la remplacent pour un même rapport : les rapports des autres mods restent.
-- m_ReportButtonIM et AddReport sont ceux de ReportsList.lua.
-- ===========================================================================
do
	local f = Civ6Proxy_Charger and Civ6Proxy_Charger("BGD_Commun");
	if type(f) == "function" then f(); end
	local L = BGD_Lookup or Locale.Lookup;
	local isXP1 = Modding.IsModActive("1B28771A-C749-434B-9053-D1380C553DE9");
	local isXP2 = Modding.IsModActive("4873eb62-8ccc-4574-b784-dda455e74e68");

	-- ReportType, libellé, événement, pile, ordre, XP1 requis, XP2 requis, capacité requise.
	local BGD_RAPPORTS = {
		{ "REPORT_EMPIRE_YIELDS",      "LOC_PARTIALSCREEN_REPORTS_YIELDS",    "ReportsList_OpenYields",          "EmpireReportsStack", 110, false, false, nil },
		{ "REPORT_EMPIRE_RESOURCES",   "LOC_PARTIALSCREEN_REPORTS_RESOURCES", "ReportsList_OpenResources",       "EmpireReportsStack", 120, false, false, nil },
		{ "REPORT_EMPIRE_CITY_STATUS", "LOC_HUD_REPORTS_TAB_CITIES",          "ReportsList_OpenCityStatus",      "EmpireReportsStack", 130, false, false, nil },
		{ "REPORT_EMPIRE_GOSSIP",      "LOC_PARTIALSCREEN_REPORTS_GOSSIP",    "ReportsList_OpenGossip",          "EmpireReportsStack", 135, false, false, "CAPABILITY_GOSSIP_REPORT" },
		{ "REPORT_EMPIRE_DEALS",       "LOC_HUD_REPORTS_TAB_DEALS",           "ReportsList_OpenDeals",           "EmpireReportsStack", 140, false, false, nil },
		{ "REPORT_EMPIRE_UNITS",       "LOC_HUD_REPORTS_TAB_UNITS",           "ReportsList_OpenUnits",           "EmpireReportsStack", 150, false, false, nil },
		{ "REPORT_EMPIRE_POLICIES",    "LOC_HUD_REPORTS_TAB_POLICIES",        "ReportsList_OpenPolicies",        "EmpireReportsStack", 160, false, false, nil },
		{ "REPORT_EMPIRE_MINORS",      "LOC_HUD_REPORTS_TAB_MINORS",          "ReportsList_OpenMinors",          "EmpireReportsStack", 170, false, false, nil },
		{ "REPORT_EMPIRE_CITIES2",     "LOC_HUD_REPORTS_TAB_CITIES2",         "ReportsList_OpenCities2",         "EmpireReportsStack", 180, false, true,  nil },
		{ "REPORT_GLOBAL_RESOURCES",   "LOC_PARTIALSCREEN_REPORTS_RESOURCES", "GlobalReportsList_OpenResources", "GlobalReportsStack", 510, false, false, "CAPABILITY_DIPLOMACY_DEALS" },
	};

	function LateInitialize()
		m_ReportButtonIM:ResetInstances();
		local parType = {};
		for _, r in ipairs(BGD_RAPPORTS) do
			parType[r[1]] = { ReportType = r[1], ButtonLabel = r[2], LuaEvent = r[3], StackID = r[4], SortOrder = r[5],
				RequiresXP1 = r[6], RequiresXP2 = r[7], GameCapability = r[8] };
		end
		if GameInfo.RLLReports ~= nil then
			for row in GameInfo.RLLReports() do
				parType[row.ReportType] = { ReportType = row.ReportType, ButtonLabel = row.ButtonLabel, LuaEvent = row.LuaEvent,
					StackID = row.StackID, SortOrder = row.SortOrder, RequiresXP1 = row.RequiresXP1, RequiresXP2 = row.RequiresXP2,
					GameCapability = row.GameCapability };
			end
		end
		local liste = {};
		for _, r in pairs(parType) do
			local ok = true;
			if r.RequiresXP1 and not (isXP1 or isXP2) then ok = false; end
			if r.RequiresXP2 and not isXP2 then ok = false; end
			if r.GameCapability ~= nil and not GameCapabilities.HasCapability(r.GameCapability) then ok = false; end
			if Controls[r.StackID] == nil then ok = false; end
			if ok then table.insert(liste, r); end
		end
		table.sort(liste, function(a, b) return a.SortOrder < b.SortOrder; end);
		for _, r in ipairs(liste) do
			local evenement = r.LuaEvent;
			AddReport(L(r.ButtonLabel), function() Close(); LuaEvents[evenement](); end, Controls[r.StackID]);
		end
	end
end
