-- Shared Options context, present both at the main menu and in a match.
do
    local row, slider = Controls.BGD_PingVolumeRow, Controls.BGD_PingVolume
    if row and slider and Controls.SFXVolSlider then
        local refreshing = false
        local function percent()
            local value = UserConfiguration.GetValue("TeamPings_Volume")
            return math.max(0, math.min(100, math.floor((tonumber(value) or 100)/25 + 0.5)*25))
        end
        local function refresh()
            refreshing = true
            slider:SetValue(percent()/100)
            Controls.BGD_PingCaption:SetText(BGD_Txt("LOC_BGD_PINGS_VOLUME", percent()))
            refreshing = false
        end
        local sfx = Controls.SFXVolSlider:GetParent()
        local stack = sfx:GetParent()
        row:ChangeParent(stack)
        for i, child in ipairs(stack:GetChildren()) do
            if child == sfx or (sfx.CData ~= nil and child.CData == sfx.CData) then
                stack:AddChildAtIndex(row, i)
                break
            end
        end
        row:SetHide(false)
        stack:CalculateSize()
        stack:ReprocessAnchoring()
        slider:RegisterSliderCallback(function(value)
            if refreshing then return end
            local level = math.max(0, math.min(100, math.floor(value*4 + 0.5)*25))
            local changed = level ~= percent()
            UserConfiguration.SetValue("TeamPings_Volume", level)
            refresh()
            if changed then
                Controls.ConfirmButton:SetDisabled(false)
                LuaEvents.BGD_PingVolumePreview()
            end
        end)
        local baseShow = OnShow
        OnShow = function()
            baseShow()
            refresh()
        end
        LuaEvents.BGD_PingVolumeChanged.Add(refresh)
        ContextPtr:SetShutdown(function() LuaEvents.BGD_PingVolumeChanged.Remove(refresh) end)
        refresh()
    end
end
