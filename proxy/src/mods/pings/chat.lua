if enabled() and Controls.ChatEntry and SendChat and not BGD_PingCommands then
    BGD_PingCommands = true
    local function submit(text)
        local command, name = text:match("^%s*/(%S+)%s*(.-)%s*$")
        command = command and command:lower()
        if command == "pingvolume" then
            local level = tonumber(name)
            if level and level >= 0 and level <= 100 and level % 25 == 0 then
                UserConfiguration.SetValue("TeamPings_Volume", level)
                LuaEvents.BGD_PingVolumeChanged()
                play(calls[1])
            end
            local self = Network.GetLocalPlayerID()
            OnChat(self, self, BGD_Txt("LOC_BGD_PINGS_VOLUME", volume()), ChatTargetTypes.CHATTARGET_PLAYER, false)
            Controls.ChatEntry:ClearString()
            return
        end
        if command ~= "mute" and command ~= "unmute" then return SendChat(text) end
        local wanted = tonumber(name:match("^#(%d+)$"))
        local found
        local status
        for _, id in ipairs(GameConfiguration.GetMultiplayerPlayerIDs()) do
            local cfg = PlayerConfigurations[id]
            if cfg and (cfg:IsHuman() or observer(cfg)) and (id == wanted or (not wanted and playerName(id):lower() == name:lower())) then
                if found then
                    status = "LOC_BGD_PINGS_AMBIGUOUS"
                    break
                end
                found = id
            end
        end
        local self = Network.GetLocalPlayerID()
        if not status and found and found ~= self then
            local list = ExposedMembers.BGD_PingMutes or {}
            list[muteKey(found)] = command == "mute" or nil
            ExposedMembers.BGD_PingMutes = list
            if command == "mute" then LuaEvents.BGD_PingMuted(found) end
            status = command == "mute" and "LOC_BGD_PINGS_MUTED" or "LOC_BGD_PINGS_UNMUTED"
        end
        OnChat(self, self, BGD_Txt(status or "LOC_BGD_PINGS_PLAYER", found and playerName(found) or name), ChatTargetTypes.CHATTARGET_PLAYER, false)
        Controls.ChatEntry:ClearString()
    end
    Controls.ChatEntry:RegisterCommitCallback(submit)
end
