if not enabled() or not Controls.BGD_PingMap then return end
include("InputSupport")
include("InstanceManager")
include("SupportFunctions")

local lifetime, failureDuration = 4, 0.45
local markers = InstanceManager:new("BGD_PingMarker", "Anchor", Controls.Markers)
local active, limits = {}, {}
local wheel, failureStart, mouseX, mouseY
local held = {}
local center

if pingSkin then
    Controls.NativeWheel:SetHide(true)
    Controls.SkinWheel:SetTexture(pingSkin.wheel)
    Controls.SkinWheel:SetSizeVal(216, 216)
    Controls.SkinWheel:SetHide(false)
    Controls.Focus:SetTexture(pingSkin.focus)
    Controls.Focus:SetSizeVal(68, 68)
    Controls.FailedCross:SetHide(true)
    Controls.FailedSlash:SetTexture(pingSkin.denied)
    Controls.FailedSlash:SetSizeVal(64, 64)
    Controls.FailedSlash:SetHide(false)
    Controls.BlockedCross:SetHide(true)
    Controls.BlockedSlash:SetTexture(pingSkin.denied)
    Controls.BlockedSlash:SetSizeVal(44, 44)
    Controls.BlockedSlash:SetHide(false)
end

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
    local size = call.angle and 54 or 31
    choice.Icon:SetSizeVal(size, size)
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
    Controls.Focus:SetHide(call == center)
    Controls.Focus:SetOffsetVal(call.dx, call.dy)
    Controls.Focus:SetColor(call.tint)
    Controls.BlockedChoice:SetOffsetVal(call.dx, call.dy)
    Controls.BlockedChoice:SetHide(not blocked)
    Controls.Call:SetText(BGD_Txt(call.label))
end

-- Hauteur des cercles au sol par rapport à leur largeur (0.50 suit l'inclinaison de la caméra,
-- mais les écrase trop), taille du caractère d'appel, taille de l'icône et hauteur du badge au repos.
local RING_RATIO = 0.62
local GLYPH_SIZE = 50
local ICON_SIZE = 90
local BADGE_Y = -34

local function ripple(image, elapsed)
    if elapsed < 0 then image:SetAlpha(0); return end
    local phase = (elapsed % 1.24) / 1.24
    local width = 30 + 97 * (1 - (1-phase)^2)
    image:SetSizeVal(width, width * RING_RATIO)
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
            local size = ICON_SIZE + 27*drop + 7*spring
            art.Icon:SetSizeVal(size, size)
            art.Badge:SetOffsetY(BADGE_Y - 31*drop*drop + 4*spring)
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
    Controls.FailedIcon:SetSizeVal(60, 60)
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
    if pingSkin then
        art.Wave:SetTexture(pingSkin.ripple)
        art.Echo:SetTexture(pingSkin.ripple)
        art.Foot:SetTexture(pingSkin.ripple)
        art.Foot:SetSizeVal(35, math.floor(35 * RING_RATIO + 0.5))
    end
    art.Wave:SetColor(call.tint)
    art.Echo:SetColor(call.tint)
    art.Foot:SetColor(call.tint)
    setIcon(art.Glyph, art.Icon, call)
    art.Glyph:SetFontSize(GLYPH_SIZE)
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

-- For keyboard bindings use the ordered raw mouse/key stream, including after a
-- rebind. v9 actions can also have mouse/controller gestures, dispatched by Civ.
local raw, mapped = {}, {}
local keyNames
local function normalized(s)
    return (s:gsub("%s+", ""):lower())
end
local function fromDisplay(display)
    if not keyNames then
        keyNames = {}
        for key=1,255 do keyNames[normalized(chordText(key))] = key end
    end
    local name = normalized(display)
    local mask = 0
    while true do
        local first, rest = name:match("^([^+]+)%+(.+)$")
        if not first then break end
        if first==normalized(Locale.Lookup("LOC_OPTIONS_KEY_CONTROL")) then mask=mask+512
        elseif first==normalized(Locale.Lookup("LOC_OPTIONS_KEY_ALT")) then mask=mask+256
        elseif first==normalized(Locale.Lookup("LOC_OPTIONS_KEY_SHIFT")) then mask=mask+1024
        else return nil end
        name=rest
    end
    return keyNames[name] and keyNames[name]+mask
