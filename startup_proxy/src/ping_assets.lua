-- Images et sons des signaux d'équipe : mod local « BGE - Ping assets » (DLC\BGEPings), installé
-- avec BGE, sans identifiant Workshop. Un mod coché entre dans la configuration de chaque partie ;
-- en multijoueur, un joueur qui ne l'a pas serait expulsé en rejoignant le salon (KICK_MOD_MISSING).
-- Il reste donc décoché : l'hôte ne l'ajoute à la configuration de la partie qu'au lancement, quand
-- tous les joueurs ont BGE (fin de startup_proxy/src/salon.lua). Les versions de test de la PR #9
-- le cochaient : le menu principal le décoche.
do
    local function BGD_DecocherMediaPings()
        local handle = Modding.GetModHandle("863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd")
        if not handle then return end
        local info = Modding.GetModInfo(handle)
        if info and info.Enabled then
            Modding.DisableMod(handle)
            GameConfiguration.UpdateEnabledMods()
        end
    end
    local ok, err = pcall(BGD_DecocherMediaPings)
    if not ok then print("BGE - images et sons des signaux : " .. tostring(err)) end
end
