-- From the repository root: lua proxy/tests/test_pings.lua
local dir = "proxy/src/mods/pings/"
local function read(file)
    local f = assert(io.open(dir .. file .. ".lua","rb"))
    local src = f:read("*a")
    f:close()
    return src
end
local common = read("catalog") .. read("common")
local function run(env, src)
    return assert(load(src, "@pings", "t", env))()
end
local function event()
    local handlers = {}
    return setmetatable({
        Add=function(f) handlers[f]=true end, Remove=function(f) handlers[f]=nil end
    }, {__call=function(_,...) for f in pairs(handlers) do f(...) end end})
end
local function events()
    return setmetatable({}, {__index=function(t,k) local e=event(); t[k]=e; return e end})
end
local function control()
    local c={}
    for _,name in ipairs({"SetHide","SetText","SetColor","SetTexture","SetSizeVal","SetFontSize","SetOffsetY","SetAlpha","SetWorldPositionVal","SetValue"}) do
        c[name]=function(self,value) self[name.."Value"]=value end
    end
    function c:SetOffsetVal(x,y) self.x,self.y=x,y end
    function c:RegisterSliderCallback(fn) self.slide=fn end
    function c:RegisterCommitCallback(fn) self.submit=fn end
    function c:ClearString() self.cleared=true end
    return c
end
local function controls()
    return setmetatable({}, {__index=function(t,k) local c=control(); t[k]=c; return c end})
end
local configs={}
-- Host, team-mate, opponent, native observer, BSM spectator, old reconnect, offline team-mate, pending reconnect.
for i=0,7 do
    configs[i]={
        GetTeam=function() return i==2 and 1 or 0 end,
        GetSlotStatus=function() return i==3 and 4 or 3 end,
        GetLeaderTypeName=function() return i==4 and "LEADER_SPECTATOR" or "LEADER_TEST" end,
        IsHuman=function() return i~=3 end,
        GetValue=function(_,key)
            if key=="BGD_VERDICT" then return i==0 and "current@match:1:seat1,seat2,seat3,seat4,seat5,seat6" or nil end
            return (i==5 and "old" or "current") .. "@match#seat" .. i end,
        GetPlayerName=function() return "Seat " .. i end, GetNetworkIdentifer=function() return tostring(i) end
    }
