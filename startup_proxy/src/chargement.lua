-- ===========================================================================
-- BetterGameDLLs : correctifs POL des parties locales, ajouté à la fin de LoadScreen.lua par le
-- proxy de démarrage (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_FixerPol(actif, raison)  -> état lu par le proxy de GameCore (1 ou 0)
--   Civ6Proxy_Journal(texte)           -> écrit dans RolloverProxy.log
-- Une partie qui n'est pas en réseau (solo, hotseat) ne tourne que sur cette machine : rien à
-- désynchroniser, les correctifs POL y sont actifs. En réseau, c'est le salon qui a décidé
-- (salon.lua).
-- In a network game, one case is corrected here: the host's last verdict reached this machine
-- after its lobby stopped following it (launch countdown), so the lobby concluded "off" while
-- everyone else had "on". The player configurations as loaded with the game hold that last
-- verdict: if it is active and names this machine, POL is turned on. Never turned off here.
-- ===========================================================================

-- The lobby's rule (salon.lua, BGD_PolActifs), on the launch data: the host's verdict is active, for
-- our version, carries the token of the host's own announcement, and lists our token.
local function BGD_PolAuLancement()
	local moi, hote = Network.GetLocalPlayerID(), Network.GetGameHostPlayerID();
	local cm, ch = PlayerConfigurations[moi], PlayerConfigurations[hote];
	if cm == nil or ch == nil then return false; end
	local version, _, perso = string.match(tostring((cm:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
	local _, jetonHote = string.match(tostring((ch:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
	local vVersion, vJeton, actif, liste = string.match(tostring((ch:GetValue("BGD_VERDICT"))), "^([^@#:]*)@([^@#:]*):([01]):(.*)$");
	if version == nil or jetonHote == nil or vVersion == nil then return false; end
	if actif ~= "1" or vVersion ~= version or vJeton ~= jetonHote then return false; end
	if moi == hote then return true; end
	for j in string.gmatch(liste, "[^,]+") do
		if j == perso then return true; end
	end
	return false;
end

if Civ6Proxy_FixerPol ~= nil then
	Events.LoadScreenContentReady.Add(function()
		local ok, err = pcall(function()
			if not GameConfiguration.IsNetworkMultiplayer() then
				Civ6Proxy_FixerPol(1, GameConfiguration.IsHotseat() and "hotseat" or "partie solo");
			elseif not GameConfiguration.IsPlayByCloud() and BGD_PolAuLancement() then
				Civ6Proxy_FixerPol(1, "launch data");
				if Civ6Proxy_Journal ~= nil then
					Civ6Proxy_Journal("load: POL fixes on (launch data)");
				end
			end
		end);
		if not ok and Civ6Proxy_Journal ~= nil then
			Civ6Proxy_Journal("chargement : erreur : " .. tostring(err));
		end
	end);
end
