# Sonde Steam : obtenir le lien du salon depuis le proxy

Le flux de [hook_externe.md](hook_externe.md) a besoin, à son étape 3, du lien
`steam://joinlobby/289070/<lobbyId>/<steamId64>` du salon que l'hôte vient de créer, pour l'envoyer
à la `feedback_url` du site. L'API Steamworks n'a aucun appel « dans quel salon suis-je ? » : il
faut trancher, en conditions réelles, laquelle des voies de
[Le lien de salon Steam](hook_externe.md#le-lien-de-salon-steam--lapi-steamworks-sans-désassemblage)
marche avec le `steam_api64.dll` livré par le jeu.

Conception du 2026-11-12. **Rien n'est écrit** : la sonde sera montée et lancée depuis une machine
Windows avec le jeu et Steam (la machine de développement n'a ni l'exe, ni `steam_api64.dll`).

## Ce qu'on veut savoir

1. Quels exports `steam_api64.dll` expose : l'API plate récente, l'ancienne, les deux, aucune.
2. Quelles chaînes de version d'interface répondent (`SteamFriends0xx`, `SteamUser0xx`,
   `SteamMatchmaking0xx`).
3. Si `GetFriendGamePlayed` sur **son propre** Steam ID rend le salon (`m_steamIDLobby`), et à
   partir de quand après l'hébergement.
4. Si ce salon est bien le nôtre : `GetLobbyOwner` rend notre Steam ID, et un ami rejoint avec le
   lien reconstruit.
5. Si la table Lua `Network` a un accesseur au salon qui nous aurait échappé.

Une seule séance de test doit répondre aux cinq questions.

## Où elle vit

- **Un module `startup_proxy/src/steam.rs`.** La même logique servira ensuite à la vraie fonction
  (le lien envoyé à `feedback_url`) : seule la partie journalisation sera retirée.
- **Une native `Civ6Proxy_SondeSteam()`**, enregistrée avec les autres natives du salon
  ([salon.rs](../../startup_proxy/src/salon.rs)). Elle fait tous les appels, journalise, et rend le
  lien reconstruit, ou nil.
