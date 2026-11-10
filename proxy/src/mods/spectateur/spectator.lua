------------------------------------------------------------------------------
--	FILE:	 Spectator.lua
--	AUTHOR:  D. / Jack The Narrator
--	PURPOSE: Gameplay script - Add an Spectator to the game in a non-disruptive manner
-------------------------------------------------------------------------------
-- logs
-- v0.1 	added Observer Support
-- v0.2 	added Mod basic UI
--		Spectator players now default movement to quick combat/movement to not delay the MP games
-- v0.3 	New Implementation
-- v0.4 	Congress integrated
-- v0.41	Fixed a typo in the congress code
-- v0.42 	Now can follow the scores and process of players
-- v0.43	Now with correct GS banner
-- v0.44	fix
-- v0.5		Now includes City-State Panel
-- v0.6		Now have icons
--		Better unit handling
--		Permanent visibility on cities
--		Can see unit flgs during the turn changes
-- v0.7		working on one way visiblity / GetOwner plot
--		Introduce jump observer
-- v0.8		Observer now goes back to the previous player after the tunr completes
--		Add resources icons again (different technique)
-- v0.9		Add Tech/Civic View
--		Kill the delegation and other stupid auto popup from aI to the Observer
--		Recall Observer onpromotion and unitkilled event (test)
--		Added strategic resources / yields
-- v0.91	Corrected toppanel glitches
--		Added the great work panel
--		banner production and food growth
-- v0.92	Corrected more glitcjes
--		Governement panel
--		Governor panel
--		Added hook for world tracker to update quickly	
-- v0.93	Moved UI Observer storage to GameConfiguration for easier bouncing on the UI side
-- v0.94	Added Unit support
--		Added diplomacy info when switching players
--		Added era score mouse over on diplomacy ribbon
-- v0.95	Dedications added to leader tootip
--		Cleaned up the Observer view
--		Working spy screen
--		Put the view point
-- v0.96	Display loyalty (to do)
--		Detailled scores on top
--		Now would pick the name not the coded code for leaders/cities
--		Goody Hut notifications
--		Great People Notifications
--		First important unit notifications
--		Added Tech #, Civivc #, unit # in the tooltip to help comparing
-- v0.97	Improve chat visibility while being player 1000
--		Extended visibility to adjacent tiles (so that 3d models don t disappear close to the city border)
--		Added Envoy supports in Top Panel
--		to do add ranking on top panel?
--		to do improve/add more notifications
--		Imporved resources / flags refresh
--		Added Multi-Observer supports
--		Add reports supports (bugged)
--		Added Graphs Reports from EMR
-- v0.98	Fixed the techtree
--		Fixed a bug notifying GP to non spectators
-- v0.99	Fixed a bug with game with open slots
-- v1.00	Added A tick-box for Single Player Auto-End Turn
--		Spectator can now votes in congress (temp fix) Need Firaxis solution
-- v1.01	Removed some trails to clean up the lua.log
--		Change the logic so that spectator votes can be manualy inputed
-- v1.02	Improve Ribbon sorting, tied it to MP Helper
-- v1.03	Correctly Reanchor Congress
--		Display pings & Lag when used with MP Helper
--		Fixed tooltip for all observer
--		Added Nukes to the HUD
-- v1.04	PLayers would have their portrait on the left even in Teamers
--			Spec now get 4 tabs triggered by mouseover on their spec panel
-- v1.05	Added a Yield button
--			Several minors visual tweaks
--			When used with Multiplayer Helper you no longer need to allow duplicate leader for multi-observers
--			Tech/Score view is changing every 20 secs (from 5 previously)
--			Auto-end of turn for spec can be changed in-game
--			Code cleanup
-- v1.06	Adjusted the Score screen for Religious and Cultural Victory to ignore the Observers
--		Added MPH hooks to display victory screen if MPH is used
-- v1.07	Emergency New Frontier Pass path
--		More fixes will come later
-- v1.08	Improved the Diplomacy Ribbon code
--		Spectator now sees the Civ Colour/Logo to help viewers identify the teams
-- v1.09	Ruleset Based
-- v1.10	Remove Ruleset to allow NFP / Ethiopia compatibility.
--		Only top 4 slots can be Observers
-- v1.11	Support for the Gaul Patch
-- v1.12	Added Tournament support with MPH
--		Fixed some database errors and other minors glitches	
-- v1.14	Compatible with Portugal Patch
-- v1.15 	Compatible with Balance Patch
--		Search Function works as an Observer

