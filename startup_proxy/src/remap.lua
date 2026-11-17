-- ===========================================================================
-- BetterGameExperience (startup_proxy/src/remap.rs): before `.mph_ui_snap`, the host loads its
-- own game (snapshot of itself); its fresh interface then says `.mph_ui_snap`. The request is
-- kept in BGE's startup DLL, which stays loaded across that load. Fallback after WAIT seconds:
-- say it at once, as MPH does.
-- ===========================================================================
do
	local WAIT = 10;
	local MAX_AGE = 600;
	local RETRIES = 3;
	local pending = nil;   -- os.clock() when this interface asked for the host's own load
	local retries = 0;     -- new requests for the host's own load, in a game started without terrain
	local full = false;    -- the next restart builds the terrain (last resort)
	local closed = false;  -- the loading screen of this interface has closed

	local function Log(text)
		if Civ6Proxy_Journal ~= nil then
			Civ6Proxy_Journal("remap : " .. text);
		end
	end

	local function Snap()
		Network.SendChat(".mph_ui_snap", -2, -1);
	end

	local function Loading(me)
		return (UI ~= nil and UI.IsResyncLoadInProgress ~= nil and UI.IsResyncLoadInProgress())
			or Network.IsPlayerResyncing(me) or Network.IsPlayerHotJoining(me);
	end

	-- This game started without terrain: the host must never play it.
	local function Light()
		return Civ6Proxy_RemapLegerActif ~= nil and Civ6Proxy_RemapLegerActif() == 1;
	end

	-- Last resort in a game without terrain: a full remap, once, without arming the light load.
	local function Full(why)
		if full then
			return;
		end
		full = true;
		pending = nil;
		pcall(Civ6Proxy_RemapTake);
		Log(why .. "; full remap");
		if OnLocalHostRestart ~= nil then
			OnLocalHostRestart();
		else
			Network.RestartGame();
		end
	end

	-- In place of MPH's `.mph_ui_snap`, on the host, once its restarted game is loaded.
	function BGD_RemapSnap()
		if Civ6Proxy_RemapMark == nil or Civ6Proxy_RemapTake == nil then
			Snap();
			return;
		end
		local ok, err = pcall(function()
			Civ6Proxy_RemapMark();
			pending = os.clock();
			Log("the host loads its own game before the clients load it (Network.RequestSnapshot)");
			Network.RequestSnapshot();
		end);
		if not ok then
			if Light() then
				Full("error in a game without terrain: " .. tostring(err));
				return;
			end
			Log("error: " .. tostring(err) .. "; clients asked to load at once");
			pending = nil;
			pcall(Civ6Proxy_RemapTake);
			Snap();
		end
	end

	-- Before MPH's `Network.RestartGame()`: the host's first load may skip the 3D terrain, since the
	-- host reloads its own save at once (startup_proxy/src/remap_leger.rs). Host of a network game
	-- only: `.mph_ui_remap_execute` runs on every machine.
	function BGD_RemapLeger()
		if full or Civ6Proxy_RemapLeger == nil or Civ6Proxy_RemapTake == nil then
			return;
		end
		pcall(function()
			if Network.GetLocalPlayerID() == Network.GetGameHostPlayerID() and GameConfiguration.IsNetworkMultiplayer() then
				Civ6Proxy_RemapLeger();
			end
		end);
	end

	local function Tick()
		local me = Network.GetLocalPlayerID();
		if me ~= Network.GetGameHostPlayerID() or Civ6Proxy_RemapTake == nil then
			return;
		end
		if full then
			return;   -- a full remap is under way
		end
		if pending == nil and closed and Light() and not Loading(me) then
			-- What MPH's OnRefresh does before its snapshot (it waits 1 s, the light load is
			-- quicker): the host's save must not say "Y", or the reloaded game asks again.
			Log("game started without terrain and no reload asked: the host loads its own game");
			if GameConfiguration.GetValue("GAME_HOST_IS_JUST_RELOADING") == "Y" then
				GameConfiguration.SetValue("GAME_HOST_IS_JUST_RELOADING", "N");
				Network.BroadcastGameConfig();
			end
			BGD_RemapSnap();
			return;
		end
		if pending ~= nil then
			if Loading(me) then
				return;   -- the load has started; this interface goes away with it
			end
			if os.clock() - pending > WAIT and Light() then
				if retries < RETRIES then
					retries = retries + 1;
					pending = os.clock();
					Log("the host's own load did not start within " .. WAIT .. " s in a game without terrain; asked again (" .. retries .. ")");
					pcall(Network.RequestSnapshot);
				else
					Full("the host's own load never started in a game without terrain");
				end
				return;
			end
			if os.clock() - pending > WAIT then
				pending = nil;
				Civ6Proxy_RemapTake();
				Log("the host's own load did not start within " .. WAIT .. " s; clients asked to load at once, as MPH does");
				Snap();
			end
			return;
		end
		-- A fresh interface after the host's own load: now the clients load the same game.
		if Loading(me) then
			return;
		end
		local age = Civ6Proxy_RemapTake();
		if age == nil or age < 0 then
			return;
		end
		if age <= MAX_AGE then
			Log("the host has loaded its own game; clients asked to load the same game");
			Snap();
		else
			Log("request older than " .. MAX_AGE .. " s ignored");
		end
	end

	local function Safely()
		local ok, err = pcall(Tick);
		if not ok then
			Log("error: " .. tostring(err));
		end
	end
	Events.GameCoreEventPublishComplete.Add(Safely);
	Events.LoadScreenClose.Add(function()
		closed = true;
		Safely();
	end);
end
