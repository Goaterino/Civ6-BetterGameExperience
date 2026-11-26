-- ===========================================================================
-- BetterGameDLLs : correctifs POL des parties locales, ajouté à la fin de LoadScreen.lua par le
-- proxy de démarrage (startup_proxy/src/salon.rs). Fonctions natives posées par le proxy :
--   Civ6Proxy_FixerPol(actif, raison)  -> état lu par le proxy de GameCore (1 ou 0)
--   Civ6Proxy_FixerDeparts(actif, raison) -> the same for the drop protection (salon.lua)
--   Civ6Proxy_NiveauDeparts()          -> this proxy's drop protection level
--   Civ6Proxy_Journal(texte)           -> écrit dans RolloverProxy.log
--   Civ6Proxy_PartieReseau(actif)      -> 1 pour une partie en réseau en direct (ni solo, ni hotseat,
--                                         ni PlayByCloud), lu par le proxy de GameCore
-- Une partie qui n'est pas en réseau (solo, hotseat) ne tourne que sur cette machine : rien à
-- désynchroniser, les correctifs POL y sont actifs. En réseau, c'est le salon qui a décidé
-- (salon.lua).
-- In a network game, one case is corrected here: the host's last verdict reached this machine
-- after its lobby stopped following it (launch countdown), so the lobby concluded "off" while
-- everyone else had "on". The player configurations as loaded with the game hold that last
-- verdict: if it is active and names this machine, POL is turned on. Never turned off here. The
-- same for the host's drop protection verdict.
-- Checked when the game is loaded and again when the load screen closes: a verdict received during
-- the load (a player who joined a game in progress) is applied before the player plays.
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

-- The lobby's rule for the drop protection (salon.lua, BGD_DepartsActifs), on the launch data: the
-- host's drop verdict is active, of our level, carries the token of the host's own announcement, and
-- the host's verdict lists our token. Our version does not matter.
local function BGD_DepartsAuLancement()
	if Civ6Proxy_FixerDeparts == nil or Civ6Proxy_NiveauDeparts == nil then return false; end
	local moi, hote = Network.GetLocalPlayerID(), Network.GetGameHostPlayerID();
	local cm, ch = PlayerConfigurations[moi], PlayerConfigurations[hote];
	if cm == nil or ch == nil then return false; end
	local _, _, perso = string.match(tostring((cm:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
	local _, jetonHote = string.match(tostring((ch:GetValue("BGD_VERSION"))), "^([^@#:]*)@([^@#:]*)#([^@#:,]*)$");
	local niveau, dJeton, actif = string.match(tostring((ch:GetValue("BGD_VERDICT_DEPARTS"))), "^(%d+)@([^@#:]*):([01])$");
	if perso == nil or jetonHote == nil or niveau == nil then return false; end
	if actif ~= "1" or niveau ~= string.format("%d", Civ6Proxy_NiveauDeparts()) or dJeton ~= jetonHote then return false; end
	if moi == hote then return true; end
	local _, vJeton, _, liste = string.match(tostring((ch:GetValue("BGD_VERDICT"))), "^([^@#:]*)@([^@#:]*):([01]):(.*)$");
	if vJeton ~= jetonHote then return false; end
	for j in string.gmatch(liste, "[^,]+") do
		if j == perso then return true; end
	end
	return false;
end

if Civ6Proxy_FixerPol ~= nil then
	local dit, ditDeparts = false, false;
	local function BGD_PolDuChargement()
		local ok, err = pcall(function()
			if Civ6Proxy_PartieReseau ~= nil then
				local reseau = GameConfiguration.IsNetworkMultiplayer() and not GameConfiguration.IsPlayByCloud()
					and not GameConfiguration.IsHotseat();
				Civ6Proxy_PartieReseau(reseau and 1 or 0);
			end
			if not GameConfiguration.IsNetworkMultiplayer() then
				Civ6Proxy_FixerPol(1, GameConfiguration.IsHotseat() and "hotseat" or "partie solo");
			elseif not GameConfiguration.IsPlayByCloud() then
				if BGD_PolAuLancement() then
					Civ6Proxy_FixerPol(1, "launch data");
					if Civ6Proxy_Journal ~= nil and not dit then
						dit = true;
						Civ6Proxy_Journal("load: POL fixes on (launch data)");
					end
				end
				if BGD_DepartsAuLancement() then
					Civ6Proxy_FixerDeparts(1, "launch data");
					if Civ6Proxy_Journal ~= nil and not ditDeparts then
						ditDeparts = true;
						Civ6Proxy_Journal("load: drop protection on (launch data)");
					end
				end
			end
		end);
		if not ok and Civ6Proxy_Journal ~= nil then
			Civ6Proxy_Journal("chargement : erreur : " .. tostring(err));
		end
	end
	Events.LoadScreenContentReady.Add(function()
		dit, ditDeparts = false, false;
		BGD_PolDuChargement();
	end);
	Events.LoadScreenClose.Add(BGD_PolDuChargement);
end