end
local function refreshBindings()
    raw,mapped = {},{}
    local native = nativeActions()
    for _, binding in ipairs(pingBindings) do
        local id = native[binding]
        local chords, useMapped = {},false
        for slot=0,1 do
            if id then
                local display=Input.GetGestureDisplayString(id,slot)
                if display and display~="" then
                    local chord=fromDisplay(display)
                    if chord then chords[#chords+1]=chord else useMapped=true end
                end
            else
                local chord=localChord(binding,slot)
                if chord~=0 then chords[#chords+1]=chord end
            end
        end
        if useMapped then mapped[id]=binding
        else for _, chord in ipairs(chords) do raw[chord]=binding end end
    end
end

local function startBinding(binding, owner, x, y)
    if held[owner] then return true end
    if Input.GetActiveContext() ~= InputContext.World or not UIManager:GetMouseOverWorld() then return false end
    local seat = PlayerConfigurations[Network.GetLocalPlayerID()]
    if not seat or observer(seat) then return false end
    local plot = plotAt(x,y)
    if not plot then return false end
    held[owner] = true
    if binding.call then
        sendPing(binding.call, plot)
    elseif not wheel then
        local w,h = UIManager:GetScreenSizeVal()
        local left,top = math.max(0,math.min(x-108,w-216)), math.max(0,math.min(y-108,h-240))
        wheel = {owner=owner,x=x,y=y,cursorX=x,cursorY=y,centerX=left+108,centerY=top+108,plot=plot,time=UI.GetElapsedTime()}
        Controls.Wheel:SetOffsetVal(left,top)
        Controls.Wheel:SetHide(false)
        highlight(center)
        wake()
    end
    return true
end

local function finishBinding(owner)
    if not held[owner] then return false end
    held[owner] = nil
    if wheel and wheel.owner==owner then
        local gesture = wheel
        closeWheel()
        if Input.GetActiveContext()==InputContext.World then sendPing(selection(gesture),gesture.plot) end
    end
    return true
end

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
    if message == KeyEvents.KeyUp then return finishBinding(input:GetKey()) end
    if message ~= KeyEvents.KeyDown then return false end
    if held[input:GetKey()] then return true end
    local binding = raw[keyboardChord(input)]
    if not binding then return false end
    local x,y = mouseX,mouseY
    if x == nil then x,y = UIManager:GetMousePos() end
    return startBinding(binding,input:GetKey(),x,y)
end

local function onActionStarted(id)
    local binding = mapped[id]
    if binding then startBinding(binding,binding,UIManager:GetMousePos()) end
end
local function onActionFinished(id)
    local binding = mapped[id]
    if binding then finishBinding(binding) end
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
refreshBindings()
LuaEvents.BGD_PingBindingsChanged.Add(refreshBindings)
Events.InputActionStarted.Add(onActionStarted)
Events.InputActionFinished.Add(onActionFinished)
Events.LoadGameViewStateDone.Add(onReady)
Events.LoadScreenClose.Add(onReady)
Events.MultiplayerChat.Add(onChat)
LuaEvents.BGD_PingMuted.Add(hideSender)
LuaEvents.BGD_PingVolumeChanged.Add(refreshVolume)
ContextPtr:SetInputHandler(onInput,true)
ContextPtr:SetAppLostFocusHandler(resetInput)
ContextPtr:SetShutdown(function()
    LuaEvents.BGD_PingBindingsChanged.Remove(refreshBindings)
    Events.InputActionStarted.Remove(onActionStarted)
    Events.InputActionFinished.Remove(onActionFinished)
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
