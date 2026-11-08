# Double chargement à l'entrée en partie quand l'hôte est lent

Question : quand l'hôte charge une partie beaucoup plus lentement qu'un autre joueur, ce joueur
arrive en partie avant l'hôte, repart dans un écran de chargement, puis le jeu plante. Pourquoi ?

Analyse statique du 2026-11-07 de `CivilizationVI_DX12.exe` (build 1.0.12.54, projet Ghidra
`~/Tools/ghidra-projects/civ6exe`) et des journaux réseau du 2026-10-29
([netcode/runs/20261029-reconnexion](../../netcode/runs/20261029-reconnexion)), où l'hôte chargeait
plus vite que le client. **Aucun journal d'un vrai double chargement n'a encore été lu** : voir
[À vérifier](#à-vérifier).

## Conclusion

Le second chargement est un **second instantané (`NetSnapshot`) que l'hôte envoie au client dès que
l'hôte a fini son propre chargement**. Le jeu envoie ce renvoi à chaque entrée en partie, même quand
il ne sert à rien. Le client n'a qu'une seule protection : il ignore un instantané reçu **pendant**
son chargement.

- Hôte rapide (cas normal) : le second instantané arrive chez un client qui charge encore. Il est
  ignoré (`ProcessSnapshot, ignored snapshot due to load-in-progress`). Rien ne se voit.
- Hôte lent : le client a déjà fini, il est en partie. Rien ne l'arrête : il recharge la partie
  depuis l'écran de jeu. C'est le double chargement. Le plantage vient de ce rechargement en pleine
  partie, un chemin que le jeu n'emprunte normalement jamais à cet instant.

La cause à la racine est donc côté hôte : l'état « ce joueur attend un instantané » n'est pas
effacé par l'envoi groupé du début. Il ne l'est que par l'accusé du client (`NetSnapshotProcessed`).
Un détail reste ouvert : pourquoi cet état est encore levé, chez un hôte lent, à la fin de son
chargement (deux scénarios ci-dessous).

## Ce que montrent les journaux (hôte rapide, 2026-10-29)

Hôte :

```
12:57:57  SEND (0,1) NetSnapshotRequest PlayerID=0, Reason=2
12:57:57  SEND (0,1) NetSnapshotRequest PlayerID=1, Reason=2
12:57:57  SEND (0,1) NetSnapshot 2 723 845 octets          <- envoi groupé, à tous
12:57:57  CanRecieveMessage: Queuing Messages Start (LoadInProgress)
12:58:25  NetSnapshotProcessed 0  /  TurnSynchronizationBarrier   <- fin du chargement de l'hôte
12:58:25  Requesting snapshot data from the app.            <- HandleSnapshots
12:58:25  SEND (1) NetSnapshot 2 723 302 octets              <- second instantané, au client seul
12:58:28  Queuing Messages End / Flushing Incoming Message Queue
12:58:38  RECV (1) NetSnapshotProcessed 1                    <- accusé du client, après le renvoi
```

Client :

```
12:58:01  RECV NetSnapshot 2 723 845                         <- début du chargement
12:58:30  RECV NetSnapshot 2 723 302
12:58:30  ProcessSnapshot, ignored snapshot due to load-in-progress.   <- sauvé par le hasard
12:58:38  NetSnapshotProcessed 1  /  TurnSynchronizationBarrier        <- fin du chargement
```

Le second instantané est une nouvelle sauvegarde prise par l'hôte : 543 octets de moins que la
première. Le client l'a reçue 8 s avant la fin de son chargement. S'il avait fini 9 s plus tôt, il
l'aurait chargée.

## Le mécanisme dans l'exe

Chaque joueur réseau (`FNetPlayer`) a deux drapeaux :

| champ | sens | posé par | effacé par |
|---|---|---|---|
| `+0x1a` | attend un instantané (raison en `+0x28`) | `NetSnapshotRequest::Process` (`140810ed0` → `140817b80(p, 1, raison)`) | `NetSnapshotProcessed::Process` (`140810e50` → `140817b80(p, 0, -1)`) |
| `+0x1b` | instantané déjà envoyé | `NetSnapshot::Process` (`140810dd0`), **pour le joueur local seulement** ; `140845770` après un envoi par `HandleSnapshots` | `NetSnapshotRequest::Process` |

- **`HandleSnapshots`** (`14083fac0`, hôte seulement, appelé à chaque mise à jour de session par
  `140847260`) : pour chaque joueur avec `+0x1a` levé et `+0x1b` baissé, demande une sauvegarde à
  l'application (`140274f90`), puis `140845770` l'envoie à ce joueur et lève `+0x1b`. Conditions,
  callbacks de l'application (vtable `140fe1968`) :
  - `+0xc8` (`140272240`) : état de l'application = `0xCF36F2EE`, l'hôte est en partie ;
  - `+0xd0` (`140271680`) : GameCore prêt ;
  - `+0xd8(joueur)` (`140272260`) : emplacement du joueur valide.

  Pendant le chargement de l'hôte, ces conditions sont fausses. Le renvoi part au premier tour de
  boucle après la fin du chargement.
- **L'envoi groupé du début** ne lève `+0x1b` que pour le joueur local de chaque machine. Chez
  l'hôte, le client reste donc à `+0x1a = 1, +0x1b = 0` jusqu'à son accusé : c'est exactement la
  condition du renvoi.
- **Côté client, `ProcessSnapshot`** (`140274cd0`, callback `+0xe8`) : seule garde,
  `14016a090` (chargement en cours). Sinon :
  - si l'état de l'application est `0xCA4009DE` ou `0x8356B191`, il passe à `0xCF36F2EE` ;
  - dans tous les cas, il lance le chargement de l'instantané (`14016de20`).

  Un client déjà en partie recharge donc sans condition.
- Les trois messages (`NetSnapshotRequest` 0x4e7, `NetSnapshot` 0x4e8, `NetSnapshotProcessed`
  0x4e9) sont des messages **système** (octet `+8 = 1`) : le dispatcher (`140843470`) ne les met pas
  en file pendant un chargement, contrairement aux `NetGameCoreMessage` (`(queued, 0)`).

## Pourquoi le drapeau est encore levé chez un hôte lent

Puisque `NetSnapshotProcessed` est un message système, l'accusé d'un client rapide devrait effacer
`+0x1a` chez l'hôte avant la fin du chargement de l'hôte, et aucun renvoi ne devrait partir. Deux
scénarios restent possibles :

1. **L'hôte ne traite pas l'accusé avant `HandleSnapshots`.** C'est le cas si la boucle de session
   de l'hôte est bloquée pendant la phase synchrone du chargement (désérialisation de GameCore), et
   si, à la reprise, `HandleSnapshots` passe avant le traitement des messages reçus. Dans le journal
   du 2026-10-29, l'hôte n'écrit rien de 12:57:57 à 12:58:25, et le client rien de 12:58:01 à
   12:58:25 : ni l'un ni l'autre n'a reçu de message dans cette phase. Le journal ne permet donc pas
   de trancher.
2. **Le client redemande un instantané lui-même.** Arrivé en partie, le client passe la barrière de
   tour et envoie son contrôle de synchro (`NetGameSyncCheck`) à un hôte qui charge encore. Si
   l'hôte le compare à un état incomplet, le client reçoit une réparation qui échoue. Il demande
   alors l'instantané complet (`1407f8020` : `NetSnapshotRequest(joueur, raison 0)`), ce qui relève
   `+0x1a`. Le renvoi suit dès la fin du chargement de l'hôte.

Écarté : **MPH**. Son contrôle de seed, qui peut faire appeler `Network.RequestSnapshot()` au client
(`.mph_ui_resync_seed`), est commenté dans `MPHOptions.lua` (ligne 391). Le seul autre appel
(`.mph_ui_snap`, `VotePanel.lua`) est manuel.

## À vérifier

Il faut les `net_message_debug.log` **de l'hôte et du client** d'un vrai double chargement (dossier
`%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI\Logs`, à copier avant de relancer le jeu,
qui les écrase). [capture_double_load.ps1](capture_double_load.ps1), lancé sur les deux machines
avant la partie (`-Role hote` / `-Role client`), suit ces journaux à la milliseconde, relève l'état
du processus (dont « ne répond pas », utile pour le scénario 1), copie `Logs` à chaque fermeture ou
plantage et produit une archive sur le Bureau. On y lira :

- chez le client, un `SEND … NetSnapshotRequest : PlayerID=<client>, Reason=0` après sa barrière,
  avec `Requesting full snapshot` : c'est le scénario 2 ;
- sinon, chez l'hôte, l'ordre de `RECV (<client>) :NetSnapshotProcessed` par rapport à
  `Requesting snapshot data from the app.` : si l'accusé arrive après, c'est le scénario 1 ;
- chez le client, l'absence de `ignored snapshot due to load-in-progress` devant le second
  `RECV … NetSnapshot` : c'est la confirmation du mécanisme.

## Pistes de correctif (non commencées)

Le mécanisme est le même dans les deux scénarios, donc une garde suffit :

- **Côté client** : refuser dans `ProcessSnapshot` un instantané d'entrée en partie quand le joueur
  est déjà en partie et que son propre instantané a été traité. Il faut garder les instantanés
  légitimes : reconnexion, et resynchro complète demandée par `FGameSyncManager`.
- **Côté hôte** : ne pas renvoyer l'instantané à un joueur qui a reçu l'envoi groupé du début,
  c'est-à-dire lever `+0x1b` pour tous les destinataires de l'envoi groupé. Cela ne marche que si
  l'hôte a le proxy, et seulement dans le scénario 1.
