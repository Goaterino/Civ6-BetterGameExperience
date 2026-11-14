-- One shared preference and the stock Confirm/Cancel checkpoint for both Options contexts.
local preference, checkpoint
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
    local env = {
        Controls={BGD_PingVolumeRow=row,BGD_PingVolume=slider,BGD_PingCaption=caption,ConfirmButton=confirm,
            SFXVolSlider={GetParent=function() return effects end}},
        ContextPtr={SetShutdown=function(_,fn) callbacks.shutdown=fn end},
        OnShow=function() checkpoint=preference; confirm:SetDisabled(true) end,
        UserConfiguration={GetValue=function() if preference~=nil then return preference end end,
            SetValue=function(_,value) preference=value end},
        BGD_Txt=function(_,level) return "Volume "..level end,
        LuaEvents={
            BGD_PingVolumePreview=function() previewCount=previewCount+1 end,
            BGD_PingVolumeChanged={
                Add=function(fn) callbacks.changed=fn end,
                Remove=function(fn) assert(fn==callbacks.changed); callbacks.changed=nil end,
            },
        },
        math=math, ipairs=ipairs, tonumber=tonumber,
    }
    assert(loadfile("startup_proxy/src/ping_volume.lua","t",env))()
    assert(stack.inserted and row.hidden==false)
    return env,slider,caption,confirm,callbacks,function() return previewCount end
end
local menu,slider,caption,confirm,callbacks,previews=screen()
menu.OnShow()
assert(slider.value==1 and caption.text=="Volume 100" and confirm.disabled)
slider.callback(0.49)
assert(preference==50 and slider.value==0.5 and previews()==1 and not confirm.disabled)
-- Stock Cancel restores UserConfiguration, then reopening refreshes the row.
preference=checkpoint
menu.OnShow()
assert(slider.value==1 and confirm.disabled and previews()==1)
slider.callback(0.75)
checkpoint=preference -- stock Confirm saves this checkpoint
local game,gameSlider,gameCaption,gameConfirm,gameCallbacks=screen()
game.OnShow()
assert(gameSlider.value==0.75 and gameConfirm.disabled)
preference=25 -- /pingvolume uses the same preference and sends this event
gameCallbacks.changed()
assert(gameSlider.value==0.25 and gameCaption.text=="Volume 25")
gameCallbacks.shutdown()
assert(gameCallbacks.changed==nil)
callbacks.shutdown()
