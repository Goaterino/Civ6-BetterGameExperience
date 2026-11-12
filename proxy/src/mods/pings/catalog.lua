-- Stable IDs travel on the wire. Labels, artwork, cues and wheel positions are local.
-- icon accepts a Civ font icon or text. Set texture instead to use a mounted DDS.
-- sound names a WAV in audio/ (mono 16-bit PCM) or use event for a mounted Civ audio event.
-- angle is clockwise from the top. The center call has no angle.
local calls = {
    {id="look",   label="LOC_BGD_PINGS_LOOK",   icon="[ICON_Range]",              color={65,200,255},  sound="look", skin="Look"},
    {id="danger", label="LOC_BGD_PINGS_DANGER", icon="!",                         color={255,78,76},   sound="danger", skin="Danger", angle=0,   shortcut=2},
    {id="coming", label="LOC_BGD_PINGS_COMING", icon="[ICON_Movement]",           color={77,161,255},  sound="coming", skin="Coming", angle=90,  shortcut=3},
    {id="assist", label="LOC_BGD_PINGS_ASSIST", icon="[ICON_REQUEST_ASSISTANCE]", color={64,225,143},  sound="assist", skin="Assist", angle=180, shortcut=4},
    {id="missing",label="LOC_BGD_PINGS_MISSING",icon="?",                         color={255,205,64},  sound="missing", skin="Missing",angle=270, shortcut=1},
}

-- Retain the installed standalone mod's media as an optional skin. BGE owns input and routing.
if Modding and Modding.IsModActive("6471536e-c80a-4db4-8ddf-58cceaab9308") then
    for _, call in ipairs(calls) do
        if call.skin then
            call.texture = "TP" .. call.skin .. ".dds"
            call.event = "Play_TeamPing_" .. call.skin
            call.eventsByVolume = {}
            for _, percent in ipairs({25,50,75}) do call.eventsByVolume[percent] = call.event .. "_" .. percent end
        end
    end
end