print("BSM Game Script")
---------------------------------------------------------------------------------
ExposedMembers.LuaEvents = LuaEvents

local bLaunch = false;
local spec_num = 0
local g_version = "v1.27"


function Spec_Script_Init()
		-- Une nouvelle partie se traite tout de suite : c'est la que les unites du spectateur
		-- doivent disparaitre. Un rechargement, lui, doit attendre le premier debut de tour, car
		-- les objets joueurs ne sont pas encore construits et les manipuler fait tomber le jeu.
		-- SPEC_INIT, pose a la fin du premier traitement, est notre marqueur : absent sur une
		-- nouvelle partie, present dans une sauvegarde rechargee.
		if Game:GetProperty("SPEC_INIT") == true and not Spec_ApresChargement then
			Spec_ApresChargement = true
			local function ReprendreApresChargement()
				GameEvents.OnGameTurnStarted.Remove(ReprendreApresChargement)
				Spec_Script_Init()
			end
			GameEvents.OnGameTurnStarted.Add(ReprendreApresChargement)
			print("Spectator: chargement detecte, traitement differe au premier debut de tour")
			return
		end
		local currentTurn = Game.GetCurrentGameTurn();
		local bSpectator = false;
		local iNumMajCivs = 0;
		local MajorList = {};
		local spec_i = nil;
		spec_num = 0
		iNumMajCivs = PlayerManager.GetAliveMajorsCount();
		MajorList = PlayerManager.GetAliveMajorIDs();
		local MajorListNoSpec = {}
		if Game:GetProperty("TO_CHECK")~=nil then
			MajorListNoSpec = Game:GetProperty("TO_CHECK")
			print(ArrayStrUtil(MajorListNoSpec))
		else
			MajorListNoSpec = PlayerManager.GetAliveMajorIDs()
			print(ArrayStrUtil(MajorListNoSpec))
			--Game:SetProperty("TO_CHECK",MajorList)
			filterArray(MajorListNoSpec, function(t,i,j) local v = t[i] return PlayerConfigurations[v]:GetLeaderTypeName()~="LEADER_SPECTATOR" end)
			print(ArrayStrUtil(MajorListNoSpec))
		end
		Game:SetProperty("TO_CHECK", MajorListNoSpec)
		print("Turn ",currentTurn,": Local player Leader:", PlayerConfigurations[Game.GetLocalPlayer()]:GetLeaderTypeName()," ID:",Game.GetLocalPlayer());
		-- Detection elargie : les majeurs, plus les mineurs. Un spectateur reclasse en mineur
		-- (niveau SPECTATOR) n'est plus dans les listes de majeurs ; sans ce bloc il n'est jamais
		-- vu, et son nettoyage ne tourne pas.
		local DetectionList = {}
		for i = 1, iNumMajCivs do
			DetectionList[#DetectionList + 1] = MajorList[i]
		end
		-- Tous les emplacements configures, sans appeler GetAliveMinorIDs : cette liste native
		-- n'est pas encore construite au chargement d'une sauvegarde.
		for pid = 0, 63 do
			if PlayerConfigurations[pid] ~= nil then
				local bDup = false
				for _, existing in ipairs(DetectionList) do
					if existing == pid then bDup = true; break end
				end
				if not bDup then
					DetectionList[#DetectionList + 1] = pid
				end
			end
		end

		for i = 1, #DetectionList do
			local sPlayerLeaderName = PlayerConfigurations[DetectionList[i]]:GetLeaderTypeName();
			if (sPlayerLeaderName=="LEADER_SPECTATOR") then
				print("Turn ",currentTurn,": A Spectator Civ Has Been Detected! ID:",DetectionList[i]);
				spec_i = DetectionList[i];
				spec_num = spec_num + 1;
				local pPlayer = Players[spec_i];
				local pVis = PlayersVisibility[spec_i];
				print(GameConfiguration.GetValue("GAME_NO_BARBARIANS"))
				if (pVis ~= nil) then
					for iPlotIndex = 0, Map.GetPlotCount()-1, 1 do
						local pPlot = Map.GetPlotByIndex(iPlotIndex)
						if (pPlot:IsNaturalWonder() == false) then
							pVis:ChangeVisibilityCount(iPlotIndex, 0); 
						end
					end
				end
				if (bLaunch == true) then
 					if (pPlayer ~= nil) then
    						local pPlayerUnits:table = pPlayer:GetUnits();
						print("Deleting units on init");
    						if (pPlayerUnits) then
      							for i, pUnit in pPlayerUnits:Members() do
          							pPlayer:GetUnits():Destroy(pUnit)
      							end
    						end
  					end
				end
			
				Game:SetProperty("SPEC_ID",spec_i)
				Game:SetProperty("SPEC_ID_"..spec_num,spec_i)
				Game:SetProperty("SPEC_INIT",true)
 	 		end

		end

		if spec_num > 0 then
			Game:SetProperty("SPEC_NUM",spec_num)
		end

		print("Turn ",currentTurn,": Grant Access to other civs");
		if ( Game:GetProperty("SPEC_NUM") ~= nil and bLaunch == true) then
			for k = 1, spec_num do
			for i = 1, iNumMajCivs do
				local pPlayerCulture:table = Players[MajorList[i]]:GetCulture()
				if ( i~= Game:GetProperty("SPEC_ID_"..k)) then
					Players[Game:GetProperty("SPEC_ID_"..k)]:GetDiplomacy():SetHasMet(MajorList[i])
					--Players[Game:GetProperty("SPEC_ID_"..k)]:GetDiplomacy():SetHasDelegationAt(MajorList[i],true)
					--Players[Game:GetProperty("SPEC_ID_"..k)]:GetDiplomacy():SetHasEmbassyAt(MajorList[i],true)
					Players[Game:GetProperty("SPEC_ID_"..k)]:GetDiplomacy():SetVisibilityOn(MajorList[i],true)
					--Players[Game:GetProperty("SPEC_ID")]:GetDiplomacy():SetPermanentAlliance(MajorList[i])
					Players[Game:GetProperty("SPEC_ID_"..k)]:GetDiplomacy():RecheckVisibilityOnAll()
					--print(Players[i]:GetEras():GetEra())		
				end
				--print("GameInfo.Civics[DIPLOMATIC_SERVICE].Index",GameInfo.Civics["CIVIC_DIPLOMATIC_SERVICE"].Index)
				--pPlayerCulture:ReverseBoost(GameInfo.Civics["CIVIC_DIPLOMATIC_SERVICE"].Index)
				--local pPlayerTechs:table = Players[MajorList[i]]:GetTechs();
				--pPlayerTechs:ReverseBoost(GameInfo.Technologies["TECH_WRITING"].Index)
			end
			pPlayerCulture = Players[Game:GetProperty("SPEC_ID_"..k)]:GetCulture()
			pPlayerCulture:SetCivic(GameInfo.Civics["CIVIC_CODE_OF_LAWS"].Index,true)
			end
		end
		
		
end

function CleanBoost()
	if ( Game:GetProperty("SPEC_ID") ~= nil ) then
		local iNumMajCivs = PlayerManager.GetAliveMajorsCount();
		local MajorList = PlayerManager.GetAliveMajorIDs();
		for i = 1, iNumMajCivs do
			local pPlayerTechs:table = Players[MajorList[i]]:GetTechs();
			pPlayerTechs:ReverseBoost(GameInfo.Technologies["TECH_WRITING"].Index)
		end
	end
end

--LuaEvents.UICleanBoost.Add ( CleanBoost );

function DoObserver(id)
	if ( Game:GetProperty("SPEC_NUM") ~= nil ) then
		for k =1, Game:GetProperty("SPEC_NUM") do
			if ( Game.GetLocalPlayer() == Game:GetProperty("SPEC_ID_"..k)) then
				--print("Turning to observer")
				PlayerManager.SetLocalObserverTo(1000);
			end
		end
	end
end

LuaEvents.UIDoObserver.Add ( DoObserver );

function DoObserverPlayer(id:number)
	local currentTurn = Game.GetCurrentGameTurn();
	local bspec = false
	if ( Game:GetProperty("SPEC_NUM") ~= nil ) then
		for k =1, Game:GetProperty("SPEC_NUM") do
			if Game:GetProperty("SPEC_ID_"..k) == id then
				bspec = true
			end
		end
		if bspec == false then
			PlayerManager.SetLocalObserverTo(id);
			else		
			PlayerManager.SetLocalObserverTo(1000)
		end
		Game:SetProperty("SPEC_LAST_ID",id)
	end
	--print("Turned to Observer: Turn:",currentTurn,"PlayerID:",id)
	
end

LuaEvents.UIDoObserverPlayer.Add ( DoObserverPlayer );


function UndoObserver(id)
	if ( Game:GetProperty("SPEC_NUM") ~= nil ) then
		for k =1, Game:GetProperty("SPEC_NUM") do
			if ( Game.GetLocalPlayer() == Game:GetProperty("SPEC_ID_"..k)) then
				--print("Turning to spectator")
				PlayerManager.SetLocalObserverTo(Game.GetLocalPlayer());
			end
		end
	end
end

LuaEvents.UIUndoObserver.Add ( UndoObserver );



-------------------------------------------------------------------------------
function OnCityBuilt( playerID: number, cityID : number, cityX : number, cityY : number )	
	--print("New city found by player", playerID, "X",cityX,"Y",cityY)
	if ( Game:GetProperty("SPEC_NUM") ~= nil ) then
		for k =1, Game:GetProperty("SPEC_NUM") do
			if (Game:GetProperty("SPEC_ID_"..k) ~= nil) then
				local pVis = PlayersVisibility[Game:GetProperty("SPEC_ID_"..k)];
				for iPlotIndex = 0, Map.GetPlotCount()-1, 1 do
					local pPlot = Map.GetPlotByIndex(iPlotIndex)
					local pPlot_Owner = pPlot:GetOwner()
					if (pPlot_Owner ~=nil ) then
						if (Players[pPlot_Owner] ~= nil) then
							if (Players[pPlot_Owner]:IsMajor() == true) then
								pVis:ChangeVisibilityCount(iPlotIndex, 2); 
								for j = 0, 5 do
									local pAdjacentPlot = Map.GetAdjacentPlot(pPlot:GetX(),pPlot:GetY(),j)
									if pAdjacentPlot ~= nil then
										--print("gameplay",pAdjacentPlot:GetIndex())
										pVis:ChangeVisibilityCount(pAdjacentPlot:GetIndex(),2);
									end
								end
							end
						end
					end
				end
			end
		end
	end
end



function OnTurnStarted()
	local currentTurn = Game.GetCurrentGameTurn();
	local tmpspecid = nil
	local tmpspeclast = nil
	local bspec = false
	if Game:GetProperty("SPEC_ID") ~= nil then
		tmpspecid = Game:GetProperty("SPEC_ID")
		if Game:GetProperty("SPEC_LAST_ID") ~= nil then
			tmpspeclast =  Game:GetProperty("SPEC_LAST_ID")
			if  Game:GetProperty("SPEC_LAST_ID") == Game.GetLocalPlayer() then
				bspec = true
			end
		end
	end
	Game:SetProperty("SPEC_LAST_ID",id)
	--print("Turned to Observer: Turn:",currentTurn,"Spec ID:",tmpspecid,"Last Observed ID:",tmpspeclast,"IsLocalObserver?",bspec)
	
end

function OnPlayerTurnActivated()

end

function OnHostWritingUpdate(triggerplayerID, kParameters)
	print("Writing boost triggered in Gameplay from "..tostring(triggerplayerID).." UI Host")
	GameEvents.ApplyWritingBoost.Call(triggerplayerID, kParameters)
end

LuaEvents.HostWritingUpdate.Add(OnHostWritingUpdate)

function OnHostPopCheckList(triggerplayerID, kParameters)
	print("Pop CheckList triggered in Gameplay from "..tostring(triggerplayerID).." UI Host")
	GameEvents.PopCheckList.Call(triggerplayerID, kParameters)
end

LuaEvents.HostPopCheckList.Add(OnHostPopCheckList)

function OnApplyWritingBoost(triggerplayerID, kParameters)
	print("Writing boost triggered in Gameplay from "..tostring(triggerplayerID).." UI Request")			
	for i,playerID in ipairs(kParameters.value) do
		TriggerWritingBoost(playerID, false)
	end
end

function OnGameTurnStartedRecalculate(playerID)
	print("OnGameTurnStartedRecalculate triggered")
	local MajorList:table = {}
	local iNumMajCivs = PlayerManager.GetAliveMajorsCount();
	MajorList = PlayerManager.GetAliveMajorIDs()
	local list = Game:GetProperty("TO_CHECK")
	if list == nil or list == {} then
		for i=1,iNumMajCivs do
			print((Players[MajorList[i]]:GetTechs():HasBoostBeenTriggered(GameInfo.Technologies["TECH_WRITING"].Index) or Players[MajorList[i]]:GetTechs():HasTech(GameInfo.Technologies["TECH_WRITING"].Index)))
		end
		LuaEvents.HostWritingUpdate.Remove(OnHostWritingUpdate)
		print("HostWriting removed")
		GameEvents.ApplyWritingBoost.Remove(OnApplyWritingBoost)
		print("ApplyWritingBoost removed")
		LuaEvents.HostPopCheckList.Remove(OnHostPopCheckList)
		print("OnHostPopCheckList removed")
		GameEvents.PopCheckList.Remove(OnPopCheckList)
		print("OnPopCheckList removed")
		ExposedMembers.FreeUIHooks()
		print("UIHooks removed")
		GameEvents.OnGameTurnStarted.Remove(OnGameTurnStartedRecalculate)
		print("OnGameTurnStartedRecalculate removed")
		return
	end 
	
	for i, pID in ipairs(list) do
		local pPlayer = Players[pID] 
		for j = 1, iNumMajCivs do
			if PlayerConfigurations[MajorList[j]]:GetLeaderTypeName() ~= "LEADER_SPECTATOR" then
				if pPlayer:GetDiplomacy():HasMet(MajorList[j]) then
					TriggerWritingBoost(pID, true)
					break
				end
			end
		end
	end
end

function TriggerWritingBoost(playerID, bCheck)
	local bApplyNoBoost = false
	local pPlayer = Players[playerID]
	local pTechs = pPlayer:GetTechs()
	local result = (pTechs:HasBoostBeenTriggered(GameInfo.Technologies["TECH_WRITING"].Index) or pTechs:HasTech(GameInfo.Technologies["TECH_WRITING"].Index))
	if bCheck == true then
		if result then
			bApplyNoBoost = true
		end
	end
	if not bApplyNoBoost then
		pPlayer:AttachModifierByID("BSM_WRITING_BOOST")
		print("Writing boost Applied to "..tostring(PlayerConfigurations[playerID]:GetCivilizationTypeName()).." with PlayerID "..tostring(playerID))
		result = pTechs:HasBoostBeenTriggered(GameInfo.Technologies["TECH_WRITING"].Index)
		print("Test Boost Application. Result: ", result)
	end
end

function OnPopCheckList(triggerplayerID, kParameters)
	print("OnPopCheckList Started")
	local list = {}
	list = Game:GetProperty("TO_CHECK")
	if list==nil then
		return
	end
	if #list==0 then
		print("No Elements?") 
		return 
	end
	print(ArrayStrUtil(list))
	print("Elements => Pass")
	for i, playerID in ipairs(kParameters.value) do
		if IDtoPos(list, playerID) == false then
			print("Already Removed")
			return
		end
		print("Remove from list start")
		print(ArrayStrUtil(list))
		filterArray(list, function(t,i,j) local v = t[i] return v~= playerID end)
		print(ArrayStrUtil(list))
		if #list > 0 then
			Game:SetProperty("TO_CHECK", list)
		else
			Game:SetProperty("TO_CHECK", nil)
		end
	end
end

--Utils--
function filterArray(t, fnKeep)
    local j, n = 1, #t;

    for i=1,n do
        if (fnKeep(t, i, j)) then
            if (i ~= j) then
                t[j] = t[i];
                t[i] = nil;
            end
            j = j + 1;
        else
            t[i] = nil;
        end
    end

    return t;
end

function IDtoPos(List, SearchItem)
	if SearchItem==nil then
		return
	end
	for i, item in ipairs(List) do
		if item == SearchItem then
			return i;
		end
	end
	print("IDtoPos: item not found")
	return false
end

function ArrayStrUtil(list)
	local debugstr = ""
	for i,item in ipairs(list) do
		debugstr = debugstr..tostring(item)..", "
	end
	return debugstr
end
--------------------------------------------------------------------------------

function Initialize()
	print("-- Init D. Better Spectator Mod "..g_version.." --");
	local currentTurn = Game.GetCurrentGameTurn();
	bLaunch = false;

	if currentTurn == GameConfiguration.GetStartTurn() then
		bLaunch = true;
	end
	--print("Test G1", tostring(Players[0]:GetTechs():HasBoostBeenTriggered(GameInfo.Technologies["TECH_WRITING"].Index,true)))
	--print("Test G2", tostring(Players[0]:GetTechs():HasBoostBeenTriggered(GameInfo.Technologies["TECH_WRITING"].Index,false)))
	--print("Test G3", tostring(Players[0]:GetTechs():HasBoostBeenTriggered(GameInfo.Technologies["TECH_WRITING"].Index)))
	GameEvents.CityBuilt.Add(OnCityBuilt )
	GameEvents.ApplyWritingBoost.Add(OnApplyWritingBoost)
	print("Writing boost handler attached")
	GameEvents.OnGameTurnStarted.Add(OnGameTurnStartedRecalculate)
	print("OnGameTurnStartedRecalculate Hook Added")
	GameEvents.PopCheckList.Add(OnPopCheckList)
	print("OnPopCheckList Hook Added")
	Spec_Script_Init();
end


Initialize();