end
local seats={0,1,2,3,4,5,6,7}
local peers,packets={},{}
local now=1
local function peer(id,setting,skin)
    local p={cues={}, markers={},choices={},prefs={},events=events(),luaEvents=events(),shared={},mapControls=controls()}
    local env=setmetatable({
        Controls=p.mapControls, Events=p.events,LuaEvents=p.luaEvents,ExposedMembers=p.shared,PlayerConfigurations=configs,
        Modding={IsModActive=function(guid) return skin and guid=="6471536e-c80a-4db4-8ddf-58cceaab9308" end},
        GameConfiguration={IsNetworkMultiplayer=function() return true end, IsPlayByCloud=function() return false end,
            IsHotseat=function() return false end, GetValue=function() return setting end, GetMultiplayerPlayerIDs=function() return seats end},
        Network={GetGameHostPlayerID=function() return 0 end,GetLocalPlayerID=function() return id end,
            IsPlayerConnected=function(i) return i~=6 end,
            SendChat=function(text,channel,to)
                assert(channel==1)
                packets[#packets+1]={from=id,to=to,text=text}
                peers[to].events.MultiplayerChat(id,to,text,channel)
            end},
        TeamTypes={NO_TEAM=-1},SlotStatus={SS_OBSERVER=4},ChatTargetTypes={CHATTARGET_PLAYER=1},
        UserConfiguration={GetValue=function(k) return p.prefs[k] end,SetValue=function(k,v) p.prefs[k]=v end},
        Options={GetAudioOption=function(_,key) return key=="Mute Focus" and 0 or 100 end},
        Civ6Proxy_PingSound=function(name,gain) table.insert(p.cues,{name,gain}) end,
        Input={GetActiveContext=function() return 1 end,GetActionCount=function() return p.native and 1 or 0 end,
            GetActionId=function(k) if p.native and (k==0 or k=="TeamPingWheel") then return 7 end end,
            GetGestureDisplayString=function(_,slot) return slot==0 and p.native or nil end},InputContext={World=1}, Keys={X=24, K=11, J=10, U=21, R=18, ["1"]=28, ["2"]=29, ["3"]=30, ["4"]=31, VK_ESCAPE=99,VK_SHIFT=80,VK_CONTROL=81,VK_ALT=82},
        KeyEvents={KeyDown=1,KeyUp=2},MouseEvents={MouseMove=3,RButtonDown=4},
        UI={PlaySound=function(eventName) p.cues[#p.cues+1]={eventName} end,
            GetElapsedTime=function() return now end,GetColorValue=function() return 0 end,GridToWorld=function(x,y) return x,y,0 end,
            GetPlotCoordFromNormalizedScreenPos=function(x,y) return math.floor((x+1)*5), math.floor((1-y)*5) end},
        UIManager={GetScreenSizeVal=function() return 1000,1000 end,GetMouseOverWorld=function() return true end,
            GetMousePos=function() return 50,50 end},
        Map={GetPlot=function(x,y) return {GetIndex=function() return x+10*y end} end,IsPlot=function(i) return i>=0 and i<100 end,
            GetPlotLocation=function(i) return i%10,math.floor(i/10) end,GetGridSize=function() return 10,10 end},
        Locale={Lookup=function(v) return v end}, BGD_Txt=function(v) return v end,
        include=function() end, TruncateString=function(c,_,v) c:SetText(v) end,
        InstanceManager={new=function() return {
            GetInstance=function() local c=controls(); p.markers[#p.markers+1]=c; return c end,
            ReleaseInstance=function(_,c) c.released=true end, ResetInstances=function() end} end}
    },{__index=_G})
    env.ContextPtr={
        SetHide=function() end, BuildInstanceForControl=function(_,_,t)
            setmetatable(t,getmetatable(controls()))
            p.choices[#p.choices+1]=t
        end,
        SetInputHandler=function(_,f) p.input=f end,SetUpdate=function(_,f) p.update=f end,ClearUpdate=function() p.update=nil end,
        SetAppLostFocusHandler=function(_,f) p.blur=f end,SetShutdown=function(_,f) p.shutdown=f end,LookUpControl=function() end
    }
    run(env, common .. read("bindings") .. read("limits") .. read("map"))
    p.events.LoadGameViewStateDone()
    p.chat=setmetatable({Controls=controls(),Events=p.events,SendChat=function(text) p.sent=text end,
        OnChat=function(_,_,text) p.status=text end}, {__index=env})
    run(p.chat, common .. read("filter"))
    p.chat.Events.MultiplayerChat.Add(function() p.unread=(p.unread or 0)+1 end)
    run(p.chat, common .. read("chat"))
    return p
end
for i=0,7 do peers[i]=peer(i,true) end
local off=peer(0,false)
assert(not off.input and not off.chat.Controls.ChatEntry.submit and off.chat.Events==off.events)
assert(not peer(0,nil).input)
local function input(p,message,key,x,y,alt)
    return p.input({GetMessageType=function() return message end,GetKey=function() return key end,
        GetX=function() return x end, GetY=function() return y end,
        IsAltDown=function() return alt end,IsShiftDown=function() return false end,IsControlDown=function() return false end})
end
local host=peers[0]
local function flick(p,x,y,dx,dy)
    input(p,3,nil,x,y)
    assert(input(p,1,24))
    assert(p.mapControls.Wheel.SetHideValue==false)
    input(p,3,nil,x+dx,y+dy)
    assert(input(p,2,24))
end
flick(host,760,450,0,65)
assert(#packets==3 and packets[1].text==".bge_ping assist 7 4")
for _,p in ipairs({host,peers[1],peers[3],peers[4]}) do
    assert(p.cues[1][1]=="assist" and p.markers[1].Sender.SetTextValue=="Seat 0" and not p.unread)
end
assert(#peers[2].markers==0 and #peers[5].markers==0 and #peers[6].markers==0 and #peers[7].markers==0)
flick(peers[2],400,450,-70,0)
assert(#peers[3].markers==2 and #peers[4].markers==2 and #host.markers==1)
assert(not input(peers[3],1,24))
for _,bad in ipairs({{2,1,".bge_ping danger 1 1",1},{0,2,".bge_ping danger 1 1",1},
    {0,1,".bge_ping danger 1 1",3},{0,1,".bge_ping danger 20 1",1},{5,1,".bge_ping danger 1 1",1},
    {0,1,".bge_ping unknown 1 1",1}}) do peers[1].events.MultiplayerChat(table.unpack(bad)) end
assert(#peers[1].markers==1)
local bob=peers[1]
bob.chat.Controls.ChatEntry.submit("/mute Seat 0")
assert(bob.markers[1].released)
local unread=bob.unread
bob.events.MultiplayerChat(0,1,"ordinary whisper",1)
flick(host,760,450,70,0)
assert(bob.unread==unread and #bob.cues==1)
bob.chat.Controls.ChatEntry.submit("/unmute #0")
bob.events.MultiplayerChat(0,1,"ordinary whisper",1)
assert(bob.unread==unread+1)
bob.chat.Controls.ChatEntry.submit("ordinary outgoing message")
assert(bob.sent=="ordinary outgoing message")
bob.chat.Controls.ChatEntry.submit("/pingvolume 25")
assert(bob.cues[#bob.cues][2]==25)
now=15
for i=1,6 do flick(host,450,450,0,-70) end
local before,heard=#packets,#host.cues
flick(host,450,450,0,-70)
assert(#packets==before and #host.cues==heard and host.mapControls.FailedAnchor.SetHideValue==false)
-- Changing type remains possible, but a rotating burst stops at twelve and expires at ten seconds.
for i=1,6 do flick(host,450,450, i%2==0 and -70 or 70,0) end
before,heard=#packets,#host.cues
flick(host,450,450,0,0)
assert(#packets==before and #host.cues==heard)
now=25
flick(host,450,450,0,0)
assert(#packets==before+3 and host.cues[#host.cues][1]=="look")
input(host,3,nil,750,450); input(host,1,24); host.blur(); input(host,2,24)
assert(#packets==before+3)
input(host,1,29,nil,nil,true); input(host,2,29)
assert(host.cues[#host.cues][1]=="danger")
-- Primary/alternate and cleared bindings update without restarting or reinstalling.
host.prefs.BGD_Key_TeamPingWheel_0=11
host.prefs.BGD_Key_TeamPingWheel_1=256+21
host.luaEvents.BGD_PingBindingsChanged()
assert(not input(host,1,24) and not input(host,2,24))
assert(input(host,1,11) and input(host,2,11))
assert(input(host,1,21,nil,nil,true) and input(host,2,21))
host.prefs.BGD_Key_TeamPingWheel_0=0
host.luaEvents.BGD_PingBindingsChanged()
assert(not input(host,1,11))
-- When v9 owns a native action its saved binding wins. Native non-keyboard
-- gestures still open/finish through Civ; mapped and raw paths never both send.
host.native="LOC_OPTIONS_KEY_J"
host.luaEvents.BGD_PingBindingsChanged()
assert(not input(host,1,24))
assert(input(host,1,10))
host.events.InputActionStarted(7); host.events.InputActionFinished(7)
assert(host.mapControls.Wheel.SetHideValue==false)
assert(input(host,2,10))
host.native="Mouse 4"
host.luaEvents.BGD_PingBindingsChanged()
assert(not input(host,1,10))
host.events.InputActionStarted(7); assert(host.mapControls.Wheel.SetHideValue==false)
host.events.InputActionFinished(7); assert(host.mapControls.Wheel.SetHideValue==true)
now=30
host.update()
assert(not host.update and host.markers[#host.markers].released)
host.shutdown()
print("POL team pings: routing, spectators, input, mute, audio and rolling limits passed")

-- The intended four-seat match; the BSM caster occupies a player slot on No Team.
-- Reuse the shipping input, map, chat-submit and receive handlers in four UI contexts.
seats={0,1,2,4}
now=100
for _,id in ipairs(seats) do
    local team=({[0]=0,[1]=1,[2]=1,[4]=-1})[id]
    configs[id].GetTeam=function() return team end
    peers[id]=peer(id,true)
end
assert(configs[4]:GetLeaderTypeName()=="LEADER_SPECTATOR" and configs[4]:GetTeam()==-1)
local function pingMatrix(sender,visible)
    local begin=#packets
    local markers,cues={},{}
    for _,id in ipairs(seats) do
        markers[id]=#peers[id].markers
        cues[id]=#peers[id].cues
    end
    flick(peers[sender],700,400,0,-65)
    local delivered={}
    for i=begin+1,#packets do
        local packet=packets[i]
        assert(packet.from==sender and not delivered[packet.to])
        delivered[packet.to]=true
    end
    for _,id in ipairs(seats) do
        local shouldReceive=id~=sender and (id==4 or configs[id]:GetTeam()==configs[sender]:GetTeam())
        assert((delivered[id] or false)==shouldReceive, "wrong private recipient "..id)
        local delta=visible[id] and 1 or 0
        assert(#peers[id].markers==markers[id]+delta and #peers[id].cues==cues[id]+delta,
            "wrong marker or sound for sender "..sender.." and viewer "..id)
    end
end
local function chatVisible(sender,viewer,expected,channel)
    local p=peers[viewer]
    local before=p.unread or 0
    p.events.MultiplayerChat(sender,viewer,"ordinary chat from seat "..sender,channel or 1)
    assert((p.unread or 0)==before+(expected and 1 or 0),"wrong chat visibility")
end
pingMatrix(0,{[0]=true,[4]=true})
pingMatrix(1,{[1]=true,[2]=true,[4]=true})
pingMatrix(2,{[1]=true,[2]=true,[4]=true})
peers[2].chat.Controls.ChatEntry.submit("/mute Seat 1")
pingMatrix(1,{[1]=true,[4]=true})
for _,channel in ipairs({1,2,3}) do
    chatVisible(1,2,false,channel); chatVisible(0,2,true,channel); chatVisible(1,4,true,channel)
end
peers[2].chat.Controls.ChatEntry.submit("/unmute Seat 1")
pingMatrix(1,{[1]=true,[2]=true,[4]=true})
for _,channel in ipairs({1,2,3}) do chatVisible(1,2,true,channel) end
peers[4].chat.Controls.ChatEntry.submit("/mute Seat 0")
pingMatrix(0,{[0]=true})
pingMatrix(2,{[1]=true,[2]=true,[4]=true})
chatVisible(0,4,false); chatVisible(2,4,true)
peers[4].chat.Controls.ChatEntry.submit("/unmute Seat 0")
pingMatrix(0,{[0]=true,[4]=true})
chatVisible(0,4,true)
-- Civ can resolve a pregame No Team seat to its own team at game start.
now=111
configs[4].GetTeam=function() return 2 end
pingMatrix(0,{[0]=true,[4]=true})
pingMatrix(1,{[1]=true,[2]=true,[4]=true})
print("Four-seat fixture passed: teammates both ways, enemies excluded, No Team spectator, local mute/unmute for pings, audio and chat")

-- The mounted v9 skin supplies the whole wheel and map art, including its alpha-only ripples.
local skinned=peer(1,true,true)
local ui=skinned.mapControls
assert(ui.NativeWheel.SetHideValue==true and ui.SkinWheel.SetHideValue==false)
assert(ui.SkinWheel.SetTextureValue=="TPWheel.dds" and ui.SkinWheel.SetSizeValValue==216)
assert(ui.Focus.SetTextureValue=="TPFocus.dds" and ui.Focus.SetSizeValValue==68)
assert(ui.BlockedSlash.SetTextureValue=="TPDenied.dds" and ui.FailedSlash.SetTextureValue=="TPDenied.dds")
for _,choice in ipairs(skinned.choices) do
    local size=choice.Icon.SetTextureValue=="TPLook.dds" and 31 or 54
    assert(choice.Icon.SetSizeValValue==size and choice.Glyph.SetHideValue)
end
input(skinned,3,nil,400,450)
assert(input(skinned,1,24) and ui.Focus.SetHideValue)
input(skinned,3,nil,335,450)
skinned.update()
assert(ui.Focus.SetHideValue==false and ui.Focus.x < -61 and math.abs(ui.Focus.y)<0.01)
assert(input(skinned,2,24))
local marker=skinned.markers[1]
assert(marker.Icon.SetTextureValue=="TPMissing.dds" and marker.Glyph.SetHideValue)
assert(marker.Icon.SetSizeValValue==117 and skinned.cues[1][1]=="Play_TeamPing_Missing")
for _,ring in ipairs({marker.Foot,marker.Wave,marker.Echo}) do assert(ring.SetTextureValue=="TPRipple.dds") end
assert(host.mapControls.SkinWheel.SetTextureValue==nil and host.markers[1].Wave.SetTextureValue==nil)
