-- ChatPanel, ChatLogic (whisper reply) and WorldTracker each subscribe separately.
-- Scope the wrapper to this UI context, leaving the map receiver's engine event intact.
if enabled() and not BGD_PingChatFilter then
    BGD_PingChatFilter = true
    local original = Events
    local native = original.MultiplayerChat
    local handlers = {}
    local chat = {}
    function chat.Add(handler)
        if handlers[handler] then return end
        local function filtered(sender, recipient, text, channel, ...)
            if muted(sender) or (channel == ChatTargetTypes.CHATTARGET_PLAYER and parse(text)) then return end
            return handler(sender, recipient, text, channel, ...)
        end
        handlers[handler] = filtered
        native.Add(filtered)
    end
    function chat.Remove(handler)
        native.Remove(handlers[handler] or handler)
        handlers[handler] = nil
    end
    Events = setmetatable({MultiplayerChat=chat}, {__index=function(_, key) return original[key] end})
end
