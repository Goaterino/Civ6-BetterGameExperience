# Hook externe : brancher le salon sur CivUp (activité Discord)

Suite de [l'analyse du salon](README.md#pistes-pour-automatiser-la-création), qui établit que la
création et le réglage d'un salon se font entièrement par l'API Lua du front-end, côté hôte. Reste à
choisir **par où l'ordre arrive de l'extérieur**, et **comment le jeu le reçoit sans devenir une
porte d'entrée**.

L'outil visé est [CivUp](https://github.com/Rettend/civup) (Rettend, MIT) : un bot de draft Civ VI
et son **activité Discord**, deux Cloudflare Workers. Conception du 2026-11-04, d'après son dépôt au
2026-11-04 ; rien n'est implémenté de part ni d'autre.

## Ce que CivUp apporte déjà

| brique | où | ce qu'on en fait |
|---|---|---|
| serveur hébergé | deux Workers (bot, activité), D1 / KV / R2 / Durable Objects | il n'y a **pas de serveur à monter** : le nôtre n'existera pas |
| état du draft en direct | Durable Object, session WebSocket (`connection-store.ts`) | la source de la recette, et la cible de l'état du salon |
| jeton signé, court, à portée limitée | `download-ticket` : HMAC `CIVUP_SECRET`, 2 min, lié à `userId` + `matchId` ([activity/src/server/index.ts](https://github.com/Rettend/civup/blob/main/apps/activity/src/server/index.ts)) | le modèle exact de l'appairage : rien de nouveau à inventer côté auth |
| ouverture d'un lien externe | `openExternalLink` → `discordSdk.commands.openExternalLink`, **repli sur le presse-papiers** ([platform/external-links.ts](https://github.com/Rettend/civup/blob/main/apps/activity/src/client/platform/external-links.ts)) | la voie d'un `civ6bgd://`, et l'aveu que le repli manuel est nécessaire |
| surface « web » | la même activité en onglet de navigateur, OAuth Discord (`browser-auth.ts`) | là, et là seulement, un `fetch` vers `127.0.0.1` est envisageable |
| lien de salon Steam | `steam://joinlobby/289070/<lobbyId>/<steamId64>`, **collé à la main par l'hôte** ([steam-link.ts](https://github.com/Rettend/civup/blob/main/apps/bot/src/services/steam-link.ts)) | le gain le plus immédiat : c'est au proxy de le produire |
| bans, picks, vote de carte, modes | `packages/game` (`GameMode` `ffa`/`1v1`…`6v6`, `MAP_SCRIPT_IDS`, `permanentAlly`, `redDeath`, `duplicateFactions`) | le contenu d'une recette |

Et deux contraintes de notre côté : **le Lua du jeu n'a ni socket ni accès au disque** (tout passe
par une native du proxy, qui porte les gardes), et les deux crates sont **sans dépendance**, en
`panic = "abort"` (un serveur HTTP serait à écrire sur `std::net`, sans un `unwrap`).

## Les données circulent dans les deux sens

- **D1, draft → jeu** : créer et régler le salon à partir du draft terminé (bans, chefs, équipes,
  carte). C'est l'automatisation visée par l'analyse.
- **D2, jeu → CivUp** : le lien `steam://joinlobby/...` du salon, puis son état (emplacements tenus,
  verdict POL, lancement). Aujourd'hui l'hôte copie ce lien à la main, à chaque match.

**D2 d'abord.** Il n'écrit rien dans la config, ne touche pas à la simulation, ne demande aucune
confirmation délicate, et supprime une corvée à chaque partie. C'est aussi le meilleur banc d'essai
du canal : si le lien remonte, la recette descendra.

## Appairer le jeu et l'activité : les trois voies

Une fois le jeu et le match associés, **les deux sens passent par de simples requêtes HTTPS vers les
Workers de CivUp**, avec un jeton signé de leur fabrication (le proxy sait déjà faire : `curl.exe`
de Windows, sans fenêtre, comme [maj.rs](../../startup_proxy/src/maj.rs)). Tout le choix de
transport se réduit donc à **ce premier saut**, de l'activité vers le jeu, sur la même machine.

### 1. Micro-serveur local dans le jeu — ce que tu avais en tête

`TcpListener` sur `127.0.0.1:34560` dans le proxy de démarrage, que la page appelle en `fetch`.
C'est ainsi que Discord expose son propre RPC (ports 6463-6472).

- **Pour.** Deux sens, état en direct, aucune dépendance au registre, et la page sait si le jeu
  tourne (un `ping`).
- **Contre, et c'est rédhibitoire ici.** Une **activité Discord est une iframe servie à travers le
  proxy de Discord** (`URL Mappings`, cf. l'étape 7 du README de CivUp) : son réseau est cantonné
  aux hôtes déclarés, et `127.0.0.1` n'en est pas un ni ne peut l'être. Sur la surface « web » (un
  onglet normal), ça marcherait, mais au prix de la politique réseau local de Chrome (préflight
  `Access-Control-Allow-Private-Network`, et une permission demandée à l'utilisateur dans les
  versions récentes), d'un port à découvrir et d'une écoute ouverte dans le processus du jeu.
- **Verdict.** Ce n'est pas le transport, pour la cible qui t'intéresse. À garder en option pour un
  tableau de bord local dans un navigateur (palier 3), pas pour l'activité.

### 2. Schéma d'URI `civ6bgd://`

Windows résout un schéma par le registre ; trois valeurs sous `HKCU` suffisent (pas d'UAC,
enregistrement et retrait par l'installeur, qui écrit déjà à cet endroit) :

```
HKCU\Software\Classes\civ6bgd
    (défaut)     = "URL:BetterGameDLLs"
    URL Protocol = ""          ← la clé vide : c'est elle qui fait un schéma
HKCU\Software\Classes\civ6bgd\shell\open\command
    (défaut)     = "%LOCALAPPDATA%\BetterGameDLLs\BetterGameDLLs.exe" --uri "%1"
```

L'URL entière arrive en argument, **non décodée et forgeable par n'importe quelle page du web**.

- **Dans l'activité**, le chemin est exactement celui du bouton Steam de CivUp :
  `discordSdk.commands.openExternalLink({ url })`. Leur code **se replie sur le presse-papiers**
  quand l'ouverture échoue : le schéma maison n'est donc pas garanti, et il faut mesurer
  (`steam://` passe-t-il ? et un schéma inconnu de Discord ?).
- **Hors activité** (message du bot, surface web), un lien cliquable marche sans réserve.
- **Pour.** Un clic, rien à héberger, rien d'ouvert, jeu fermé géré (l'installeur peut proposer de
  lancer Civ par `steam://rungameid/289070`).
- **Contre.** Windows seulement (le dépôt l'est déjà), URL plafonnée (~2 000 caractères chez
  certains navigateurs) — d'où le principe : **le lien transporte un jeton, pas la recette**.

### 3. Code d'appairage saisi dans le jeu

L'activité affiche six caractères ; l'hôte les saisit dans le popup BetterGameDLLs (ou l'inverse :
le jeu affiche, l'hôte collle dans l'activité). Le proxy échange le code contre un jeton auprès du
Worker.

- **Pour.** Aucune dépendance de plateforme : marche dans l'activité, dans un navigateur, et même si
  l'hôte pilote l'activité depuis son téléphone. **Un seul appairage** : le jeton (ou son
  rafraîchissement) est gardé localement, les matchs suivants ne demandent plus rien.
- **Contre.** Une saisie manuelle, une fois.

## Choix retenu

**Appairage par lien quand la plateforme le permet, par code toujours ; puis HTTPS vers CivUp dans
les deux sens.** Le micro-serveur local n'est pas le transport : il ne voit pas l'activité.

```
activité CivUp (iframe Discord, ou onglet)
   │ bouton « Connecter BetterGameDLLs »
   │   a. openExternalLink("civ6bgd://appairer?api=<origine>&t=<jeton 2 min>")   si ça passe
   │   b. sinon : affiche un code de six caractères (repli, déjà le réflexe de CivUp)
   ▼
BetterGameDLLs.exe --uri …   ou   popup in-game « code : ______ »
   │ → %LOCALAPPDATA%\BetterGameDLLs\appairage.json  (origine, jeton, matchId)
   ▼
proxy de démarrage, dans le jeu
   │ D1  GET  <api>/api/bgd/recette/<matchId>      → popup « créer le salon ? » → Lua
   │ D2  POST <api>/api/bgd/salon/<matchId>/lien   ← steam://joinlobby/289070/<lobbyId>/<steamId64>
   │     POST <api>/api/bgd/salon/<matchId>/etat   ← emplacements, verdict POL, lancement
   ▼
l'activité voit le lien et l'état sans que personne n'ait rien collé
```

## Le contrat à demander à CivUp

Point d'architecture : **CivUp produit la recette, le proxy consomme un document versionné**. Le
proxy n'apprend pas leur modèle de draft, et leurs refontes ne nous cassent pas.

1. **Publier les identifiants internes dans le catalogue de chefs.** Aujourd'hui un chef est un
   *slug* dérivé des **noms anglais affichés** (`slugify("America Abraham Lincoln")` →
   `america-abraham-lincoln`, [sync-bbg-leaders.ts](https://github.com/Rettend/civup/blob/main/packages/game/scripts/sync-bbg-leaders.ts)).
   Le jeu, lui, veut `LEADER_ABRAHAM_LINCOLN` / `CIVILIZATION_AMERICA`. Leur script de synchronisation
   **lit déjà** `LeaderType` et `CivilizationType` dans la base du jeu : il suffit de les garder dans
   l'enregistrement généré. Sinon c'est au proxy de refaire la table à l'envers, dépendant de la
   langue et des variantes BBG — fragile et inutile.
2. **Les cartes.** `MAP_SCRIPT_IDS` est une liste de slugs (`pangaea-ultima-no-wrap`,
   `inland-sea`…). Il faut leur correspondance en noms de fichiers de script (`Continents.lua`…) et
   les conventions de taille, côté CivUp ou dans une table du proxy.
3. **Deux routes**, sur le modèle de leur `download-ticket` (HMAC `CIVUP_SECRET`, courte durée,
   portée `matchId`) : une qui échange un code ou un jeton d'appairage, une qui rend la recette ;
   plus deux routes d'écriture pour D2.
4. **Le lien Steam** : accepter qu'il soit posé par le proxy comme il l'est aujourd'hui à la main
   (leur validation `steam://joinlobby/289070/<lobbyId>/<steamId64>` ne change pas).

## Format d'une recette

JSON, versionné, **déclaratif** : des noms de paramètres de configuration, rien d'autre. Ni chemin,
ni commande, ni Lua, ni adresse, ni clé brute de l'arbre de config.

```json
{
  "v": 1,
  "match": "cu_7f3a",
  "nom": "CivUp — 2v2 — match 3",
  "source": "activity.example.com",
  "expire": "2026-11-04T21:00:00Z",
  "serveur": "internet",
  "partie": { "MPH_PRESET": 9, "RULESET": "RULESET_EXPANSION_2", "CITY_STATE_COUNT": 6 },
  "carte": { "MAP_SIZE": "MAPSIZE_TINY", "MAP_SCRIPT": "Continents.lua" },
  "emplacements": [
    { "id": 0, "leader": "LEADER_TRAJAN", "civ": "CIVILIZATION_ROME", "equipe": 0, "statut": "TAKEN" },
    { "id": 1, "statut": "OPEN", "attendu": "Zoé" }
  ],
  "bans": ["LEADER_GILGAMESH", "LEADER_GITARJA"],
  "etat": "https://activity.example.com/api/bgd/salon/cu_7f3a/etat"
}
```

- Les bans tombent sur les clés de draft de MPH, déjà repérées : `BAN_1..n`, `BAN_POOL_n_NAME`,
  `DRAFT_MAP`, `MAP_NAME` ([noms_cles.tsv](noms_cles.tsv)).
- `equipe` sert les modes `2v2`…`6v6` (`PlayerConfigurations[id]:SetTeam`).
- `attendu` est un nom pour l'affichage. **On n'écrit jamais l'emplacement d'un humain connecté**
  (L1 de l'analyse).
- `expire` : passé ce moment, la recette est jetée sans rien afficher.
- Clés et valeurs passent une **liste blanche** tirée de `SetupParameters.xml` et des `Config.xml`
  des mods installés ; une clé inconnue est journalisée, pas écrite. Au pire, un site malveillant
  obtient un salon aux réglages bizarres, que l'hôte a confirmé.

## Appliquer sans réveiller les défauts connus

L'ordre vient des pièges relevés dans l'analyse :

1. `GameConfiguration.SetToDefaults(gameMode)` ;
2. `Network.HostGame(serverType)` d'après `serveur` (`internet` / `lan`) ;
3. attendre `MultiplayerJoinGameComplete` — rien n'est écrit avant ;
4. `MPH_PRESET` **d'abord**, puis le reste : le preset cascade ses `ConfigurationUpdates`, et
   `CheckPreset` écrase les listes de celui qui écrit avant lui (L2) ;
5. écrire par `SetupParameters:SetParameterValue`, pas par `SetValue` brut, pour que les domaines
   soient revalidés tout de suite et qu'un rafraîchissement ne défasse rien ;
6. revérifier les valeurs sensibles à l'intervalle de la taille de carte — `CITY_STATE_COUNT`
   retombe à 0 hors de `[MAP_MIN_MINOR_PLAYERS, MAP_MAX_MINOR_PLAYERS]` (L2b) — et journaliser tout
   écart entre la recette et la config obtenue ;
7. **une seule** diffusion à la fin : `Network.BroadcastGameConfig(true)`, puis un
   `BroadcastPlayerInfo(id)` par emplacement touché, et seulement pour des emplacements non humains ;
8. journaliser la recette appliquée, clé par clé, et le résultat.

Le bloc Lua va à la fin de `hostgame.lua` (dispatch par nom de fichier : le remplaçant de MPH est
pris aussi), parce que c'est le contexte qui porte `HostGame`, `CheckPreset` et les
`SetupParameters`. Le popup de confirmation se pose sur `mainmenu.lua`, déjà accroché, et parle au
premier par `LuaEvents`. L'état du salon (D2) se lit dans le bloc de `stagingroom.lua`, qui suit déjà
les emplacements pour le verdict POL.

## Gardes

Un gestionnaire de protocole est invocable par n'importe quelle page ; un jeton d'appairage ouvre un
canal sortant. D'où :

- **Confirmation dans le jeu, toujours**, pour toute recette : source, nom et résumé des réglages.
  C'est l'invariant ; seul un `hook_externe=auto` explicite et journalisé s'en passe.
- **Une recette ne fait rien d'autre** que régler un salon : pas de lancement de partie
  (`Network.LaunchGame`), pas d'expulsion, pas d'écriture de `rollover_proxy.cfg`, aucun chemin de
  fichier.
- **Appairage** : l'origine de l'API est mémorisée avec le jeton et ne change pas sans nouvelle
  confirmation ; HTTPS obligatoire ; réponses plafonnées (64 Ko) et délai de 20 s ; jeton
  révocable depuis l'onglet.
- **D2 ne divulgue que ce que les joueurs du salon voient déjà** : lien de salon, noms réseau,
  emplacements. Pas de chemin local, pas de liste de mods, pas d'identifiant Steam autre que celui
  que le lien de salon contient par construction.
- **Usage unique** pour une recette, supprimée dès appliquée ou refusée ; une seule en attente.
- **Registre écrit par l'installeur seulement** (`--register-uri`), retiré à la désinstallation.
- **Réglage dans l'onglet BetterGameDLLs** (obligatoire pour toute fonction) :
  `hook_externe=off|confirmer|auto`, défaut `confirmer`, `libre = true` comme `verif_maj`.
- **Si un micro-serveur local arrive un jour** (palier 3) : écoute sur `127.0.0.1` seulement, jamais
  `0.0.0.0` ; liste blanche d'`Origin` ; **vérification de l'en-tête `Host`** (sans quoi un
  rebinding DNS contourne la vérification d'origine) ; et aucun `unwrap` dans le fil, `panic = "abort"`
  tuant le jeu.

## À vérifier avant d'écrire du code

1. **Le lien Steam du salon, depuis le jeu** : `lobbyId` et `steamId64`. L'exe importe
   `steam_api64.dll`, et le proxy sait déjà détourner une entrée de la table d'imports de l'exe
   (c'est la technique de `luaL_loadbuffer`) : regarder `ISteamMatchmaking` (création du salon,
   rappel `LobbyCreated`) et `ISteamUser::GetSteamID`, et vérifier qu'aucune des 85 fonctions de la
   table `Network` ne le donne déjà. **C'est le verrou du palier 0.**
2. **`openExternalLink` avec un schéma maison** : mesurer `steam://` puis `civ6bgd://` dans le client
   Discord, et constater le repli presse-papiers de CivUp. Décide si le palier d'appairage par lien
   existe dans l'activité ou seulement hors d'elle.
3. **`serverType` hors de l'interface** : `ServerTypeForMPLobbyType` dépend du mode posé par
   `LuaEvents.ChangeMPLobbyMode` ; confirmer qu'on peut héberger en Internet depuis le menu sans
   passer par l'écran `Lobby`.
4. **Les 76 `NetPlayerInfo` de l'hébergement** (essai du 2026-11-04) : d'où ils viennent, et si une
   écriture de recette juste après les croise.
5. **`Network.HostGame` deux fois** (recette reçue salon déjà ouvert) : exiger un `LeaveGame()`
   propre, ou refuser avec un message.
6. **CivBlitz hors périmètre au début** : ces matchs demandent le mod ZIP généré par CivUp, **activé
   avant l'hébergement**. Voir si l'API `Modding.*` permet de l'activer depuis le front-end ; sinon
   la recette doit refuser ces matchs en le disant.

## Plan par paliers

| palier | contenu | dépend de |
|---|---|---|
| 0 | **D2, le lien Steam** : trouver `lobbyId`/`steamId64` dans le jeu, appairage par code, `POST` du lien. Rien n'est écrit dans la config, rien ne touche la simulation | une route d'écriture chez CivUp |
| 1 | **D1, la recette** : `--uri`, enregistrement `HKCU`, natives `Civ6Proxy_Recette()` / `Civ6Proxy_RecetteFinie(etat)`, bloc `hostgame.lua`, popup, tests Rust (URL, origine, expiration, taille) et Lua (à la manière de [test_salon.lua](../../startup_proxy/tests/test_salon.lua)) | le contrat (chefs, cartes, route de recette) |
| 2 | **D2 complet** : état du salon en direct dans l'activité (emplacements, verdict POL, lancement) | une route d'écriture |
| 3 | **Micro-serveur local**, pour un tableau de bord dans un navigateur sur la même machine. Inutile à l'activité Discord | rien |

Le palier 0 vaut à lui seul d'être fait : il supprime un copier-coller à chaque match, et il ne
peut rien casser dans une partie.
