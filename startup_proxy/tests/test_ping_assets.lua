-- Test hors jeu de src/ping_assets.lua : le mod des images et sons des signaux reste décoché.
--   lua startup_proxy/tests/test_ping_assets.lua
local path = "startup_proxy/src/ping_assets.lua"
local handle, enabled, disableCalls, configUpdates = 17, true, 0, 0
local function env()
    return {
        Modding = {
            GetModHandle=function(id)
                assert(id=="863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd")
                return handle
            end,
            GetModInfo=function(h) assert(h==handle); return {Enabled=enabled} end,
            DisableMod=function(h)
                assert(h==handle)
                enabled=false
                disableCalls=disableCalls+1
            end,
            EnableMod=function() error("le mod ne doit plus être coché") end,
        },
        GameConfiguration = {
            UpdateEnabledMods=function() configUpdates=configUpdates+1 end,
        },
        pcall=pcall, print=print, tostring=tostring,
    }
end
-- Coché par une version de test de la PR #9 : décoché, configuration remise à jour.
assert(loadfile(path,"t",env()))()
assert(not enabled and disableCalls==1 and configUpdates==1)
-- Déjà décoché : rien.
assert(loadfile(path,"t",env()))()
assert(disableCalls==1 and configUpdates==1)
-- Mod absent : rien, et pas d'erreur.
handle=nil
assert(loadfile(path,"t",env()))()
assert(disableCalls==1 and configUpdates==1)
print("images et sons des signaux : mod décoché au menu principal")
