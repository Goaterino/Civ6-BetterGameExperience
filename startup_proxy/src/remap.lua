-- ===========================================================================
-- BetterGameExperience (startup_proxy/src/remap.rs): before `.mph_ui_snap`, the host loads its
-- own game (snapshot of itself); its fresh interface then says `.mph_ui_snap`. The request is
-- kept in BGE's startup DLL, which stays loaded across that load. Fallback after WAIT seconds:
-- say it at once, as MPH does.
-- ===========================================================================
do
	local WAIT = 10;
	local MAX_AGE = 600;
	local pending = nil;   -- os.clock() when this interface asked for the host's own load

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
			Log("error: " .. tostring(err) .. "; clients asked to load at once");
			pending = nil;
			pcall(Civ6Proxy_RemapTake);
			Snap();
		end
	end

	local function Tick()
		local me = Network.GetLocalPlayerID();
		if me ~= Network.GetGameHostPlayerID() or Civ6Proxy_RemapTake == nil then
			return;
		end
		if pending ~= nil then
			if Loading(me) then
				return;   -- the load has started; this interface goes away with it
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
	Events.LoadScreenClose.Add(Safely);
end
