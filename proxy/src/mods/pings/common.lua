-- This block and the catalog are scoped to the host UI chunk. The proxy supplies
-- them only under the existing POL verdict. A missing setting (e.g. an old save) is off.
local function enabled()
    if not GameConfiguration.IsNetworkMultiplayer() or GameConfiguration.IsHotseat() or GameConfiguration.IsPlayByCloud() then return false end
    local value = GameConfiguration.GetValue("BGD_TEAM_PINGS")
    return value == true or value == 1
end

local byID = {}
for _, call in ipairs(calls) do byID[call.id] = call end
local function parse(text)
    if type(text) ~= "string" then return end
    local id, x, y = text:match("^%.bge_ping ([%w_]+) (%d%d?%d?%d?) (%d%d?%d?%d?)$")
    if byID[id] then return byID[id], tonumber(x), tonumber(y) end
end

local function observer(config)
    return config:GetSlotStatus() == SlotStatus.SS_OBSERVER or config:GetLeaderTypeName() == "LEADER_SPECTATOR"
end

local function audience(sender, listener)
    local from, to = PlayerConfigurations[sender], PlayerConfigurations[listener]
    if not from or not to or sender == listener or observer(from) or not from:IsHuman() then return false end
    local team = from:GetTeam()
    return observer(to) or (to:IsHuman() and type(team) == "number" and team >= 0 and team ~= TeamTypes.NO_TEAM and team == to:GetTeam())
end

-- POL stays latched during play. A reconnect gets a fresh personal token. Only
-- whisper after the existing host relay (partie.lua) has admitted that exact token.
local function announced(seat)
    local hostID = Network.GetGameHostPlayerID()
    local host,peer = PlayerConfigurations[hostID],PlayerConfigurations[seat]
    if not host or not peer then return false end
    local session,tokens = tostring((host:GetValue("BGD_VERDICT"))):match("^([^@#:]+@[^@#:]+):1:([^:]*)$")
    local peerSession,token = tostring((peer:GetValue("BGD_VERSION"))):match("^([^@#:]+@[^@#:]+)#([^@#:,]+)$")
    if not session or peerSession ~= session then return false end
    if seat == hostID then return true end
    return ("," .. tokens .. ","):find("," .. token .. ",", 1, true) ~= nil
end

local function playerName(id)
    local cfg = PlayerConfigurations[id]
    local name = cfg and cfg:GetPlayerName()
    return (Locale.Lookup(name or tostring(id)):gsub("%[", "("):gsub("%]", ")"):gsub("%c", " "))
end

local function muteKey(id)
    local config = PlayerConfigurations[id]
    if not config then return end
    local key = config:GetNetworkIdentifer()
    if type(key) ~= "number" and type(key) ~= "string" then key = config:GetPlayerName() end
    return tostring(id) .. "/" .. tostring(key)
end
local function muted(id)
    local list = ExposedMembers.BGD_PingMutes
    return list ~= nil and list[muteKey(id)] == true
end

local function volume()
    local value = UserConfiguration.GetValue("TeamPings_Volume")
    return math.max(0, math.min(100, math.floor((tonumber(value) or 100)/25 + 0.5)*25))
end
local function play(call)
    local percent = volume()
    if percent == 0 then return end
    if call.event then
        -- Optional audio-bank events can supply pre-mixed 25/50/75 percent variants.
        UI.PlaySound(call.eventsByVolume and call.eventsByVolume[percent] or call.event)
    elseif Civ6Proxy_PingSound then
        local master = Options.GetAudioOption("Sound", "Master Volume")
        local effects = Options.GetAudioOption("Sound", "SFX Volume")
        Civ6Proxy_PingSound(call.sound, math.floor(percent * master * effects / 10000 + 0.5),
            Options.GetAudioOption("Sound", "Mute Focus"))
    end
end
