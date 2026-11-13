-- lua startup_proxy/tests/test_ping_bindings.lua [path/to/Base/Assets/UI/Options.lua]
local function read(path)
    local f=assert(io.open(path,"rb")); local s=f:read("*a"); f:close(); return s
end
local p="proxy/src/mods/pings/"
local adapter="local Input=(function(engine)\n"..read(p.."catalog.lua")..read(p.."bindings.lua")..read("startup_proxy/src/ping_bindings.lua").."\nend)(Input)\n"
local prefs, labels, entries = {}, {}, {}
local function ctrl()
    return setmetatable({}, {__index=function(_,name)
        if name=="SetText" then return function(c,v) c.text=v end end
        if name=="RegisterCallback" then return function(c,_,f) c.click=f end end
        if name=="SetHide" then return function(c,v) c.hidden=v end end
        return function() end
    end})
end
local function row()
    return setmetatable({}, {__index=function(t,k) t[k]=ctrl(); return t[k] end})
end
local env=setmetatable({
    Controls=row(),Mouse={eLClick=1}, KeyEvents={KeyDown=1,KeyUp=2}, Keys={X=24, K=11, J=10, U=21, R=18, ["1"]=28, ["2"]=29, ["3"]=30, ["4"]=31, VK_ESCAPE=99,VK_SHIFT=80,VK_CONTROL=81,VK_ALT=82},
    InputHandler=function() return false end, ContextPtr={SetInputHandler=function(_,f) labels.input=f end},
    Input={GetActionCount=function() return 1 end,GetActionId=function(i) return i==0 and 777 or nil end,
        ShouldShowActionKeybinding=function() return true end,GetActionName=function() return "Stock" end,
        GetActionCategory=function() return "UI" end,GetActionDescription=function() return "Stock" end,
        GetGestureDisplayString=function() return "A" end,BeginRecordingGestures=function() labels.native=true end,
        StopRecordingGestures=function() end,ClearRecordedGestures=function() end,ClearGesture=function() labels.clear=true end},
    UserConfiguration={GetValue=function(k) return prefs[k] end,SetValue=function(k,v) prefs[k]=v end},
    LuaEvents={BGD_PingBindingsChanged=function() end},
    Events={InputGestureRecorded={Add=function() end}}, Locale={Lookup=function(v) return (v:gsub("LOC_OPTIONS_KEY_","")) end,
        Compare=function(a,b) return a==b and 0 or (a<b and -1 or 1) end}, BGD_Txt=function(key) return key end,
    _KeyBindingCategories={ResetInstances=function() end,GetInstance=row},
    _KeyBindingActions={ResetInstances=function() entries={} end,GetInstance=function() local r=row(); entries[#entries+1]=r; return r end},
},{__index=_G})
local function key(k,alt)
    return {GetMessageType=function() return 1 end,GetKey=function() return k end,
        IsControlDown=function() return false end,IsShiftDown=function() return false end,IsAltDown=function() return alt end}
end
local function checkView()
    local view=assert(load(adapter.."return Input","@ping_options","t",env))()
    assert(view.GetActionCount()==6)
    assert(view.GetActionId(0)==777)
    local wheel=view.GetActionId(1)
    assert(view.GetActionName(wheel)=="LOC_BGD_PINGS_WHEEL" and view.GetGestureDisplayString(wheel,0)=="X")
    env._CurrentAction,env._CurrentActionIndex=wheel,1
    env.StopActiveKeyBinding=function() env._CurrentAction=nil end
    env.RefreshKeyBinding=function() end
    view.BeginRecordingGestures(true)
    assert(not labels.native)
    assert(labels.input(key(11,true)))
    assert(view.GetGestureDisplayString(wheel,1)=="ALT+K")
    view.ClearGesture(wheel,0)
    assert(view.GetGestureDisplayString(wheel,0)=="")
    view.BeginRecordingGestures(true)
    assert(labels.native)
    view.ClearGesture(777,1)
    assert(labels.clear)
end
checkView()
if arg[1] then
    prefs={}
    local src=read(arg[1])
    local start=assert(src:find("function InitializeKeyBinding()",1,true))
    local finish=assert(src:find("function OnShow()",start,true))
    src=src:sub(start,finish-1):gsub("function InitializeKeyBinding%(%)",function() return "function InitializeKeyBinding()\n"..adapter end,1)
    assert(load(src.."\nInitializeKeyBinding()", "@stock-keybindings","t",env))()
    local function wheelRow()
        for _,r in ipairs(entries) do if r.ActionName.text=="LOC_BGD_PINGS_WHEEL" then return r end end
        error("wheel missing")
    end
    assert(#entries==6 and wheelRow().Binding.text=="X")
    wheelRow().Binding.click()
    assert(env.Controls.KeyBindingPopup.hidden==false)
    labels.input(key(11,false))
    assert(wheelRow().Binding.text=="K" and env.Controls.KeyBindingPopup.hidden==true)
    wheelRow().AltBinding.click(); labels.input(key(10,true)); assert(wheelRow().AltBinding.text=="ALT+J")
    wheelRow().Binding.click(); env.Controls.CancelBindingButton.click(); assert(wheelRow().Binding.text=="K")
    wheelRow().Binding.click(); env.Controls.ClearBindingButton.click(); assert(wheelRow().Binding.text=="")
end

-- Civ can retain an action from InputSettings.json after its original mod is disabled.
-- Its binding still works, while its database name/category and visibility are missing.
env.Input.GetActionCount=function() return 2 end
env.Input.GetActionId=function(id)
    if id==0 then return 777 end
    if id==1 or id=="TeamPingWheel" then return 888 end
end
env.Input.ShouldShowActionKeybinding=function(id) return id==777 end
env.Input.GetActionName=function(id) if id==777 then return "Stock" end end
env.Input.GetActionCategory=function(id) if id==777 then return "UI" end end
env.Input.GetActionDescription=env.Input.GetActionName
env.Input.GetGestureDisplayString=function(id,slot)
    if id==888 then return slot==0 and "V" or "" end
    return "A"
end
local retained=assert(load(adapter.."return Input","@retained_binding","t",env))()
assert(retained.GetActionCount()==6)
assert(retained.GetActionId(1)==888 and retained.ShouldShowActionKeybinding(888))
assert(retained.GetActionName(888)=="LOC_BGD_PINGS_WHEEL")
assert(retained.GetActionCategory(888)=="LOC_BGD_PINGS_NAME")
assert(retained.GetActionDescription(888)=="LOC_BGD_PINGS_WHEEL")
assert(retained.GetGestureDisplayString(888,0)=="V")
labels.native=false
env._CurrentAction=888
retained.BeginRecordingGestures(true)
assert(labels.native)
print("Ping primary/alternate recording, clearing and native delegation passed")
