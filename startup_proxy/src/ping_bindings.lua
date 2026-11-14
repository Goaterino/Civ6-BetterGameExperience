-- Local Input adapter for Firaxis' InitializeKeyBinding only. Its normal sorted
-- rows, primary and alternate buttons, clear/cancel popup and confirm flow are reused.
local extra, nativeBindings, baseCount = {}, {}, 0
local view = setmetatable({}, {__index=engine})

function view.GetActionCount()
    baseCount = engine.GetActionCount()
    local native = nativeActions()
    extra, nativeBindings = {}, {}
    for _, binding in ipairs(pingBindings) do
        local id = native[binding]
        if id then nativeBindings[id] = binding
        else extra[#extra+1] = binding end
    end
    LuaEvents.BGD_PingBindingsChanged()
    return baseCount + #extra
end

function view.GetActionId(i)
    if i >= baseCount then return extra[i-baseCount+1] end
    return engine.GetActionId(i)
end

function view.ShouldShowActionKeybinding(action)
    return type(action)=="table" or nativeBindings[action]~=nil or engine.ShouldShowActionKeybinding(action)
end

local function text(key)
    return BGD_Txt and BGD_Txt(key) or key
end
function view.GetActionName(action)
    local binding = type(action)=="table" and action or nativeBindings[action]
    if binding then return text(binding.label) end
    return engine.GetActionName(action)
end
function view.GetActionCategory(action)
    if type(action)=="table" or nativeBindings[action] then return text("LOC_BGD_PINGS_NAME") end
    return engine.GetActionCategory(action)
end
function view.GetActionDescription(action)
    local binding = type(action)=="table" and action or nativeBindings[action]
    if binding then return text(binding.label) end
    return engine.GetActionDescription(action)
end
function view.GetGestureDisplayString(action, slot)
    if type(action)=="table" then return chordText(localChord(action,slot)) end
    return engine.GetGestureDisplayString(action,slot)
end
function view.ClearGesture(action, slot)
    if type(action)=="table" then
        UserConfiguration.SetValue(bindingKey(action,slot), 0)
        return
    end
    engine.ClearGesture(action,slot)
end
function view.BeginRecordingGestures(...)
    if type(_CurrentAction)~="table" then return engine.BeginRecordingGestures(...) end
end

local baseHandler = InputHandler
ContextPtr:SetInputHandler(function(input)
    if type(_CurrentAction)=="table" and input:GetMessageType()==KeyEvents.KeyDown then
        if input:GetKey()==Keys.VK_ESCAPE then return true end -- stock key-up cancels
        local chord = keyboardChord(input)
        if chord then
            for _, other in ipairs(extra) do
                for slot=0,1 do
                    if localChord(other,slot)==chord then view.ClearGesture(other,slot) end
                end
            end
            UserConfiguration.SetValue(bindingKey(_CurrentAction,_CurrentActionIndex),chord)
            Controls.ConfirmButton:SetDisabled(false)
            StopActiveKeyBinding()
            RefreshKeyBinding()
        end
        return true
    end
    return baseHandler(input)
end, true)
return view
