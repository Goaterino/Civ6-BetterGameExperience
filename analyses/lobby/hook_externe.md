# Hook externe : créer et régler un salon depuis un site ou un bot Discord

Suite de [l'analyse du salon](README.md#pistes-pour-automatiser-la-création), qui établit que la
création et le réglage d'un salon se font entièrement par l'API Lua du front-end, côté hôte. Reste à
choisir **par où arrive l'ordre** venu de l'extérieur du jeu (fin d'un civdraft sur un site, commande
d'un bot Discord, feuille de match d'un tournoi), et **comment le jeu le reçoit sans devenir une porte
d'entrée**.

Conception du 2026-11-04. Rien n'est implémenté : ce document tranche le transport et pose le format,
les gardes et le plan.

## Ce qui existe déjà et qu'on réutilise

| brique | où | ce qu'elle apporte |
|---|---|---|
| injection de Lua dans le front-end | [startup_proxy/src/options.rs](../../startup_proxy/src/options.rs) (`hook_loadbuffer`, dispatch par nom de fichier) | un bloc ajouté à la fin de `hostgame.lua` / `lobby.lua`, même remplacés par MPH |
| fonctions natives dans le contexte | `publish_natives` (`Civ6Proxy_*`) | passage de données DLL → Lua sans fichier lu par le Lua |
| requête HTTPS sortante | [startup_proxy/src/maj.rs](../../startup_proxy/src/maj.rs) (`curl.exe` de Windows, `CREATE_NO_WINDOW`, thread) | aller chercher ou renvoyer du JSON, sans dépendance ajoutée |
| exe installé à un emplacement fixe | `%LOCALAPPDATA%\BetterGameDLLs\BetterGameDLLs.exe` ([updater](../../updater/README.md)) | une cible stable pour un gestionnaire de protocole, et des fenêtres (TaskDialog) déjà bilingues |
| popup du menu principal | [startup_proxy/src/maj.lua](../../startup_proxy/src/maj.lua) | la confirmation à l'écran, y compris avec le `MainMenu` de MPH |
| réglages + journal communs | onglet BetterGameDLLs, `RolloverProxy.log` | un interrupteur et une trace pour la nouvelle fonction |

Contrainte de fond : **le Lua du jeu n'a ni socket ni accès au disque**. Tout ce qui vient de
l'extérieur doit passer par une native du proxy. C'est elle qui porte les gardes.

## Les trois transports possibles

### A. `civ6bgd://` — un schéma d'URI maison

**Comment ça marche.** Windows résout un schéma d'URI par le registre. Un lien
`civ6bgd://salon?...` cliqué dans un navigateur, dans Discord ou dans un `.url` lance la commande
enregistrée sous `HKCU\Software\Classes\civ6bgd`, l'URL entière passée en argument :

```
HKCU\Software\Classes\civ6bgd
    (défaut)        = "URL:BetterGameDLLs"
    URL Protocol    = ""                                  (la clé vide : c'est elle qui fait un schéma)
HKCU\Software\Classes\civ6bgd\shell\open\command
    (défaut)        = "%LOCALAPPDATA%\BetterGameDLLs\BetterGameDLLs.exe" --uri "%1"
```

`HKCU` suffit : pas d'UAC, enregistrement et retrait par l'installeur, qui écrit déjà à cet
emplacement. Le navigateur demande une confirmation la première fois (« Autoriser ce site à ouvrir
BetterGameDLLs ? »). `%1` est fourni **non décodé** et peut être forgé par n'importe quelle page du
web : c'est l'argument d'un inconnu, pas celui du joueur.

- **Pour.** Aucun serveur à tenir, aucun compte, aucune latence, aucun secret en transit, rien
  d'ouvert sur le réseau ; marche depuis une page statique comme depuis un message Discord.
- **Contre.** Windows seulement (ce dépôt l'est déjà) ; un seul sens (le site n'apprend rien) ;
  les URL sont plafonnées en pratique (~2 000 caractères chez certains navigateurs), trop court pour
  une config complète ; une **activité** Discord (iframe sandbox) ne peut pas ouvrir un schéma
  maison de façon fiable — un message ou un DM du bot, oui.

### B. API web — la DLL interroge un serveur

Le proxy demande périodiquement `GET https://…/salon/attente?machine=<jeton d'appairage>` et
applique ce qu'il reçoit.

- **Pour.** Le lien peut être cliqué depuis un téléphone ; le bot Discord déclenche sans que l'hôte
  touche à son PC ; le canal de retour (état du salon) est gratuit ; rien dans le registre ; une
  activité Discord sait parler à un serveur.
- **Contre.** Un service à héberger, payer et maintenir, avec appairage et authentification ; une
  panne de plus ; du trafic et l'IP du joueur qui tape un serveur en continu ; une latence égale au
  pas d'interrogation.

### C. Boucle locale — la DLL écoute sur `127.0.0.1`

Un petit serveur HTTP dans le proxy, pendant que le jeu tourne, appelé par le `fetch` de la page.
C'est ainsi que Discord lui-même expose son RPC (ports 6463-6472).

- **Pour.** Deux sens, état en direct (« salon créé, 4/6 joueurs »), aucun serveur à nous.
- **Contre.** Il faut que le jeu tourne **et** que le navigateur soit sur la même machine ; un port à
  choisir ; pare-feu et antivirus à convaincre ; côté navigateur, le Private Network Access de
  Chrome impose un préflight `Access-Control-Request-Private-Network` en plus du CORS ; et c'est la
  seule option qui ouvre vraiment une écoute dans le processus du jeu.

## Choix retenu : A pour l'ordre, HTTPS pour le contenu et le retour

Un lien `civ6bgd://` ne transporte pas la config : il transporte une **référence**. Le gros du
contenu est allé chercher par le proxy en HTTPS, chez l'émetteur, qui héberge déjà sa page de draft.
On garde ainsi les avantages de A (rien à tenir, immédiat) sans sa limite de taille, et le canal de
retour devient un simple `POST` facultatif.

```
site de draft / bot Discord
   │ (1) lien civ6bgd://salon?v=1&src=https://exemple/r/AB12CD&nom=PPL+match+3
   ▼
Windows → HKCU\…\civ6bgd\shell\open\command → BetterGameDLLs.exe --uri "<url>"
   │ (2) analyse, hôte autorisé ?, curl -fsSL --max-time 20 <src>, taille et forme contrôlées
   │     → %LOCALAPPDATA%\BetterGameDLLs\recettes\<id>.json   (écriture atomique : .part puis rename)
   │     → jeu fermé : fenêtre « Lancer Civilization VI ? » (steam://rungameid/289070)
   ▼
startup_proxy : à l'ouverture du menu, lecture du dossier ; Civ6Proxy_Recette() la rend au Lua
   │ (3) popup « Recette "PPL match 3" reçue de exemple. Créer le salon ? » — résumé des réglages
   ▼  (4) bloc Lua : HostGame, puis SetupParameters, puis une seule diffusion
salon créé
   │ (5) facultatif : Civ6Proxy_Rapport(json) → POST <rapport> (nom du salon, emplacements, verdict POL)
   ▼
le site affiche l'état du salon
```

Ce que ça donne par usage :
- **site de civdraft** : bouton « Ouvrir dans Civ6 » en fin de draft ; bans et chefs déjà posés ;
- **bot Discord** : `/salon preset:PPL-Team` répond avec le lien, que seul l'hôte clique ;
- **activité Discord** : l'activité ne peut pas ouvrir le schéma ; elle publie le lien dans le canal,
  ou passe par B, ajouté plus tard sans rien changer au format ni à l'applicateur.

## Format d'une recette

JSON, versionné, **déclaratif** : des noms de paramètres de configuration, rien d'autre. Ni chemin,
ni commande, ni Lua, ni adresse, ni clé brute de l'arbre de config.

```json
{
  "v": 1,
  "nom": "PPL Arena S4 — match 3",
  "source": "exemple.org/draft/AB12CD",
  "expire": "2026-11-04T21:00:00Z",
  "serveur": "internet",
  "partie": { "MPH_PRESET": 9, "RULESET": "RULESET_EXPANSION_2", "CITY_STATE_COUNT": 6 },
  "carte": { "MAP_SIZE": "MAPSIZE_TINY", "MAP_SCRIPT": "Continents.lua" },
  "emplacements": [
    { "id": 0, "leader": "LEADER_TRAJAN", "equipe": 0, "statut": "TAKEN" },
    { "id": 1, "statut": "OPEN", "attendu": "Zoé" }
  ],
  "bans": ["LEADER_GILGAMESH", "LEADER_GITARJA"],
  "rapport": "https://exemple.org/draft/AB12CD/etat"
}
```

- `expire` : passé ce moment, la recette est jetée sans rien afficher. Un lien cliqué la veille ne
  doit pas surgir au prochain lancement.
- `attendu` : un nom pour l'affichage seulement. **On n'écrit jamais l'emplacement d'un humain
  connecté** (L1 de l'analyse).
- `rapport` : même hôte que `src`, sinon ignoré.
- Clés et valeurs sont filtrées par une **liste blanche** tirée de `SetupParameters.xml` et des
  `Config.xml` des mods installés : une clé inconnue est journalisée et ignorée, elle n'est pas
  écrite. Au pire, un site malveillant obtient un salon aux réglages bizarres, que l'hôte a confirmé.

## Appliquer sans réveiller les défauts connus

L'ordre vient directement des pièges relevés dans l'analyse :

1. `GameConfiguration.SetToDefaults(gameMode)` ;
2. `Network.HostGame(serverType)` d'après `serveur` (`internet` / `lan`) ;
3. attendre `MultiplayerJoinGameComplete` — rien n'est écrit avant ;
4. `MPH_PRESET` **d'abord**, puis les autres réglages : le preset cascade ses
   `ConfigurationUpdates`, et `CheckPreset` écrase les listes de celui qui écrit avant lui (L2) ;
5. écrire par `SetupParameters:SetParameterValue`, pas par `SetValue` brut, pour que les domaines
   soient revalidés tout de suite et qu'un rafraîchissement ne défasse rien ;
6. vérifier après coup les valeurs sensibles à l'intervalle de la taille de carte — `CITY_STATE_COUNT`
   retombe à 0 si elle sort de `[MAP_MIN_MINOR_PLAYERS, MAP_MAX_MINOR_PLAYERS]` (L2b) — et
   journaliser tout écart entre la recette et la config obtenue ;
7. **une seule** diffusion à la fin : `Network.BroadcastGameConfig(true)`, puis un
   `BroadcastPlayerInfo(id)` par emplacement touché, et seulement pour des emplacements non humains ;
8. écrire dans le journal la recette appliquée, clé par clé, et le résultat.

Le bloc Lua va à la fin de `hostgame.lua` (dispatch par nom de fichier : le remplaçant de MPH est
pris aussi), parce que c'est le contexte qui porte `HostGame`, `CheckPreset` et les
`SetupParameters`. Le bouton d'entrée et le popup de confirmation se posent sur `mainmenu.lua`, déjà
accroché, et parlent au premier par `LuaEvents`. Rien n'est touché hors de la création d'un salon.

## Gardes

Un gestionnaire de protocole est une surface d'attaque : **n'importe quelle page peut l'invoquer**.

- **Confirmation dans le jeu, toujours** : popup avec la source, le nom et le résumé des réglages.
  C'est l'invariant ; aucun réglage ne permet de s'en passer autrement qu'explicitement
  (`hook_externe=auto`, non recommandé, journalisé).
- **Une recette ne fait rien d'autre** que régler un salon : pas de chemin de fichier, pas de
  lancement de partie (`Network.LaunchGame`) ni d'expulsion, pas d'écriture de
  `rollover_proxy.cfg`.
- **`src` et `rapport`** : HTTPS obligatoire, hôtes d'une liste blanche réglable dans le fichier de
  réglages (`hook_hotes=exemple.org,civdraft.com`), réponse plafonnée (64 Ko) et délai de 20 s. TLS
  suffit à l'authentification de la source : pas besoin de la clé de signature des releases, qui
  protège autre chose (du code).
- **Jamais de lancement automatique du jeu** sans fenêtre : l'installeur demande.
- **Usage unique** : la recette est supprimée dès appliquée ou refusée ; une seule en attente,
  la plus récente ; dossier purgé des fichiers périmés à chaque passage.
- **Rien dans le registre sans l'installeur** : `--register-uri` à l'installation,
  retrait à la désinstallation.
- **Réglage dans l'onglet BetterGameDLLs** (obligatoire pour toute nouvelle fonction) :
  `hook_externe=off|confirmer|auto`, défaut `confirmer`, `libre = true` comme `verif_maj` puisqu'il
  ne dépend pas du proxy de GameCore.

## Ce qu'il faut vérifier avant d'écrire du code

1. **`serverType`** utilisable hors de l'interface : `ServerTypeForMPLobbyType` dépend du mode de
   lobby posé par `LuaEvents.ChangeMPLobbyMode`. À confirmer qu'on peut héberger en Internet depuis
   le menu sans passer par l'écran `Lobby`.
2. **Les 76 `NetPlayerInfo` de l'hébergement** (essai du 2026-11-04) : voir s'ils viennent de
   `PlayerSetupLogic` ou de MPH, et si une écriture de recette juste après les croise.
3. **`Network.HostGame` deux fois de suite** (recette reçue alors qu'un salon est déjà ouvert) :
   exiger un `Network.LeaveGame()` propre, ou refuser la recette avec un message.
4. **Confirmation du navigateur** : mesurer ce que donne le clic dans Chrome, Firefox et le client
   Discord, et si le lien survit à un copier-coller en message.

## Plan

| étape | contenu | sans dépendance extérieure |
|---|---|---|
| 1 | `--uri` dans l'installeur, enregistrement `HKCU`, écriture de la recette, tests Rust (analyse de l'URL, hôtes, expiration, taille, atomicité) | oui |
| 2 | natives `Civ6Proxy_Recette()` / `Civ6Proxy_RecetteFinie(etat)`, bloc `hostgame.lua`, popup `mainmenu.lua`, tests Lua à la manière de `tests/test_salon.lua` (recette complète, clé inconnue, emplacement humain, preset hors intervalle, refus, expirée) | oui |
| 3 | `src` en HTTPS pour les grosses recettes, `rapport` en `POST` | un site qui l'implémente |
| 4 | si un déclenchement depuis le téléphone devient nécessaire : B, en gardant le même format de recette et le même applicateur | un serveur |
| 5 | si un état en direct dans une page devient nécessaire : C, boucle locale, mêmes gardes | non |

Les étapes 1 et 2 suffisent à l'usage visé (un draft terminé → un salon réglé d'un clic) et ne
demandent **ni serveur ni compte**. 3 est une ligne de `curl` de plus. 4 et 5 ne changent que le
transport.
