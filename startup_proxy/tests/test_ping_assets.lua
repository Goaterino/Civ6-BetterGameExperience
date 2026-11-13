-- Existing profiles and mod groups acquire the packaged media automatically.
local path = "startup_proxy/src/ping_assets.lua"
local handle, enabled, enabledCalls, configUpdates = 17, false, 0, 0
local changed
local env = {
    Modding = {
        GetModHandle=function(id)
            assert(id=="863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd")
            return handle
        end,
        GetModInfo=function(h) assert(h==handle); return {Enabled=enabled} end,
        EnableMod=function(h,withDependencies)
            assert(h==handle and withDependencies)
            enabled=true
            enabledCalls=enabledCalls+1
        end,
    },
    GameConfiguration = {
        UpdateEnabledMods=function() configUpdates=configUpdates+1 end,
    },
    Events = {
        FinishedGameplayContentConfigure={Add=function(fn) changed=fn end},
    },
}
assert(loadfile(path,"t",env))()
assert(enabled and enabledCalls==1 and configUpdates==1)
changed()
assert(enabledCalls==1 and configUpdates==1)
enabled=false
changed()
assert(enabled and enabledCalls==2 and configUpdates==2)
handle=nil
changed()
assert(enabledCalls==2 and configUpdates==2)
