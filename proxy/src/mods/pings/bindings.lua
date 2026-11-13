-- Shared by Options and the world overlay. Real actions supplied by standalone v9
-- keep their native input profile. DLL-only installs store keyboard chords in Civ's
-- local UserConfiguration, which participates in the Options Save/RestoreCheckpoint.
local wheelBinding = {action="TeamPingWheel", label="LOC_BGD_PINGS_WHEEL", default=Keys.X}
local pingBindings = {wheelBinding}
for _, call in ipairs(calls) do
    if call.shortcut then
        pingBindings[#pingBindings+1] = {
            action=call.action or ("BGD_Ping_" .. call.id), label=call.label,
            default=256+Keys[tostring(call.shortcut)], call=call
        }
    end
end

local function bindingKey(binding, slot)
    return "BGD_Key_" .. binding.action .. "_" .. slot
end

local function localChord(binding, slot)
    local value = UserConfiguration.GetValue(bindingKey(binding,slot))
    if value ~= nil then return tonumber(value) or 0 end
    return slot == 0 and binding.default or 0
end

-- GetKey() returns Firaxis key IDs, including for letters and modifiers. These
-- are also the IDs stored by the recorder. Windows virtual-key numbers differ.
local modifiers = {}
for _,name in ipairs({"SHIFT","CONTROL","ALT","LSHIFT","RSHIFT","LCONTROL","RCONTROL","LALT","RALT","LMENU","RMENU","MENU"}) do
    local id = Keys["VK_"..name] or Keys[name]
    if type(id)=="number" then modifiers[id] = true end
end
local function keyboardChord(input)
    local key = input:GetKey()
    if type(key) ~= "number" or key <= 0 or key > 255 or modifiers[key] then return nil end
    return key + (input:IsAltDown() and 256 or 0) + (input:IsControlDown() and 512 or 0) + (input:IsShiftDown() and 1024 or 0)
end

local function nativeActions()
    local ids, result = {}, {}
    for i=0,Input.GetActionCount()-1 do ids[Input.GetActionId(i)] = true end
    for _, binding in ipairs(pingBindings) do
        local id = Input.GetActionId(binding.action)
        if ids[id] then result[binding] = id end
    end
    return result
end

local keyTokens = {}
local function addKey(name, token)
    local id = Keys[name]
    if type(id)=="number" then keyTokens[id] = token end
end
for n=65,90 do local letter=string.char(n); addKey(letter,letter) end
for n=0,9 do
    local digit=tostring(n)
    addKey(digit,digit)
    addKey("VK_NUMPAD"..digit,"NP_"..digit)
end
for n=1,24 do addKey("F"..n,"F"..n); addKey("VK_F"..n,"F"..n) end
for _,name in ipairs({"SPACE","TAB","RETURN","END","HOME","LEFT","UP","RIGHT","DOWN","INSERT","DELETE",
    "NUMLOCK","SCROLLLOCK","PRINTSCREEN","BACKSPACE","PAGEUP","PAGEDOWN","PAUSE","SEMICOLON","PLUS",
    "COMMA","MINUS","PERIOD","SLASH","TILDE","LBRACKET","BACKSLASH","RBRACKET","QUOTE"}) do
    addKey(name,name)
    addKey("VK_"..name,name)
end
for name,token in pairs({BACK="BACKSPACE",PRIOR="PAGEUP",NEXT="PAGEDOWN",CAPITAL="CAPSLOCK",
    SCROLL="SCROLLLOCK",SNAPSHOT="PRINTSCREEN",MULTIPLY="NP_MULTIPLY",ADD="NP_PLUS",SUBTRACT="NP_MINUS",
    DECIMAL="NP_DECIMAL",DIVIDE="NP_DIVIDE",OEM_1="SEMICOLON",OEM_PLUS="PLUS",OEM_COMMA="COMMA",
    OEM_MINUS="MINUS",OEM_PERIOD="PERIOD",OEM_2="SLASH",OEM_3="TILDE",OEM_4="LBRACKET",OEM_5="BACKSLASH",
    OEM_6="RBRACKET",OEM_7="QUOTE"}) do addKey("VK_"..name,token) end

local function chordText(chord)
    if chord == 0 then return "" end
    local key = chord % 256
    local token = keyTokens[key]
    local name = token and Locale.Lookup("LOC_OPTIONS_KEY_" .. token) or string.format("#%d",key)
    local parts = {}
    if math.floor(chord/512)%2 == 1 then parts[#parts+1]=Locale.Lookup("LOC_OPTIONS_KEY_CONTROL") end
    if math.floor(chord/256)%2 == 1 then parts[#parts+1]=Locale.Lookup("LOC_OPTIONS_KEY_ALT") end
    if math.floor(chord/1024)%2 == 1 then parts[#parts+1]=Locale.Lookup("LOC_OPTIONS_KEY_SHIFT") end
    parts[#parts+1]=name
    return table.concat(parts,"+")
end
