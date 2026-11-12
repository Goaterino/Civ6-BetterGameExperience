if not enabled() or not Controls.BGD_PingMap then return end
include("InputSupport")
include("InstanceManager")
include("SupportFunctions")

local lifetime, failureDuration = 4, 0.45
local markers = InstanceManager:new("BGD_PingMarker", "Anchor", Controls.Markers)
local active, limits, shortcuts = {}, {}, {}
local wheel, failureStart, mouseX, mouseY
local held = {}
local center

local function setIcon(glyph, image, call)
    glyph:SetHide(call.texture ~= nil)
    image:SetHide(call.texture == nil)
    if call.texture then
        image:SetTexture(call.texture)
    else
        glyph:SetText(call.icon)
        glyph:SetColor(call.tint)
    end
end

for _, call in ipairs(calls) do
    call.tint = UI.GetColorValue(call.color[1]/255, call.color[2]/255, call.color[3]/255, 1)
    if call.angle then
        local angle = math.rad(call.angle)
        call.dx,call.dy = 62*math.sin(angle), -62*math.cos(angle)
    else
        center = call
        call.dx,call.dy = 0,0
    end
    local choice = {}
    ContextPtr:BuildInstanceForControl("BGD_PingChoice", choice, Controls.Choices)
    choice.Root:SetOffsetVal(call.dx, call.dy)
    setIcon(choice.Glyph, choice.Icon, call)
    if call.shortcut then shortcuts[48 + call.shortcut] = call end
end

local function closeWheel()
    wheel = nil
    Controls.Wheel:SetHide(true)
end

local function selection(gesture)
    local x,y = gesture.cursorX,gesture.cursorY
    local radius = UI.GetElapsedTime()-gesture.time < 0.06 and 46 or 34
    local dx,dy = x-gesture.x,y-gesture.y
    if dx*dx + dy*dy < radius*radius then return center end
    dx,dy = x-gesture.centerX, y-gesture.centerY
    if dx*dx + dy*dy < radius*radius then return center end
    local best, score = center,-math.huge
    for _, call in ipairs(calls) do
        if call.angle then
            local projection = dx*call.dx + dy*call.dy
            if projection > score then best,score = call,projection end
        end
    end
    return best
end

local function highlight(call)
    local blocked = coolingDown(limits, Network.GetLocalPlayerID(), call.id, UI.GetElapsedTime())
    if wheel.highlight == call and wheel.blocked == blocked then return end
    wheel.highlight, wheel.blocked = call, blocked
    Controls.Focus:SetOffsetVal(call.dx, call.dy)
    Controls.Focus:SetColor(call.tint)
    Controls.BlockedChoice:SetOffsetVal(call.dx, call.dy)
    Controls.BlockedChoice:SetHide(not blocked)
    Controls.Call:SetText(BGD_Txt(call.label))
end

local function ripple(image, elapsed)
    if elapsed < 0 then image:SetAlpha(0); return end
    local phase = (elapsed % 1.24) / 1.24
    local width = 30 + 97 * (1 - (1-phase)^2)
    image:SetSizeVal(width, width * 0.50)
    image:SetAlpha((1-phase)^0.85)
end

local function animate()
    local now = UI.GetElapsedTime()
    if wheel then
        if Input.GetActiveContext() ~= InputContext.World then
            closeWheel()
        else
            highlight(selection(wheel))
        end
    end
    for i = #active, 1, -1 do
        local marker = active[i]
        local age = now - marker.start
        local art = marker.controls
        if age >= lifetime then
            markers:ReleaseInstance(art)
            table.remove(active, i)
        else
            ripple(art.Wave, age)
            ripple(art.Echo, age-0.24)
            local drop = math.max(0, 1-age/0.18)
            local spring = math.sin(math.max(0,age-0.18)*21) * math.exp(-math.max(0,age-0.18)*8)
            local size = 54 + 20*drop + 5*spring
            art.Badge:SetSizeVal(size, size)
            art.Badge:SetOffsetY(-28 - 31*drop*drop + 4*spring)
            art.Art:SetAlpha(math.min(1, (lifetime-age)/0.62))
        end
    end
    if failureStart then
        local progress = math.min(1, (now - failureStart) / failureDuration)
        if progress >= 1 then
            Controls.FailedAnchor:SetHide(true)
            failureStart = nil
        else
            local shake = 7 * (1 - progress) * math.sin(progress * 5 * math.pi)
            Controls.FailedArt:SetOffsetVal(shake, -28)
            Controls.FailedArt:SetAlpha(math.min(1, (1 - progress) * 2))
        end
    end
    if #active == 0 and not wheel and not failureStart then
        ContextPtr:ClearUpdate()
    end