- **Le déclencheur, dans le bloc ajouté à `stagingroom.lua`**
  ([salon.lua](../../startup_proxy/src/salon.lua)) :
  - sur `LuaEvents.HostGame_ShowStagingRoom` (l'hôte), déjà écouté par `BGD_Ouverture` ;
  - sur `LuaEvents.JoiningRoom_ShowStagingRoom` (un client) : la même lecture côté client dira si un
    joueur qui rejoint voit le même salon, utile plus tard à `rejoindre_salon` ;
  - puis **de nouveau à 2, 5, 10 et 30 secondes**. La présence de jeu que lit
    `GetFriendGamePlayed` peut être mise à jour par Steam avec retard : la première lecture
    seule ne suffit pas à conclure. Le minuteur passe par le `SetUpdate` du contexte, enchaîné avec
    celui qui existe éventuellement déjà.
- **Activée par une clé de `rollover_proxy.cfg`** : `sonde_steam = true`, absente par défaut. C'est un
  outil de diagnostic temporaire, pas une fonction pour les joueurs : il n'a pas sa place dans
  l'onglet, et disparaît une fois la question tranchée.
- **Livrée en pré-release** (canal pré-release de l'installeur), sans changement de version du POL.

## Les appels, dans l'ordre

Tous sur le fil du Lua (celui qui appelle la native), celui du jeu : l'API Steam n'est pas sûre
d'un autre fil.

### 1. Le module

`GetModuleHandleW("steam_api64.dll")`, **jamais** `LoadLibrary` : on ne charge pas une DLL que le jeu
n'aurait pas chargée. Nul : journaliser « steam_api64 absent », et rendre nil.

### 2. Les exports

Parcourir la table d'exports du module (en-têtes PE, déjà lus ailleurs par le proxy), et journaliser
chaque nom qui contient `SteamFriends`, `SteamUser`, `SteamMatchmaking`, `GetHSteamUser`,
`FindOrCreate` ou `RegisterCallback`. La liste complète tranche la question 1 d'un coup, même si les
essais qui suivent échouent.

### 3. Les interfaces

Trois façons de les obtenir, essayées dans cet ordre, en journalisant celle qui répond :

| voie | exports | SDK |
|---|---|---|
| accesseurs plats versionnés | `SteamAPI_SteamFriends_v017()`, `SteamAPI_SteamUser_v023()`, `SteamAPI_SteamMatchmaking_v009()` | 1.48 et après |
| interface par son nom | `SteamAPI_GetHSteamUser()`, puis `SteamInternal_FindOrCreateUserInterface(h, "SteamFriends017")` ; versions 017 à 015, `SteamUser023` à 019, `SteamMatchmaking009` à 008 | 1.37 et après |
| accesseurs anciens | `SteamFriends()`, `SteamUser()`, `SteamMatchmaking()` | avant 1.37 |

Les numéros de version sont ceux du SDK public. La liste des exports (étape 2) donne les bons si
aucun ne répond.

### 4. Les appels plats

| appel | signature (x64, `uint64` pour un `CSteamID`) | ce qu'on journalise |
|---|---|---|
| `SteamAPI_ISteamUser_GetSteamID` | `(user) -> uint64` | notre Steam ID |
| `SteamAPI_ISteamFriends_GetFriendGamePlayed` | `(friends, uint64 id, FriendGameInfo_t*) -> bool` | la valeur rendue, les 24 octets en hexadécimal, puis chaque champ |
| `SteamAPI_ISteamMatchmaking_GetLobbyOwner` | `(mm, uint64 lobby) -> uint64` | le propriétaire du salon lu |
| `SteamAPI_ISteamMatchmaking_GetNumLobbyMembers` | `(mm, uint64 lobby) -> int` | le nombre de membres |

`FriendGameInfo_t`, 24 octets :

| offset | champ | taille | contrôle |
|---|---|---|---|
| 0 | `m_gameID` | 8 | les 24 bits bas valent **289070** : preuve que la structure est bien lue |
| 8 | `m_unGameIP` | 4 | |
| 12 | `m_usGamePort` | 2 | |
| 14 | `m_usQueryPort` | 2 | |
| 16 | `m_steamIDLobby` | 8 | non nul, et différent de notre Steam ID |

La structure est passée remplie de zéros : un appel qui rend `true` sans rien écrire se voit.

**Le contrôle sans second joueur** : sur l'hôte, `GetLobbyOwner(m_steamIDLobby)` doit rendre notre
Steam ID, et `GetNumLobbyMembers` au moins 1. Les deux ensemble disent que l'identifiant lu est un
salon où l'on est, et qu'on en est l'hôte. C'est presque une preuve à soi seul ; l'ami qui rejoint
(protocole, étape 2) la complète.

### 5. Côté Lua

Dans le même déclencheur :
- `PlayerConfigurations[Network.GetLocalPlayerID()]:GetNetworkIdentifer()`, comparé au Steam ID de
  l'étape 4 : s'ils diffèrent, on ne lit pas la bonne interface ;
- les noms des fonctions de la table `Network` (`for k, v in pairs(Network)`), triés : l'exe en
  enregistre 85, et un accesseur au salon nous a peut-être échappé.

### 6. Le lien

`steam://joinlobby/289070/<m_steamIDLobby>/<notre Steam ID>`, journalisé en entier.

## Sûreté

- `panic = "abort"` : aucun `unwrap`, aucun indexage qui puisse paniquer. Chaque pointeur rendu par
  Steam est testé avant usage.
- On ne passe jamais une interface à une fonction plate tant que son pointeur n'est pas non nul.
- Aucune écriture, aucun rappel enregistré, aucun détournement dans l'exe : la sonde ne fait que lire.
- Le journal ne contient que des identifiants que le lien du salon contient déjà : notre Steam ID et
  le salon. Pas de liste d'amis, pas de nom de compte.

## Journal attendu

Dans `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`, préfixe `sonde steam :` :

```
sonde steam : t=0s hôte, exe CivilizationVI_DX12.exe
sonde steam : exports SteamAPI_GetHSteamUser, SteamInternal_FindOrCreateUserInterface, SteamAPI_ISteamFriends_GetFriendGamePlayed, …
sonde steam : interfaces par FindOrCreate : SteamFriends017, SteamUser021, SteamMatchmaking009
sonde steam : GetSteamID 76561198000000001 ; GetNetworkIdentifer 76561198000000001 (identiques)
sonde steam : GetFriendGamePlayed(soi) true, octets 2e6904…, appid 289070, lobby 109775241000000000
sonde steam : GetLobbyOwner 76561198000000001 (nous), 1 membre
sonde steam : lien steam://joinlobby/289070/109775241000000000/76561198000000001
sonde steam : Network = BroadcastGameConfig, BroadcastPlayerInfo, …
```

## Protocole de test

| essai | qui | ce qu'il valide |
|---|---|---|
| 1 | l'hôte seul : `sonde_steam = true`, héberger un salon Internet depuis le menu, attendre 30 s | questions 1, 2, 3 et la moitié de 4 (`GetLobbyOwner`) ; question 5 |
| 2 | un ami, sans BGE : clic sur le lien journalisé à l'essai 1 | la seconde moitié de 4 : il arrive dans **ce** salon |
| 3 | l'ami, avec BGE et la sonde : rejoindre normalement | le client lit-il le même salon ? |
| 4 | essai 1 avec l'autre exe (DX11 et DX12) | même DLL Steam, mêmes résultats |
| 5 | essai 1 en salon privé, puis visible par les amis | la présence de jeu, donc `GetFriendGamePlayed`, dépend-elle de la visibilité ? |
| 6 | essai 1 puis migration d'hôte (l'hôte quitte) | le nouvel hôte lit-il le même salon ? |

Récupérer le journal par le bouton de rapport de bug du menu Échap, ou directement dans le dossier
ci-dessus.

## Ce qu'on fait des résultats

| résultat | suite |
|---|---|
| lobby non nul, appid 289070, `GetLobbyOwner` = nous, l'ami arrive | **voie 1 validée** : `steam.rs` devient la fonction du lien, sans la journalisation. Noter le délai observé avant la première lecture valable |
| `true` mais lobby nul, ou valable seulement après un délai variable | garder la voie 1 avec relectures, plafonnées (30 s), et préparer la voie 2 en repli |
| `false`, ou un lobby qui n'est pas le nôtre | **voie 2** : s'abonner à `LobbyCreated_t` / `LobbyEnter_t` par `SteamAPI_RegisterCallback`. Une seconde sonde, sur le même modèle |
| aucune interface obtenue | lire la liste des exports, corriger les noms ou versions, recommencer |
| lien valable selon la sonde, mais l'ami n'arrive pas | le format `steam://joinlobby` en cause, pas la lecture : comparer avec le lien que Steam donne lui-même (clic droit sur l'hôte dans la liste d'amis, « Rejoindre la partie ») |

La voie 3 (`lobbysteamid` de la Web API) se teste à part, sans le jeu : un `curl` sur
`GetPlayerSummaries` avec une clé Web API, pendant l'essai 1. Elle ne servirait qu'au site, en repli.
