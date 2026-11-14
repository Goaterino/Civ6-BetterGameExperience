-- This local media component ships with BGE. Register it automatically for every
-- mod group, including groups that existed before the player installed this update.
do
    local function enablePingAssets()
        local handle = Modding.GetModHandle("863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd")
        if not handle then return end
        local info = Modding.GetModInfo(handle)
        if info and not info.Enabled then
            Modding.EnableMod(handle, true)
            GameConfiguration.UpdateEnabledMods()
        end
    end
    Events.FinishedGameplayContentConfigure.Add(enablePingAssets)
    enablePingAssets()
end