end

local function wake()
    ContextPtr:SetUpdate(animate)
end

local function showFailure(call, x, y, now)
    -- Holding or tapping repeatedly cannot stack failures or restart a shake mid-flight.
    if failureStart and now - failureStart < failureDuration then return end
    local worldX, worldY, worldZ = UI.GridToWorld(x, y)
    Controls.FailedAnchor:SetWorldPositionVal(worldX, worldY, worldZ)
    setIcon(Controls.FailedGlyph, Controls.FailedIcon, call)
    Controls.FailedArt:SetOffsetVal(0, -28)
    Controls.FailedArt:SetAlpha(1)
    Controls.FailedAnchor:SetHide(false)
    failureStart = now
    wake()
end

local function showPing(sender, call, x, y)
    local now = UI.GetElapsedTime()
    if muted(sender) then return false end
    if not admit(limits, sender, call.id, now) then
        if sender == Network.GetLocalPlayerID() then
            showFailure(call, x, y, now)
        end
        return false
    end
    if sender == Network.GetLocalPlayerID() and failureStart then
        failureStart = nil
        Controls.FailedAnchor:SetHide(true)
    end
    if #active == 12 then markers:ReleaseInstance(table.remove(active, 1).controls) end
    local art = markers:GetInstance()
    local worldX, worldY, worldZ = UI.GridToWorld(x, y)
    art.Anchor:SetWorldPositionVal(worldX, worldY, worldZ)
    art.Wave:SetColor(call.tint)
    art.Echo:SetColor(call.tint)
    art.Foot:SetColor(call.tint)
    setIcon(art.Glyph, art.Icon, call)
    TruncateString(art.Sender, 130, playerName(sender))
    table.insert(active, {controls=art, start=now, sender=sender})
    wake()
    animate()
    play(call)
    return true
end

local function sendPing(call, plot)
    local sender = Network.GetLocalPlayerID()
    if sender < 0 or not PlayerConfigurations[sender] or observer(PlayerConfigurations[sender]) or not call or not plot or not Map.IsPlot(plot) then return end
    local x,y = Map.GetPlotLocation(plot)
    if showPing(sender,call,x,y) then
        local message = string.format(".bge_ping %s %d %d", call.id, x, y)
        for _, recipient in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
            if audience(sender, recipient) and Network.IsPlayerConnected(recipient) and announced(recipient) then
                Network.SendChat(message, ChatTargetTypes.CHATTARGET_PLAYER, recipient)
            end
        end
    end
end

local function plotAt(x, y)
    local w,h = UIManager:GetScreenSizeVal()
    if w <= 0 or h <= 0 or x < 0 or x >= w or y < 0 or y >= h then return nil end
    -- Use the same screen-to-map conversion as Firaxis' PlotToolTip.
    local col,row = UI.GetPlotCoordFromNormalizedScreenPos(2*x/w-1, 1-2*y/h)
    if not col or not row then return nil end
    local plot = Map.GetPlot(col, row)
    if plot then return plot:GetIndex() end
end

-- Read the ordered mouse/key message stream for both ends of this stroke. Poll only
-- for the first position, before the context has seen a mouse-move event.
local function onInput(input)
    local message = input:GetMessageType()
    if message == MouseEvents.MouseMove then
        mouseX,mouseY = input:GetX(),input:GetY()
        if wheel then wheel.cursorX,wheel.cursorY = mouseX,mouseY end
        return false
    end
    if wheel and (message == MouseEvents.RButtonDown or (message == KeyEvents.KeyDown and input:GetKey() == Keys.VK_ESCAPE)) then
        closeWheel()
        return true
    end
    if message ~= KeyEvents.KeyDown and message ~= KeyEvents.KeyUp then return false end
    local key = input:GetKey()
    if message == KeyEvents.KeyUp then
        if not held[key] then return false end
        held[key] = nil
        if key == Keys.X and wheel then
            local gesture = wheel
            closeWheel()
            if Input.GetActiveContext() == InputContext.World then sendPing(selection(gesture), gesture.plot) end
        end
        return true
    end
    if held[key] then return true end
    local direct = input:IsAltDown() and shortcuts[key]
    if not direct and (key ~= Keys.X or input:IsAltDown()) then return false end
    if input:IsControlDown() or input:IsShiftDown() or Input.GetActiveContext() ~= InputContext.World
        or not UIManager:GetMouseOverWorld() then return false end
    local seat = PlayerConfigurations[Network.GetLocalPlayerID()]
    if not seat or observer(seat) then return false end
    local x,y = mouseX,mouseY
    if x == nil then x,y = UIManager:GetMousePos() end
    local plot = plotAt(x,y)
    if not plot then return false end
    held[key] = true
    if direct then
        sendPing(direct, plot)
    else
        local w,h = UIManager:GetScreenSizeVal()
        local left,top = math.max(0,math.min(x-108,w-216)), math.max(0,math.min(y-108,h-240))
        wheel = {x=x,y=y,cursorX=x,cursorY=y,centerX=left+108,centerY=top+108,plot=plot,time=UI.GetElapsedTime()}
        Controls.Wheel:SetOffsetVal(left,top)
        Controls.Wheel:SetHide(false)
        highlight(center)
        wake()
    end
    return true
