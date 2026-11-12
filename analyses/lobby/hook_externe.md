# Hook externe : créer et rejoindre un salon depuis un site de draft

Suite de [l'analyse du salon](README.md#pistes-pour-automatiser-la-création), qui établit que la
création et le réglage d'un salon se font entièrement par l'API Lua du front-end, côté hôte. Reste à
choisir **par où l'ordre arrive de l'extérieur**, et **comment le jeu le reçoit**.

Conception du 2026-11-04 (première cible : [CivUp](https://github.com/Rettend/civup), bot de draft
et activité Discord), révisée le 2026-11-12 pour n'importe quel site de draft. Rien n'est
implémenté.

## Le flux visé

1. Les joueurs font le draft sur le site.
2. Draft terminé, l'un d'eux clique « Héberger la partie » : le site ouvre
   `civ6bge://creation_salon?data=…`, et son jeu crée le salon réglé.
3. Le proxy de ce joueur renvoie au site le lien `steam://joinlobby/...` du salon (`feedback_url`).
4. Les autres cliquent « Rejoindre » : le site ouvre `civ6bge://rejoindre_salon?steamurl=…`, et leur
   jeu, s'il est ouvert, rejoint le salon.

Le site ne connaît que deux liens et une route HTTPS. Les réglages propres à chaque communauté sont
des **presets entretenus par BGE**, que le site désigne par leur nom.

## Architecture : un relais figé, toute la logique dans le proxy

L'installeur ne se met pas à jour lui-même : le manifeste d'une release ne liste que `winmm.dll` et
`BGD_GameCore_XP2.dll` ([release/manifest.py](../../release/manifest.py)), et une mise à jour lancée
depuis le jeu (`--after-game`) passe par la copie de l'installeur déjà en place. Seul un
`BetterGameExperience_V<x>.exe` téléchargé et ouvert remplace cette copie (`copy_self`). Le proxy de
démarrage, lui, change à chaque release.

D'où le partage :
- **l'installeur est un relais muet, écrit une fois pour toutes** : il transmet l'URI au jeu sans la
  lire ;
- **le proxy de démarrage fait tous les contrôles** : schéma, verbes, versions du format, presets,
  expiration, confirmation. Le schéma peut s'étoffer sans toucher à l'installeur.

### Le registre

Trois valeurs sous `HKCU`, sans droits administrateur :

```
HKCU\Software\Classes\civ6bge
    (défaut)     = "URL:BetterGameExperience"
    URL Protocol = ""          ← la clé vide : c'est elle qui fait un schéma
HKCU\Software\Classes\civ6bge\shell\open\command
    (défaut)     = "%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe" --uri "%1"
```

La commande désigne toujours **la copie à l'emplacement fixe**, jamais le
`BetterGameExperience_V<x>.exe` téléchargé (que `remove_old_installers` supprime de toute façon).
Elle ne change jamais : une nouvelle release n'a pas à réécrire le registre.

L'argument doit être `--uri "%1"`, pas `"%1"` seul : un premier argument qui ne commence pas par
`--` est pris pour le `%command%` de l'option de lancement Steam (`launch(&args)`,
[main.rs](../../updater/src/main.rs)) et lancé comme le jeu.

### Le relais (`--uri`)

Ce qui est figé, et ne doit plus changer une fois publié :
- la branche `--uri` passe **en tout premier** dans `main()`, avant `migrate_data_dir`,
  `lang::init` et `copy_self`, sans fenêtre ni réseau ;
- elle écrit la chaîne reçue, brute, dans `%LOCALAPPDATA%\BetterGameExperience\uri.txt` : d'abord
  dans `uri.txt.tmp`, puis par renommage, pour que le proxy ne lise jamais un fichier à moitié
  écrit. Un fichier déjà présent est remplacé : **le dernier clic gagne** ;
- elle se termine aussitôt.

Aucun contrôle : ni du schéma, ni de la taille (la ligne de commande de Windows plafonne déjà à
32 767 caractères). Le manifeste de l'installeur est en `asInvoker` : un clic ne demande pas
d'élévation.

Jeu fermé, le relais ne le lance pas : le fichier attend le prochain lancement, et le proxy juge
alors s'il est périmé.

### L'amorçage par l'installeur

Une étape `assurer_amorcage()`, indépendante de l'état des DLL, à **chaque** lancement de
l'installeur (fenêtre, `--after-game`, option de lancement Steam ; pas avec `--uri`) :

1. `copy_self()` vers `%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe`, comme
   aujourd'hui ;
2. **vérifier que la copie a les octets de l'exe lancé.** Si l'ancienne copie est verrouillée
   (l'option de lancement Steam la garde ouverte pendant la partie), `std::fs::copy` échoue en
   silence, et la clé désignerait un exe qui ne connaît pas `--uri` : chaque clic ouvrirait la
   fenêtre de l'installeur. Copie différente : pas de clé, une ligne de journal, et nouvel essai au
   lancement suivant ;
3. écrire la clé si elle est absente ou différente de la commande attendue.

**Rien de cela ne bloque** : pas de fenêtre, pas d'élévation, pas de réseau. Une clé absente ne
fait **pas** passer l'installation en « à réparer » (ce qui mènerait à réinstaller les DLL, en
administrateur) : DLL à jour, la fenêtre dit « à jour », et l'amorçage s'est fait sans rien demander.
La désinstallation retire la clé.

Un joueur qui ne met à jour que par la notification du jeu ne reçoit jamais le nouvel installeur,
donc jamais la clé. Le proxy le voit (clé absente) et le menu principal le dit une fois : « ouvrez
une fois le dernier installeur pour activer les liens civ6bge:// ».

### La surveillance de `uri.txt`, côté proxy

Un **fil Rust du proxy de démarrage**, démarré au chargement :
- toutes les secondes, il ne lit que la date et la taille du fichier (`GetFileAttributesExW`).
  `FindFirstChangeNotificationW` sur le dossier, avec un délai d'une seconde, réagit tout de suite
  et garde la seconde comme filet ;
- au changement, il lit le fichier (plafonné à 64 Ko), et le range dans une case partagée, sous
  verrou, avec un compteur de génération ;
- aucun `unwrap` : `panic = "abort"` tuerait le jeu. Toute erreur est journalisée, puis ignorée.

Côté Lua, une native `Civ6Proxy_Uri()` rend la génération et le contenu ; une seconde,
`Civ6Proxy_UriFinie(etat)`, supprime le fichier. Les contextes concernés l'interrogent depuis leur
`SetUpdate`, en cumulant `dt` jusqu'à une seconde environ. Un contexte n'a **qu'un** `SetUpdate`, et
[maj.lua](../../startup_proxy/src/maj.lua) en pose déjà un sur `MainMenu` : il faut les enchaîner,
sinon le dernier posé écrase l'autre.

Le fichier n'est supprimé qu'une fois l'URI appliquée, refusée ou périmée : un plantage entre-temps
ne la perd pas.

## Quand une URI est traitée

| où est le joueur | ce qui se passe |
|---|---|
| menu principal | traitée tout de suite, sans popup |
| dans un salon | **popup de confirmation**, puis traitée (voir ci-dessous) |
| lancement engagé (compte à rebours, `Network.LaunchGame`, chargement) | en attente |
| en partie | en attente, en silence : une ligne de journal, aucun popup |

Par construction : seuls les contextes du menu principal et de la staging room interrogent
`Civ6Proxy_Uri()`. Aucun contexte de partie ne le fait, donc un clic pendant une partie ne peut pas
en faire sortir un joueur.

Une URI en attente est traitée au retour au menu ou dans un salon si elle est encore fraîche :
l'`expire` de la recette s'il y en a un, sinon dix minutes après la date du fichier. Sinon elle est
supprimée et journalisée, sans rien afficher : un clic oublié pendant une partie de trois heures ne
doit pas faire surgir un salon à la fin.

**Le popup ne sert qu'à ce cas** : le joueur est déjà dans un salon, et le lien va l'y changer ou
l'en faire sortir.
- `creation_salon`, **hôte** du salon ouvert : « appliquer le draft à ce salon ? ». Ni `HostGame`, ni
  `LeaveGame` (voir [Appliquer à un salon déjà ouvert](#appliquer-à-un-salon-déjà-ouvert)) ;
- `creation_salon`, **client** dans le salon d'un autre : « quitter ce salon et héberger celui du
  draft ? », puis `Network.LeaveGame()` propre et la suite comme depuis le menu ;
- `rejoindre_salon` alors qu'on est dans un autre salon : « quitter ce salon et rejoindre celui-là ? ».
  Le lien du salon où l'on est déjà est ignoré.

Un refus supprime l'URI.

## Les liens

### `civ6bge://creation_salon?data=<JSON en base64url>`

La recette tient entière dans le lien : aucun serveur à interroger, n'importe quel site ou message
peut la porter. Douze emplacements avec des identifiants en clair tiennent sous 2 000 caractères.

```json
{
  "v": 1,
  "preset": "cpl-ffa",
  "nom": "Draft #4812",
  "feedback_url": "https://civdraft.example/api/draft/4812/lobby",
  "feedback_token": "<jeton opaque fourni par le site>",
  "expire": "2026-11-12T22:00:00Z",
  "overrides": { "MAP_SCRIPT": "Pangaea.lua" },
  "emplacements": [
    { "id": 0, "leader": "LEADER_TRAJAN", "civ": "CIVILIZATION_ROME", "equipe": 0, "steam": "76561198000000001" },
    { "id": 1, "leader": "LEADER_GANDHI", "civ": "CIVILIZATION_INDIA", "equipe": 1, "attendu": "Zoé" }
  ]
}
```

- `v` : version du format. Une version inconnue du proxy affiche « mettez à jour BGE ».
- `preset` : nom d'un preset de BGE (voir [Les presets](#les-presets-par-communauté)). Inconnu : le
  salon n'est pas créé, et un message le dit.
- `overrides` : seulement les clés que le preset déclare modifiables. Une autre clé est journalisée,
  pas écrite.
- `emplacements` : le résultat du draft. `equipe` sert les modes par équipes
  (`PlayerConfigurations[id]:SetTeam`) ; `attendu` est un nom pour l'affichage ; `steam` est
  facultatif (voir [Le Steam ID facultatif](#le-steam-id-facultatif)).
- `feedback_url` / `feedback_token` : où renvoyer le lien du salon, et de quoi le rattacher au draft
  côté site (voir [Le retour du lien](#le-retour-du-lien-feedback_url)). Facultatifs.
- `expire` : passé ce moment, la recette est supprimée sans rien afficher.

**Les bans ne descendent pas dans le jeu.** Ils appartiennent au draft, qui se déroule entièrement
sur le site ; une fois les chefs attribués, il n'y a plus rien à interdire. Les clés `BAN_*` et
`DRAFT_MAP` de MPH servent son draft par le chat : la recette n'y touche pas.

**Ce qu'on attend du site** : les identifiants internes du jeu (`LEADER_ABRAHAM_LINCOLN`,
`CIVILIZATION_AMERICA`), pas des noms affichés ni des *slugs*. CivUp, par exemple, dérive ses slugs
des noms anglais ([sync-bbg-leaders.ts](https://github.com/Rettend/civup/blob/main/packages/game/scripts/sync-bbg-leaders.ts)),
mais son script lit déjà `LeaderType` et `CivilizationType` dans la base du jeu : il suffit de les
garder. Même chose pour les cartes : des noms de fichiers de script (`Continents.lua`), pas des slugs.

### `civ6bge://rejoindre_salon?steamurl=steam://joinlobby/289070/<lobbyId>/<steamId64>`

- **Forme stricte** : appid 289070 et deux identifiants numériques, rien d'autre. Le proxy ne passe
  jamais à Steam qu'un `joinlobby` **reconstruit** à partir de ces trois nombres.
- **Pour rejoindre**, l'URL reconstruite est confiée à Steam (`ShellExecute`) : c'est le chemin d'un
  joueur qui colle le lien aujourd'hui, et Steam transmet la demande au jeu ouvert comme une
  invitation. À vérifier en jeu, depuis le menu principal et depuis un autre salon.
- Le site peut proposer le lien Steam brut en repli, aux joueurs sans BGE. Ce que BGE ajoute : la
  garde contre le clic en partie, la sortie propre d'un autre salon, et plus tard le déplacement
  automatique vers l'emplacement drafté.

## Les presets par communauté

Les réglages varient d'une communauté à l'autre ; ils changent peu. Ils sont donc **embarqués dans
le proxy de démarrage**, et suivent les releases.

Un preset contient :
- les réglages de partie et de carte (dont `MPH_PRESET` si la communauté s'en sert) ;
- les mods requis (version de BBG…) ;
- le type et la visibilité du salon (Internet, privé ou public) ;
- les clés que le site peut modifier (`overrides`) ;
- les **hôtes autorisés pour `feedback_url`**.

Un mod requis absent : le salon n'est pas créé, et un message dit lequel activer. Le point ouvert
n°6 (activer un mod depuis le front-end, par l'API `Modding.*`) dira si l'on peut faire mieux.

## Le retour du lien (`feedback_url`)

Dès que le lien `steam://joinlobby/289070/<lobbyId>/<steamId64>` du salon est connu, le proxy
l'envoie :
- `POST` en JSON : `{ "token": <feedback_token>, "steam_url": <lien>, "bge": <version> }` ;
- HTTPS obligatoire ;
- par `curl.exe`, sans fenêtre, comme [maj.rs](../../startup_proxy/src/maj.rs) ; délai de 20 s,
  deux nouveaux essais, réponse ignorée au-delà du code HTTP ;
- **seulement si l'hôte de `feedback_url` est dans la liste du preset.** Sinon le salon est créé,
  rien n'est envoyé, et un message le dit. Un lien forgé par une page quelconque ne fait donc pas
  partir le lien du salon (Steam ID de l'hôte compris) vers un inconnu. La liste des presets sert
  aussi de registre des sites de confiance : une communauté qui veut le retour automatique demande à
  y être ajoutée.

Le retour n'envoie que ce que les joueurs du salon voient déjà : le lien. Pas de chemin local, pas de
liste de mods.

## Créer le salon depuis le menu

L'ordre vient des pièges relevés dans l'analyse :

1. `GameConfiguration.SetToDefaults(gameMode)` ;
2. `Network.HostGame(serverType)` d'après le preset ;
3. attendre `MultiplayerJoinGameComplete` — rien n'est écrit avant ;
4. `MPH_PRESET` **d'abord**, puis le reste : le preset de MPH cascade ses `ConfigurationUpdates`, et
   `CheckPreset` écrase les listes de celui qui écrit avant lui (L2) ;
5. écrire par `SetupParameters:SetParameterValue`, pas par `SetValue` brut, pour que les domaines
   soient revalidés tout de suite et qu'un rafraîchissement ne défasse rien ;
6. revérifier les valeurs sensibles à l'intervalle de la taille de carte — `CITY_STATE_COUNT`
   retombe à 0 hors de `[MAP_MIN_MINOR_PLAYERS, MAP_MAX_MINOR_PLAYERS]` (L2b) — et journaliser tout
   écart entre la recette et la config obtenue ;
7. **une seule** diffusion à la fin : `Network.BroadcastGameConfig(true)`, puis un
   `BroadcastPlayerInfo(id)` par emplacement touché, et seulement pour des emplacements non humains ;
8. journaliser la recette appliquée, clé par clé, et le résultat ;
9. lire le lien du salon, et l'envoyer à `feedback_url`.

**Qui écrit le chef d'un joueur, et quand.** Le draft étant joué d'avance, l'hôte écrit **tous** les
emplacements juste après `HostGame`, pendant qu'ils sont encore IA ou ouverts : aucun humain n'est là
pour contredire (L1). Un joueur qui rejoint reçoit la config complète de l'hôte, puis rediffuse son
propre emplacement, avec le chef qu'il vient de recevoir : sans dégât. Reste la course de la
conclusion 4 (une rediffusion partie d'un état antérieur). D'où une **vérification après chaque
arrivée** : comparer le chef et l'équipe de l'emplacement à la recette, et ne rediffuser que s'ils
ont dérivé, **au plus deux fois par emplacement**. Le ping-pong du mode anonyme (L5) montre ce que
coûte une correction sans plafond.

Le bloc Lua va à la fin de `hostgame.lua` (dispatch par nom de fichier : le remplaçant de MPH est
pris aussi), le contexte qui porte `HostGame`, `CheckPreset` et les `SetupParameters`. La
surveillance de `uri.txt` vit dans les blocs de `mainmenu.lua` et `stagingroom.lua`, qui lui parlent
par `LuaEvents`. La staging room suit déjà les emplacements pour le verdict POL.

## Appliquer à un salon déjà ouvert

Cas de l'hôte qui confirme le popup. Des humains sont déjà connectés :
- leurs emplacements ne sont **jamais** écrits par l'hôte (L1) ;
- chefs et équipes sont écrits sur les emplacements IA ou ouverts seulement ;
- les Steam ID de la recette déclenchent les déplacements (section suivante) ;
- `MPH_PRESET` d'abord, puis le reste, et une seule diffusion à la fin, comme depuis le menu ;
- le lien du salon est envoyé à `feedback_url`.

## Le Steam ID facultatif

L'hôte connaît le Steam ID de chaque joueur en Lua : `PlayerConfigurations[id]:GetNetworkIdentifer()`,
déjà utilisé par [rapport.lua](../../startup_proxy/src/rapport.lua).

**Sans `steam` dans la recette**, les chefs et les équipes sont écrits sur les emplacements, et
chacun prend le sien à la main, comme aujourd'hui.

**Avec `steam`**, sur tous les emplacements ou seulement certains :
- **l'hôte** : si son Steam ID est dans la recette, son entrée est **renumérotée en emplacement 0**
  avant l'écriture. Aucun swap : l'ordre des emplacements ne porte rien que la recette ne réécrive ;
- **un joueur qui rejoint** atterrit sur le premier emplacement ouvert, que l'hôte ne choisit pas.
  S'il est attendu ailleurs :
  - **avec BGE**, l'hôte lui envoie « emplacement N » par le canal d'annonce BGE du salon, et c'est
    sa propre machine qui fait `Network.RequestPlayerIDChange(N)`. L'hôte l'accepte : l'emplacement
    visé est IA ou ouvert. L'hôte n'écrit jamais l'emplacement d'un humain connecté (L1) ;
  - **sans BGE**, la staging room affiche « Zoé est attendue en 3 », et le joueur se déplace à la
    main ;
- les emplacements sans `steam` restent libres : premier arrivé, premier servi ;
- un Steam ID absent de la recette est **signalé** à l'hôte. Une recette n'expulse personne et ne
  bloque pas le lancement ;
- validation : 17 chiffres, commençant par `7656119`.

## Le lien de salon Steam : l'API Steamworks, sans désassemblage

C'est l'étape 3 du flux, et **le premier point à trancher** : sans lien, pas de retour au site.

Notre DLL tourne **dans le processus du jeu**, et `steam_api64.dll`, que l'exe importe, expose l'API
plate (*flat API*) de Steamworks en C. Le jeu a déjà fait son `SteamAPI_Init` ; il suffit d'un
`GetProcAddress`. Les fonctions plates prennent le `CSteamID` en `uint64` : aucun ABI C++ à imiter,
aucun en-tête du SDK à embarquer.

**Obtenir les interfaces**, sans dépendre du numéro de version du SDK :

```
SteamAPI_GetHSteamUser()                                        → HSteamUser
SteamInternal_FindOrCreateUserInterface(h, "SteamUser023")      → ISteamUser*
SteamInternal_FindOrCreateUserInterface(h, "SteamFriends017")   → ISteamFriends*
```

On essaie les quelques versions voisines, et on garde la première qui rend un pointeur non nul. Un
`steam_api64.dll` d'un ancien SDK n'a peut-être pas ces deux exports : les accesseurs exportés
directement (`SteamFriends`, `SteamUser`) servent de repli.

**`steamId64`** : `SteamAPI_ISteamUser_GetSteamID(user)`.

**`lobbyId` : la moitié difficile.** L'API Steamworks n'a **aucun** appel « dans quel salon
suis-je ? » : c'est au jeu de le retenir, depuis `LobbyCreated_t` / `LobbyEnter_t`. Trois voies :

1. `SteamAPI_ISteamFriends_GetFriendGamePlayed(friends, <notre propre Steam ID>, &info)` →
   `info.m_steamIDLobby`. `FriendGameInfo_t` fait 24 octets : `m_gameID` (8), `m_unGameIP` (4),
   `m_usGamePort` (2), `m_usQueryPort` (2), `m_steamIDLobby` (8, offset 16). La
   [doc d'ISteamFriends](https://partner.steamgames.com/doc/api/ISteamFriends) ne décrit l'appel que
   pour un ami : sur soi-même, il n'est pas documenté. **À tester d'abord.**
2. S'abonner à `LobbyCreated_t` / `LobbyEnter_t` par `SteamAPI_RegisterCallback`, export de la même
   DLL, comme le fait le modèle `CCallback` du SDK. On ne détourne rien dans l'exe, au prix d'une
   dépendance à la table virtuelle de `CCallbackBase`, publique dans le SDK mais interne.
3. Côté site, sans code dans le jeu : `GetPlayerSummaries` de la Web API rendrait un champ
   `lobbysteamid`. Valve ne le documente pas, et il dépend de la visibilité du profil. Se teste avec
   un `curl` et une clé Web API, pendant qu'un joueur au profil public héberge un salon.
   `ILobbyMatchmakingService.GetLobbyData`, l'interface officielle, réclame une clé d'éditeur : hors
   de portée.

Le format `steam://joinlobby/<appid>/<lobbyId>/<steamId64>` n'est pas documenté par Valve non plus,
mais c'est ce que les joueurs collent aujourd'hui, et ce que CivUp valide : il marche.

### La sonde

Ni l'exe ni `steam_api64.dll` ne sont sur la machine de développement : on tranche en conditions
réelles, par une sonde du proxy de démarrage livrée en pré-release. Elle ne fait que lire et
journaliser, quand on héberge un salon (`MultiplayerJoinGameComplete`) :
1. les exports présents dans `steam_api64.dll` (récents et anciens) ;
2. les chaînes de version d'interface qui répondent (`SteamFriends017` et précédentes, idem
   `SteamUser0xx`) ;
3. le Steam ID par `GetSteamID`, comparé à `GetNetworkIdentifer()` côté Lua : la preuve qu'on lit
   la bonne interface ;
4. `GetFriendGamePlayed(soi-même)` : valeur rendue et `m_steamIDLobby` ;
5. le lien reconstruit ;
6. les noms des fonctions de la table Lua `Network` : l'exe en enregistre 85, et un accesseur au
   salon nous a peut-être échappé.

Le testeur héberge, un ami rejoint **avec le lien journalisé**. S'il arrive dans le salon, la
voie 1 est validée ; sinon le journal dit où ça casse, et on passe à la voie 2.

Détail des appels, du journal attendu, du protocole et des suites selon le résultat :
[sonde_steam.md](sonde_steam.md).

## Gardes

- **Jamais en partie** : seuls les contextes du menu et de la staging room traitent une URI, et
  jamais après l'engagement du lancement.
- **Une recette ne fait rien d'autre** que régler un salon : pas de `Network.LaunchGame`, pas
  d'expulsion, pas d'écriture de `rollover_proxy.cfg`, aucun chemin de fichier. Seul un
  `LeaveGame()` confirmé dans le popup fait quitter un salon.
- **Liste blanche** : clés et valeurs tirées de `SetupParameters.xml` et des `Config.xml` des mods
  installés, plus les `overrides` déclarés par le preset. Une clé inconnue est journalisée, pas
  écrite.
- **Retour** limité aux hôtes du preset, en HTTPS ; réponses plafonnées (64 Ko), délai de 20 s.
- **Une seule URI en attente**, supprimée dès appliquée, refusée ou périmée.
- **Registre** écrit par l'installeur seulement, retiré à la désinstallation.
- **Réglage dans l'onglet BetterGameExperience** (obligatoire pour toute fonction) :
  `liens_civ6bge = true|false`, défaut `true`, `libre = true` comme `verif_maj`.

## À vérifier avant d'écrire du code

1. **Le lien du salon** : la sonde, voie 1 puis 2. C'est le verrou de l'étape 3.
2. **`rejoindre_salon` par `ShellExecute`** : depuis le menu principal, et depuis un autre salon.
3. **`serverType` hors de l'interface** : `ServerTypeForMPLobbyType` dépend du mode posé par
   `LuaEvents.ChangeMPLobbyMode` ; confirmer qu'on peut héberger en Internet depuis le menu sans
   passer par l'écran `Lobby`.
4. **Les 76 `NetPlayerInfo` de l'hébergement** (essai du 2026-11-04) : d'où ils viennent, et si une
   écriture de recette juste après les croise.
5. **L'engagement du lancement** : quel événement de la staging room marque le début du compte à
   rebours, pour arrêter d'y traiter les URI.
6. **Les mods requis** : l'API `Modding.*` permet-elle d'activer un mod depuis le front-end ? Sinon,
   message et refus.

## Plan par paliers

| palier | contenu | dépend de |
|---|---|---|
| 0 | **La sonde Steam**, en pré-release ([sonde_steam.md](sonde_steam.md)) | une machine Windows avec le jeu et Steam |
| 1 | **Le relais et l'amorçage** : `--uri`, `uri.txt`, `assurer_amorcage()`, retrait à la désinstallation ; surveillance de `uri.txt` par le proxy (fil, natives) | rien |
| 2 | **`creation_salon`** : format v1, presets embarqués, application depuis le menu et dans un salon, popup ; tests Rust (décodage, versions, expiration, taille) et Lua (à la manière de [test_salon.lua](../../startup_proxy/tests/test_salon.lua)) | palier 1 |
| 3 | **`feedback_url`** et **`rejoindre_salon`** | paliers 0 et 2 |
| 4 | **Steam ID** : renumérotation de l'hôte, déplacement des joueurs avec BGE, affichage pour les autres | palier 2 |

Le relais (palier 1) est le seul morceau de l'installeur : une fois publié, il ne bouge plus. Tout
le reste suit les releases du proxy.

## Pistes écartées

- **Micro-serveur local dans le jeu** (`127.0.0.1`, appelé en `fetch` par la page). Une activité
  Discord est une iframe servie par le proxy de Discord, qui ne peut pas joindre `127.0.0.1`. Dans
  un onglet normal, il faudrait passer la politique réseau local de Chrome (préflight
  `Access-Control-Allow-Private-Network`, permission demandée à l'utilisateur), découvrir un port,
  et garder une écoute ouverte dans le processus du jeu. Le relais par fichier fait la même chose
  sans rien ouvrir. Si un tableau de bord local arrive un jour : `127.0.0.1` seulement, liste
  blanche d'`Origin`, vérification de l'en-tête `Host` (contre le rebinding DNS), aucun `unwrap`.
- **Lien qui ne transporte qu'un jeton, et appairage avec le site** (conception du 2026-11-04). Il
  supposait un contrat serveur propre à chaque site. La recette dans le lien et la `feedback_url`
  donnent le même flux sans appairage.
- **Recette téléchargée depuis une URL du lien.** Toute page pourrait faire appeler un hôte
  arbitraire par le jeu.
- **Un installeur qui se met à jour lui-même.** Inutile ici : le relais ne change plus, et la logique
  vit dans le proxy.
- **Placer les joueurs sans BGE par l'hôte**, qui réécrirait leur emplacement après leur arrivée :
  c'est la course de L1.

## L'activité Discord de CivUp

CivUp ouvre ses liens externes par `discordSdk.commands.openExternalLink`, et **se replie sur le
presse-papiers** quand l'ouverture échoue
([platform/external-links.ts](https://github.com/Rettend/civup/blob/main/apps/activity/src/client/platform/external-links.ts)).
Reste à mesurer si un schéma maison passe dans le client Discord (`steam://` d'abord, puis
`civ6bge://`). Hors activité (message du bot, onglet de navigateur), un lien cliquable marche sans
réserve. Le navigateur demande au premier clic « Ouvrir BetterGameExperience ? », avec une case
« toujours autoriser ».
