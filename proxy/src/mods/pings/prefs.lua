-- Civ's UserConfiguration lives in memory only: a game load (Multiplayer Helper's remap)
-- or a restart of the game empties it. The ping volume and the DLL-only keyboard chords
-- are also kept in BGE's settings file (`ping_*` lines of rollover_proxy.cfg) and put
-- back where UserConfiguration has lost them. Composed after catalog.lua and bindings.lua.
local function pingPrefKeys()
    local keys = {{"TeamPings_Volume", "ping_volume"}}
    for _, binding in ipairs(pingBindings) do
        if tostring(binding.action):match("^[%w_]+$") then
            for slot = 0, 1 do
                keys[#keys+1] = {bindingKey(binding, slot), "ping_key_" .. binding.action .. "_" .. slot}
            end
        end
    end
    return keys
end

-- File values by setting name. GameCore contexts read one key at a time
-- (Civ6Proxy_PingPref); the Options context reads the whole file.
local function readPingPrefs(keys)
    local saved = {}
    if Civ6Proxy_PingPref then
        for _, pair in ipairs(keys) do
            local ok, value = pcall(Civ6Proxy_PingPref, pair[2])
            if ok and type(value) == "number" then saved[pair[2]] = value end
        end
    elseif Civ6Proxy_LireReglages then
        local ok, text = pcall(Civ6Proxy_LireReglages)
        if ok and type(text) == "string" then
            -- As read_config: the last line of a key wins.
            for line in text:gmatch("[^\r\n]+") do
                local key, value = line:match("^%s*(ping_[%w_]+)%s*=%s*(%d+)%s*$")
                if key then saved[key] = tonumber(value) end
            end
        end
    end
    return saved
end

-- Only values UserConfiguration has lost: one set in this session stays.
local function restorePingPrefs()
    local keys = pingPrefKeys()
    local saved = readPingPrefs(keys)
    for _, pair in ipairs(keys) do
        if saved[pair[2]] ~= nil and UserConfiguration.GetValue(pair[1]) == nil then
            UserConfiguration.SetValue(pair[1], saved[pair[2]])
        end
    end
end

local function savePingPrefs()
    local write = Civ6Proxy_PingPrefsSave or Civ6Proxy_EcrireReglages
    if not write then return end
    local lines = {}
    for _, pair in ipairs(pingPrefKeys()) do
        local value = tonumber(UserConfiguration.GetValue(pair[1]))
        if value and value >= 0 then lines[#lines+1] = pair[2] .. "=" .. math.floor(value) end
    end
    if #lines > 0 then pcall(write, table.concat(lines, "\n")) end
end
