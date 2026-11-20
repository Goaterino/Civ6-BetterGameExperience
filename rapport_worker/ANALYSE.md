# Récupérer et analyser un rapport de bug

Le bouton « Signaler un bug (BGE) » envoie un zip au relais ([README.md](README.md)), qui le range
dans R2 et ouvre un fil Discord. Ce guide part du numéro `BGE-XXXXXX` (ou de rien) et va jusqu'à la
cause.

## Récupérer

[rapports.ps1](rapports.ps1), depuis la racine du dépôt :

```powershell
.\rapport_worker\rapports.ps1
```

liste les 15 derniers rapports (`-Liste 50` pour plus) : numéro, date locale, taille, joueur, tour,
hôte, nombre d'humains, version de BGE, début de la description. Les rapports reçus avant le
2026-11-20 n'ont que le numéro, la date, la taille et la version : leur résumé n'est pas dans R2.

```powershell
.\rapport_worker\rapports.ps1 BGE-331EFB BGE-EFD552
```

```powershell
.\rapport_worker\rapports.ps1 -Derniers 2
```

téléchargent chaque zip dans `rapports\<id>.zip` (racine du dépôt, ignoré par git), l'extraient dans
`rapports\<id>\` et affichent le résumé de `meta.json` et les graines de carte de `Lua.log`. Un
rapport déjà là n'est pas retéléchargé (`-Force` pour le refaire).

**Ce qu'il faut sur la machine :**
- L'adresse du relais : `$env:BGE_RAPPORT_URL`, sinon la variable GitHub `BGE_RAPPORT_URL`, lue avec
  `gh` (connecté au dépôt).
- La clé de l'équipe : `$env:BGE_CLE_ADMIN`, sinon `rapport_worker\.cle_admin` (ignoré par git). Sur
  une nouvelle machine, la copier depuis une autre, ou en créer une nouvelle avec la commande
  ci-dessous. Elle remplace la clé du Worker : les autres machines doivent alors recevoir la nouvelle.

  ```powershell
  .\rapport_worker\rapports.ps1 -NouvelleCle
  ```

  Il faut `npx wrangler login` une fois, avec le compte Cloudflare du Worker.

Sans la clé, le fil Discord donne aussi un lien de téléchargement (`/rapport/<id>?cle=…`).

## Contenu du zip

| Fichier | Ce qu'on y cherche |
|---|---|
| `meta.json` | Joueur, `joueur_id`, Steam ID, civilisation, tour, `hote`, `humains`, `en_pause`, langue, version de BGE, exe, `date_unix` (envoi), description, liste des fichiers. |
| `bge/RolloverProxy.log` | Journal des proxys, horodaté en secondes unix (`[1795163565]`). Plusieurs lancements du jeu à la suite. |
| `bge/rollover_proxy.cfg` | Réglages du joueur, s'il en a changé. |
| `logs/Lua.log` | Erreurs Lua (`Runtime Error`, `stack traceback`), `GenerateRandomMap: Map Seed = …` à chaque génération de carte. |
| `logs/net_message_debug.log` | Messages réseau et chat, **à l'heure locale du joueur**. |
| `logs/net_connection_debug.log` | Connexions, migrations d'hôte, noms Steam des joueurs, à l'heure locale. |
| `logs/GameCore.log`, `Modding.log`, `Database.log` | Chargement des mods et de la base. |
| `saves/BGE_rapport.Civ6Save` | Sauvegarde faite à l'envoi ; `saves/auto/` : les dernières sauvegardes automatiques. |

## Méthode

1. **Lire `meta.json`** et la description. Noter `date_unix`, le tour, l'hôte, le nombre d'humains.
2. **Situer la bonne partie.** Un rapport peut décrire une partie précédente : le joueur a relancé
   et signale plus tard. Les graines de `Lua.log` listent les parties de la session, dans l'ordre.
   Dans `RolloverProxy.log` :
   - un lancement du jeu commence par `démarrage : proxy winmm chargé dans …CivilizationVI…` ;
   - chaque partie chargée commence par `chargeur : … BGD_GameCore_XP2.dll chargé à sa place` ;
   - `salon : lancement : correctifs POL actifs/désactivés` donne l'état des correctifs au lancement.
3. **Vérifier l'équipement BGE.** En tête de lancement : `messages: guard installed (DX12
   1.0.12.74 …)` si les correctifs réseau sont posés, `unknown exe (…)` sinon (exe différent de la
   version Steam du jour : correctifs réseau absents sur ce PC). Dans le salon, les messages `BGE -`
   du chat (`net_message_debug.log`) donnent l'état de chaque joueur et le verdict de l'hôte.
4. **Faire la chronologie.** Convertir les horodatages unix en UTC (`date -u -d @1795163565`) et
   trouver le décalage des journaux réseau (heure locale) en alignant un même événement : un départ
   de joueur (`drop: player N left` dans le proxy, `ConnectionClosed Player(N)` dans le réseau).
5. **Associer joueurs et numéros.** Dans `net_message_debug.log`, `NetIntroductionSteam :
   m_iPlayerID=N, m_steamID=…` (les 32 bits bas du Steam ID) ; dans `net_connection_debug.log`,
   `Steam ConnectionID Created: … (nom [Steam ID 64])` et `(nom [32 bits bas:1:1])` donnent le nom.
6. **Chercher par type de problème** (voir ci-dessous).
7. **Croiser les rapports d'une même partie** : même graine, mêmes départs de joueurs aux mêmes
   secondes. Deux joueurs du même côté d'une désynchro ne montrent pas sa cause : il faut le
   journal d'un joueur de l'autre côté.

### Désynchronisation

- Fin de tour : `StartGameSyncCheck()` puis `NetGameSyncCheck` envoyé à l'hôte.
- Côté hôte et pairs, `Net RECV (N) :NetSnapshotRequest : PlayerID=N, Reason=0` : le joueur N a
  échoué au contrôle et recharge l'état de l'hôte ; `NetSnapshotProcessed` quand c'est fait.
- `.mph_ui_seed_m<graine>` dans le chat : MPH annonce la graine de chaque joueur après chargement.
- Ce que disent les joueurs : `said "…"` dans `net_message_debug.log`.
- Exemple, partie du 2026-11-20 (BGE-331EFB, BGE-EFD552) : après le T1, 6 clients sur 12 ont
  rechargé l'état de l'hôte et ont trouvé un autre point de départ (« new spawn ») ; la partie a
  continué sans souci ensuite.

### Déconnexions, migration d'hôte, rechargements

- `drop: player N left, its slot stays human` (proxy), `ConnectionClosed Player(N)` (réseau).
- `Checking Steam lobby migration, lobby owner is (nom)`, `I am also the host` : migration.
- `partie : verdict taken over as the new host` : ce joueur reprend le verdict de BGE.
- `snapshots:`, `rejoin:`, `reload:` en tête de lancement : correctifs de rechargement posés ou non.

### Interface et Lua

- `Runtime Error` dans `Lua.log`, avec la pile ; un fichier de mod du Workshop
  (`workshop\content\289070\<id>\…`) que BGE modifie apparaît aussi dans `RolloverProxy.log`
  (`… : … ajouté, compilation réussie` ou une erreur de compilation).

### Commandes utiles

```bash
# Dernier lancement du jeu dans le journal des proxys, sans le bruit des fins de tour.
f=rapports/BGE-XXXXXX/bge/RolloverProxy.log
n=$(grep -n "démarrage : proxy winmm chargé dans .*CivilizationVI" "$f" | tail -1 | cut -d: -f1)
sed -n "$n,\$p" "$f" | grep -vE "debordement : joueur|event type 15|passes mode"

# Événements réseau d'une fenêtre de temps (heure locale du joueur).
awk -F'\t' '$1>="[2026-11-20 08:37:40]" && $1<="[2026-11-20 08:40:20]"' rapports/BGE-XXXXXX/logs/net_message_debug.log \
  | grep -vE "mph_ui_modversion|Started receiving|UnitOperation|TurnActive"
```

## Après l'analyse

- Rapporter au joueur ou à l'équipe ce qui est établi, ce qui ne l'est pas, et le rapport qui
  manque (le joueur de l'autre côté d'une désynchro, par exemple).
- Un bug confirmé devient un ticket Linear (équipe BET) ou une issue GitHub, avec le numéro du
  rapport.
- Les zips restent dans `rapports\` (ignoré par git) ; le bucket supprime les plus anciens au-delà
  de 9,5 Go.