end

local function onChat(sender, recipient, text, channel)
    if channel ~= ChatTargetTypes.CHATTARGET_PLAYER or recipient ~= Network.GetLocalPlayerID() then return end
    local call,x,y = parse(text)
    if not call or not audience(sender, recipient) or not announced(sender) then return end
    local w,h = Map.GetGridSize()
    if x < w and y < h then showPing(sender,call,x,y) end
end

local function hideSender(sender)
    for i = #active,1,-1 do
        if active[i].sender == sender then
            markers:ReleaseInstance(active[i].controls)
            table.remove(active,i)
        end
    end
end

local function resetInput()
    mouseX,mouseY = nil,nil
    held = {}
    closeWheel()
end

local volumeParent
local refreshing = false
local function refreshVolume()
    refreshing = true
    Controls.BGD_PingVolume:SetValue(volume()/100)
    Controls.BGD_PingCaption:SetText(BGD_Txt("LOC_BGD_PINGS_VOLUME", volume()))
    refreshing = false
end
local function onReady()
    Controls.BGD_PingMap:SetHide(false)
    ContextPtr:SetHide(false)
    if volumeParent then return end
    local slider = ContextPtr:LookUpControl("/InGame/TopOptionsMenu/Options/SFXVolSlider")
    if not slider then return end
    local sfx = slider:GetParent()
    local stack = sfx:GetParent()
    local row = Controls.BGD_PingVolumeRow
    volumeParent = row:GetParent()
    row:ChangeParent(stack)
    for i, child in ipairs(stack:GetChildren()) do
        if child == sfx or (sfx.CData ~= nil and child.CData == sfx.CData) then
            stack:AddChildAtIndex(row, i)
            break
        end
    end
    row:SetHide(false)
    refreshVolume()
    stack:CalculateSize()
    stack:ReprocessAnchoring()
end
Controls.BGD_PingVolume:RegisterSliderCallback(function(value)
    if refreshing then return end
    local percent = math.floor(value*4 + 0.5)*25
    local changed = percent ~= volume()
    UserConfiguration.SetValue("TeamPings_Volume", percent)
    refreshVolume()
    if changed then
        local confirm = ContextPtr:LookUpControl("/InGame/TopOptionsMenu/Options/ConfirmButton")
        if confirm then confirm:SetDisabled(false) end
        play(center)
    end
end)
Events.LoadGameViewStateDone.Add(onReady)
Events.LoadScreenClose.Add(onReady)
Events.MultiplayerChat.Add(onChat)
LuaEvents.BGD_PingMuted.Add(hideSender)
LuaEvents.BGD_PingVolumeChanged.Add(refreshVolume)
ContextPtr:SetInputHandler(onInput,true)
ContextPtr:SetAppLostFocusHandler(resetInput)
ContextPtr:SetShutdown(function()
    Events.LoadGameViewStateDone.Remove(onReady)
    Events.LoadScreenClose.Remove(onReady)
    Events.MultiplayerChat.Remove(onChat)
    LuaEvents.BGD_PingMuted.Remove(hideSender)
    LuaEvents.BGD_PingVolumeChanged.Remove(refreshVolume)
    if volumeParent then Controls.BGD_PingVolumeRow:ChangeParent(volumeParent) end
    ContextPtr:ClearUpdate()
    markers:ResetInstances()
    if Civ6Proxy_PingSound then Civ6Proxy_PingSound(nil) end -- release asynchronous audio buffers
    ExposedMembers.BGD_PingMutes = nil
end)
