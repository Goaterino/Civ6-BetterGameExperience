-- lua startup_proxy/tests/test_ping_volume.lua
-- One shared UserConfiguration and the stock Confirm/Cancel checkpoint for both Options
-- contexts, plus rollover_proxy.cfg, which survives game loads (prefs.lua).
local function read(path)
    local f=assert(io.open(path,"rb")); local s=f:read("*a"); f:close(); return s
end
local p="proxy/src/mods/pings/"
local block="(function()\n"..read(p.."catalog.lua").."\n"..read(p.."bindings.lua").."\n"..read(p.."prefs.lua")
    .."\n"..read("startup_proxy/src/ping_volume.lua").."\nend)()\n"

local config, checkpoint = {}, {}
local file = "actif=on\r\n"
local writes = 0
local function copy(t) local c={} for k,v in pairs(t) do c[k]=v end return c end
local function screen()
    local row, slider, caption, confirm = {}, {}, {}, {}
    local callbacks, previewCount = {}, 0
    local siblings = {{}, {}, {}, {}}
    local effects = siblings[3]
    local stack = {}
    function stack:GetChildren() return siblings end
    function stack:AddChildAtIndex(child,index)
        assert(child==row and index==3)
        self.inserted=true
    end
    function stack:CalculateSize() end
    function stack:ReprocessAnchoring() end
    function effects:GetParent() return stack end
    function row:ChangeParent(parent) assert(parent==stack) end
    function row:SetHide(hidden) self.hidden=hidden end
    function slider:SetValue(value)
        self.value=value
        if self.callback then self.callback(value) end
    end
    function slider:RegisterSliderCallback(callback) self.callback=callback end
    function caption:SetText(text) self.text=text end
    function confirm:SetDisabled(value) self.disabled=value end
    function confirm:RegisterCallback(_,fn) self.click=fn end
    local env = {
        Controls={BGD_PingVolumeRow=row,BGD_PingVolume=slider,BGD_PingCaption=caption,ConfirmButton=confirm,
            SFXVolSlider={GetParent=function() return effects end}},
        ContextPtr={SetShutdown=function(_,fn) callbacks.shutdown=fn end},
        Mouse={eLClick=1}, Keys={X=24, ["1"]=28, ["2"]=29, ["3"]=30, ["4"]=31},
        OnShow=function() checkpoint=copy(config); confirm:SetDisabled(true) end,
        OnConfirm=function() checkpoint=copy(config) end,
        OnCancel=function() config=copy(checkpoint) end,
        UserConfiguration={GetValue=function(k) return config[k] end, SetValue=function(k,v) config[k]=v end},
        Civ6Proxy_LireReglages=function() return file, 1, 0 end,
        Civ6Proxy_EcrireReglages=function(lines)
            writes=writes+1
            for k, v in lines:gmatch("([%w_]+)=(%d+)") do
                local replaced
                file=file:gsub(k.."=%d+", function() replaced=true; return k.."="..v end)
                if not replaced then file=file..k.."="..v.."\r\n" end
            end
            return 1, 0
        end,
        BGD_Txt=function(_,level) return "Volume "..level end,
        LuaEvents={
            BGD_PingVolumePreview=function() previewCount=previewCount+1 end,
            BGD_PingVolumeChanged={
                Add=function(fn) callbacks.changed=fn end,
                Remove=function(fn) assert(fn==callbacks.changed); callbacks.changed=nil end,
            },
        },
        math=math, ipairs=ipairs, pairs=pairs, tonumber=tonumber, tostring=tostring, pcall=pcall, table=table, type=type, string=string,
    }
    assert(load(block,"@ping_options","t",env))()
    assert(stack.inserted and row.hidden==false)
    return env,slider,caption,confirm,callbacks,function() return previewCount end
end
local menu,slider,caption,confirm,callbacks,previews=screen()
menu.OnShow()
assert(slider.value==1 and caption.text=="Volume 100" and confirm.disabled)
slider.callback(0.49)
assert(config.TeamPings_Volume==50 and slider.value==0.5 and previews()==1 and not confirm.disabled)
-- Stock Cancel restores UserConfiguration and writes nothing; reopening refreshes the row.
menu.OnCancel()
menu.OnShow()
assert(slider.value==1 and confirm.disabled and previews()==1 and writes==0)
slider.callback(0.75)
confirm.click()
assert(file:find("ping_volume=75",1,true) and file:find("actif=on",1,true) and writes==1)
local game,gameSlider,gameCaption,gameConfirm,gameCallbacks=screen()
game.OnShow()
assert(gameSlider.value==0.75 and gameConfirm.disabled)
config.TeamPings_Volume=25 -- /pingvolume uses the same preference and sends this event
gameCallbacks.changed()
assert(gameSlider.value==0.25 and gameCaption.text=="Volume 25")
gameCallbacks.shutdown()
assert(gameCallbacks.changed==nil)

-- A game load (MPH's remap) empties UserConfiguration: the file brings the volume and
-- the chords back, before the stock checkpoint.
config.BGD_Key_TeamPingWheel_1=256+11
confirm.click()
assert(file:find("ping_key_TeamPingWheel_1=267",1,true) and file:find("ping_volume=25",1,true))
config={}
menu.OnShow()
assert(config.TeamPings_Volume==25 and config.BGD_Key_TeamPingWheel_1==267 and checkpoint.TeamPings_Volume==25)
assert(slider.value==0.25)
-- A value set in this session is never overwritten by the file.
config={TeamPings_Volume=0}
menu.OnShow()
assert(config.TeamPings_Volume==0 and slider.value==0)
-- A fresh context (in-game Options after the load) restores as it loads.
config={}
local after,afterSlider=screen()
assert(config.TeamPings_Volume==25 and config.BGD_Key_TeamPingWheel_1==267)
after.OnShow()
assert(afterSlider.value==0.25)
callbacks.shutdown()
print("Ping volume options and saved preferences passed")
