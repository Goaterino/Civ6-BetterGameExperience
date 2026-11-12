local totalWindowSeconds = 10
local totalLimit = 12
local streakWindowSeconds = 5
local streakLimit = 6

local function coolingDown(limits, sender, kind, now)
    local history = limits[sender]
    if not history then return false end
    while history[1] and now - history[1].time >= totalWindowSeconds do
        table.remove(history, 1)
    end
    if #history >= totalLimit then return true end
    local first = #history - streakLimit + 1
    if first < 1 or now - history[first].time >= streakWindowSeconds then return false end
    for i = first, #history do
        if history[i].kind ~= kind then return false end
    end
    return true
end

-- Only accepted pings consume a slot. Each one ages out independently.
local function admit(limits, sender, kind, now)
    if coolingDown(limits, sender, kind, now) then return false end
    if not limits[sender] then limits[sender] = {} end
    table.insert(limits[sender], {kind=kind, time=now})
    return true
end
