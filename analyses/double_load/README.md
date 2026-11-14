# Double chargement à l'entrée en partie quand l'hôte est lent

Question : quand l'hôte charge une partie beaucoup plus lentement qu'un autre joueur, ce joueur
arrive en partie avant l'hôte, repart dans un écran de chargement, puis le jeu plante. Pourquoi ?

Analyse statique du 2026-11-07 de `CivilizationVI_DX12.exe` (build 1.0.12.54, projet Ghidra
`~/Tools/ghidra-projects/civ6exe`) et des journaux réseau du 2026-10-29
([netcode/runs/20261029-reconnexion](../../netcode/runs/20261029-reconnexion)), où l'hôte chargeait
plus vite que le client. **Confirmée le 2026-11-13** par la capture d'un vrai double chargement,
des deux côtés : voir [Capture du 2026-11-13](#capture-du-2026-11-13).

## Conclusion

Le second chargement est un **second instantané (`NetSnapshot`) que l'hôte envoie au client dès que
l'hôte a fini son propre chargement**, à tout joueur dont il n'a pas encore l'accusé. Le jeu envoie ce
renvoi à chaque entrée en partie où l'hôte finit avant le client, même quand il ne sert à rien. Le
client n'a qu'une seule protection : il ignore un instantané reçu **pendant** son chargement.

- Hôte nettement plus rapide (cas courant) : le renvoi arrive chez un client qui charge encore. Il
  est ignoré (`ProcessSnapshot, ignored snapshot due to load-in-progress`). Rien ne se voit.
- Hôte nettement plus lent : l'accusé du client arrive pendant le chargement de l'hôte. Aucun renvoi
  ne part.
- **Entre les deux** : l'hôte finit avant le client, donc il renvoie, mais le renvoi met plus de
  temps à arriver que ce qu'il reste au client à charger. Le client est déjà en partie quand il le
  reçoit, et rien ne l'arrête : il recharge la partie depuis l'écran de jeu. C'est le double
  chargement. Le « plantage » vient de ce rechargement en pleine partie : le 2026-11-13, il reste
  bloqué dans la désérialisation de GameCore.

Le client commence à charger en retard sur l'hôte, du temps de transfert de l'envoi groupé. Il peut
donc charger plus vite que l'hôte et finir après lui : c'est le « l'hôte est plus lent » des
joueurs. Le double chargement se produit quand la fin du chargement de l'hôte tombe dans la fenêtre

```
fin client − (sauvegarde + transfert du renvoi)  <  fin hôte  <  fin client + latence de l'accusé
```

Elle est à peu près aussi large que le temps de transfert de l'instantané : 11 s le 2026-11-13
(2,9 Mo à environ 265 Ko/s, partie au tour 123), une ou deux secondes en début de partie. Le risque
grandit donc avec la taille de la sauvegarde et la lenteur du lien.

La cause à la racine est côté hôte : l'état « ce joueur attend un instantané » n'est pas effacé par
l'envoi groupé du début. Il ne l'est que par l'accusé du client (`NetSnapshotProcessed`). Côté client,
la garde ne couvre que le chargement lui-même, pas le transfert du renvoi.

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

*Résolu le 2026-11-13 : aucun des deux scénarios ci-dessous. Le drapeau est légitimement levé parce
que le client a commencé à charger tard, du temps de transfert de l'envoi groupé, et n'a pas encore
fini. Voir [Capture du 2026-11-13](#capture-du-2026-11-13). Les deux scénarios sont gardés pour
mémoire.*

Puisque `NetSnapshotProcessed` est un message système, l'accusé d'un client rapide devrait effacer
`+0x1a` chez l'hôte avant la fin du chargement de l'hôte, et aucun renvoi ne devrait partir. Deux
scénarios restaient possibles :

1. **L'hôte ne traite pas l'accusé avant `HandleSnapshots`.** *Écarté le 2026-11-08 pour un hôte
   lent ordinaire, voir [Captures du 2026-11-08](#captures-du-2026-11-08).* C'est le cas si la boucle de session
   de l'hôte est bloquée pendant la phase synchrone du chargement (désérialisation de GameCore), et
   si, à la reprise, `HandleSnapshots` passe avant le traitement des messages reçus. Dans le journal
   du 2026-10-29, l'hôte n'écrit rien de 12:57:57 à 12:58:25, et le client rien de 12:58:01 à
   12:58:25 : ni l'un ni l'autre n'a reçu de message dans cette phase. Le journal ne permet donc pas
   de trancher.
2. **Le client redemande un instantané lui-même.** *Écarté le 2026-11-13 : le client n'envoie aucun
   `NetSnapshotRequest`.* Arrivé en partie, le client passe la barrière de
   tour et envoie son contrôle de synchro (`NetGameSyncCheck`) à un hôte qui charge encore. Si
   l'hôte le compare à un état incomplet, le client reçoit une réparation qui échoue. Il demande
   alors l'instantané complet (`1407f8020` : `NetSnapshotRequest(joueur, raison 0)`), ce qui relève
   `+0x1a`. Le renvoi suit dès la fin du chargement de l'hôte.

Écarté : **MPH**. Son contrôle de seed, qui peut faire appeler `Network.RequestSnapshot()` au client
(`.mph_ui_resync_seed`), est commenté dans `MPHOptions.lua` (ligne 391). Le seul autre appel
(`.mph_ui_snap`, `VotePanel.lua`) est manuel.

## Captures du 2026-11-08

Captures faites avec [capture_double_load.ps1](capture_double_load.ps1) sur les deux machines
(archives dans [runs/20261108](runs/20261108)) : `DSHQ3VU`, la machine lente (12 cœurs, exe DX11),
et `OJR1UG7` (20 cœurs, DX12). Les écarts d'horloge NTP sont de −0,27 s et +0,51 s. **Le double
chargement ne s'est pas produit**, même dans la partie montée pour le provoquer. L'hôte change
d'une partie à l'autre : c'est la machine qui envoie l'instantané groupé (`SEND (0, x) NetSnapshot`).

| entrée | hôte (chargement) | client (chargement) | accusé du client reçu par l'hôte | renvoi |
|---|---|---|---|---|
| 19:24:12 | DSHQ3VU, 54 s | OJR1UG7, 21 s | 19:24:35, pendant le chargement de l'hôte | aucun |
| 19:28:13 | OJR1UG7, 23 s | DSHQ3VU, 64 s | 19:29:20, après | parti à 19:28:37, ignoré par le client (`load-in-progress`) |
| 19:31:45 (marqueur « lobby tentative double load ») | DSHQ3VU, 66 s | OJR1UG7, 22 s | 19:32:09, pendant | aucun |

Ce que ça établit :

- **La boucle réseau de l'hôte tourne pendant tout son chargement.** Dans le suivi à la
  milliseconde de l'hôte, à 19:31:45, l'accusé est écrit à 19:32:09.29, environ une seconde après
  l'envoi par le client. Les `NetGameCoreMessage` du client (des déplacements faits pendant que
  l'hôte charge) arrivent ensuite en direct, en file (`queued`), à 19:32:14, 19:32:15, 19:32:41 et
  19:32:44. L'accusé efface `+0x1a`, et `HandleSnapshots` ne renvoie rien à la fin du chargement de
  l'hôte (pas de `Requesting snapshot data from the app.`). Le scénario 1 ne se produit donc pas
  avec un hôte simplement lent. Il faudrait une phase où la boucle est vraiment bloquée au moment
  où l'accusé arrive, et les captures n'en montrent aucune.
- **Le cas hôte rapide (19:28) se déroule comme le 2026-10-29** : renvoi au client seul, ignoré
  parce que le client charge encore.
- **Un rechargement en pleine partie a bien cette forme dans le scénario 2.** La session de
  l'après-midi de DSHQ3VU (journaux copiés par la première capture, 16:41 → 19:18) en contient deux
  chez le client, à 17:29:19 et 17:30:28. Le contrôle de synchro de fin de tour échoue
  (`AutoArchive out of sync (doesn't exist on host) Requesting full snapshot`, unités et formations
  d'un joueur absentes chez l'autre), le client envoie `NetSnapshotRequest : PlayerID=1, Reason=0`,
  reçoit l'instantané et recharge (`LoadInProgress`, puis barrière). Ces deux-là arrivent un et
  deux tours après l'entrée, pas à l'entrée, et le jeu n'a pas planté. Ce sont des désynchros
  ordinaires, mais c'est la trace à chercher dans un vrai double chargement.
- L'indicateur « ne répond pas » du script ne repère pas les chargements : le jeu reste
  « répondant » pendant toute la durée de l'écran de chargement.

## Capture du 2026-11-13

Premier vrai double chargement capturé, des deux côtés (archives dans
[runs/20261113](runs/20261113)) : hôte `OJR1UG7` (joueur 6, UTC+1), client `HULKUS` (joueur 0,
UTC−5), un troisième joueur (1) sans capture. Les écarts d'horloge NTP sont de +0,50 s et +0,63 s.
Partie rechargée au tour 123 : l'instantané fait 2,9 Mo et passe à environ 265 Ko/s entre les deux
machines, soit **11 s de transfert**, à l'aller comme au renvoi. Heures de l'hôte, au dixième de
seconde (suivi du script) :

| heure | hôte | client |
|---|---|---|
| 18:52:57.4 | envoi groupé `NetSnapshot` 2 914 845, début du chargement | |
| 18:53:08.5 | | `NetSnapshot` reçu (11 s plus tard), `LoadGameViewState::Begin` |
| 18:53:18.98 | `NetSnapshotProcessed 6` | |
| 18:53:21.4 | `Queuing Messages End`, `Requesting snapshot data from the app.` ×2 | en chargement |
| 18:53:21.8 | `SEND (0)` et `SEND (1)` : renvoi de 2 913 189 octets | |
| 18:53:27.9 | | `SEND NetSnapshotProcessed 0` |
| 18:53:28.2 | accusé du client reçu, **après le renvoi** | |
| 18:53:31.45 | | `Queuing Messages End` : la garde est retombée |
| **18:53:32.95** | | **renvoi reçu, sans `ignored … load-in-progress`** : rechargement |
| 18:53:38 → 18:54:17 | | « ne répond plus » (39 s), puis `LoadGameViewState::Begin` |
| 18:54:23 → 18:56:41 | | bloqué dans `GAMECORE_UNSERIALIZE_GAMESTATE` (6 s d'habitude), le processus répond toujours |

Durées de chargement : client 22,5 s (de `LoadGameViewState::Begin` à `Queuing Messages End`), hôte
23,5 s. **Le client charge plus vite mais finit 10 s après l'hôte**, parce qu'il a commencé 11 s plus
tard. Le renvoi arrive 1,5 s après la fin de son chargement.

Ce que ça établit :

- **Le mécanisme statique est confirmé** : renvoi par `HandleSnapshots` à tout joueur sans accusé,
  rechargement sans condition par un client déjà en partie.
- **Scénario 1 écarté à nouveau** : l'accusé du client n'est pas retardé par l'hôte, il est
  simplement envoyé après la fin du chargement de l'hôte.
- **Scénario 2 écarté** : le client n'envoie aucun `NetSnapshotRequest` dans toute la session. Le
  renvoi arrive sans aucune demande depuis celle de l'envoi groupé (18:52:57).
- **Ce n'est pas un cas isolé.** L'entrée précédente de la même session (18:39:18, même
  sauvegarde) suit le même schéma : fin de l'hôte et renvoi à 18:39:43–44, accusé du client à
  18:39:49, puis `P2P Connection Failed … Timeout` pour ce client à 18:40:33 et reconnexion à
  18:41:01. Le joueur 1, plus lent, accuse 31 s après le renvoi : il l'a reçu en chargeant.
- **Pourquoi les captures du 2026-11-08 n'ont rien donné** : l'hôte y était 30 à 45 s plus lent que
  le client, hors de la fenêtre (accusé reçu avant la fin du chargement de l'hôte). Dans l'autre
  sens, l'hôte était 40 s plus rapide : renvoi reçu en chargement, ignoré.
- **Une reconnexion n'a pas de `NetSnapshotRequest`.** À la reconnexion du client (18:44:59, sa
  capture commence là), il reçoit `NetSnapshot` à 18:45:19 sans aucune demande avant. Une garde côté
  client doit donc accepter le premier instantané d'une connexion.
- **Le rechargement depuis la partie se fige.** Chez le client, juste avant le renvoi :
  `SESSION: Failed to create netmessage` deux fois, à la vidange de la file. Ensuite 44 s entre
  `Serialization Request` (18:53:32) et `OpenFromSource` (18:54:16), avec les options `00000000` au
  lieu de `00000010` pour les chargements d'entrée, puis plus rien après
  `GAMECORE_UNSERIALIZE_GAMESTATE Begin`. La raison de ce blocage reste ouverte. Elle n'est pas
  nécessaire au correctif, qui évite le rechargement.

## Vérifications dans l'exe installé (2026-11-14)

Les adresses des sections précédentes sont celles de l'exe du 2026-10-27 (projet `civ6exe`). L'exe
installé est celui du 2026-11-05 (même md5 que `~/Tools/ghidra-projects/bin1105`, projet
`civ6exe1105`, celui de `startup_proxy/src/relink.rs`). Adresses DX12 de cet exe :

| adresse | rôle |
|---|---|
| `141050c30` | vtable de `NetSnapshotRequest` ; `Process` = `1408110a0` (entrée `+0x20`, en `141050c50`) |
| `141050c58` | vtable de `NetSnapshot` ; `Process` = `140810fa0` (entrée en `141050c78`) |
| `141050c80` | vtable de `NetSnapshotProcessed` ; `Process` = `140811020` (déjà accrochée par `relink`) |
| `140fe2a10` | entrée `+0xe8` des callbacks de l'application : `ProcessSnapshot` = `140275ad0` |
| `1408179e0` | `SetWaitingForSnapshot(joueur, attend, raison)` (ancien `140817b80`) |
| `140817920` | constructeur de `FNetPlayer` (appelé par `CreateNetPlayer`, `140812f90`) |
| `140840260` | `HandleSnapshots` (hôte) |
| `140845f10` | envoi de l'instantané de l'application ; lève `+0x1b` |
| `140844ad0` | arrivée d'un joueur (« Matchmaking Complete ») |
| `1407f84b0` | resynchro complète de `FGameSyncManager` |
| `14026deb0` | `Network.RequestSnapshot()` en Lua |
| `1400f1ab0` | « la vue de jeu est l'état courant » |

Ce qui est établi :

- **`ProcessSnapshot` (`140275ad0`)** : si `140169bf0` (chargement en cours) est vrai, la branche
  « ignored » écrit la ligne de journal et **retourne, sans rien libérer** : les données
  appartiennent au message. Sinon, état de configuration `0xCA4009DE` ou `0x8356B191` →
  `0xCF36F2EE`, puis demande de chargement (`14016d980`).
- **`NetSnapshot::Process` (`140810fa0`)** : lève `+0x1b` du joueur local, appelle `ProcessSnapshot`
  seulement si les données sont non vides (`+0x40 != +0x48` et taille `+0x58 > 0`), puis appelle
  toujours `1407f8b80(FGameSyncManager)`. Celle-ci, si une resynchro complète est en attente
  (`+0x38`), l'efface et envoie `NetGameSyncProcessed`.
- **`NetSnapshotProcessed::Process` (`140811020`)** efface `+0x1a` **même avec `infoOnly=1`** ;
  avec `infoOnly=0`, il prévient en plus `FGameSyncManager` et l'application.
- **Le constructeur de `FNetPlayer`** écrit `0x10000` en `+0x18` : `+0x1a = 1` (attend un
  instantané), `+0x1b = 0`, `+0x1c = 0`, raison `+0x28 = -1`. **Tout nouveau joueur attend donc un
  instantané** : c'est ce qui déclenche l'envoi par `HandleSnapshots` à une reconnexion, sans
  `NetSnapshotRequest`. Une reconnexion crée un nouveau `FNetPlayer` (`CreateNetPlayer`), donc
  repart de `+0x1b = 0` chez l'hôte.
- **`140844ad0`, à l'arrivée d'un joueur**, efface `+0x1a` si l'application n'est pas en partie
  (callback `+0xc8`, état `0xCF36F2EE`) ou si c'est l'hôte. **Chez un client qui se reconnecte,
  le `+0x1a` local est donc probablement déjà à 0** quand l'instantané arrive : une garde fondée sur
  ce seul drapeau bloquerait la reconnexion.
- **Les deux demandes de resynchro passent par le joueur local.** `1407f84b0` et
  `Network.RequestSnapshot()` (`14026deb0`, via `140278340`) envoient
  `NetSnapshotRequest(local, 0)` par l'entrée `+0xb8` de la session, à tous y compris soi : le
  `+0x1a` local est relevé avant l'arrivée de l'instantané (vu le 2026-11-08 :
  `SEND (0, 1)` puis `RECV (1) :NetSnapshotRequest`).
- **L'état de vue courant** est en `[DAT_1412fb2c8] + 0x5420`. L'écran de chargement est
  `+0x14f0` (c'est le test « chargement en cours » de `140169bf0`), la vue de jeu `+0x14f8`
  (`1400f1ab0`). Le 2026-11-13, le renvoi arrive 1,7 s après `LoadGameViewState::End()` : la vue
  de jeu est alors l'état courant. À une reconnexion ou à l'entrée depuis le salon, c'est l'écran
  du menu.

## Correctif

Le renvoi ne sert à rien : l'envoi groupé a déjà mis toutes les machines sur le même état, et quand
le renvoi arrive pendant le chargement, le jeu l'ignore déjà sans dommage (2026-10-29, 2026-11-08).

- **Côté client : implémenté le 2026-11-14** dans le proxy de démarrage,
  [startup_proxy/src/snapshots.rs](../../startup_proxy/src/snapshots.rs). Il refuse un `NetSnapshot`
  quand **la vue de jeu est l'état courant et que le joueur local n'attend pas d'instantané**
  (`+0x1a == 0`). L'entrée `Process` de la vtable de `NetSnapshot` est remplacée, comme `relink.rs`
  le fait pour `NetSnapshotProcessed` :
  - le joueur local est lu par la session (`+0x148` puis `+0x158`), comme le fait l'original ;
  - la vue de jeu est testée en appelant la fonction du jeu elle-même (`1400f1ab0`) ;
  - pour refuser, la taille du message (`+0x58`) est mise à 0 le temps de l'appel à l'original,
    puis remise. L'original lève alors `+0x1b` et appelle `1407f8b80` comme d'habitude, mais
    n'appelle pas `ProcessSnapshot` : c'est l'équivalent exact de la branche « ignored », sans
    appeler de fonction interne ;
  - chaque refus est écrit dans le journal du proxy.

  Cas couverts :

  | cas | vue courante | `+0x1a` local | résultat |
  |---|---|---|---|
  | entrée depuis le salon | menu | 1 (demande raison 2) | accepté |
  | reconnexion | menu | 0 ou 1 | accepté |
  | resynchro complète, `Network.RequestSnapshot()` | jeu | 1 (demande raison 0 reçue de soi) | accepté |
  | rechargement demandé par l'hôte en partie | jeu | 1 (demande raison 2) | accepté |
  | renvoi pendant le chargement | chargement | 0 ou 1 | accepté, puis ignoré par le jeu |
  | **renvoi après le chargement (2026-11-13)** | jeu | 0 (accusé traité) | **refusé** |

  **Adresses DX11** (exe du 2026-11-05) : vtable de `NetSnapshot` en `1410417e8`, juste avant celle
  de `NetSnapshotProcessed` (`141041810`, celle de `relink`), comme en DX12. Ses cinq entrées
  (`14080c7a0`, `14080d950`, `14080f810`, `1408101b0`, `14080f230`) correspondent aux fonctions
  DX12, et les 103 premiers octets de `Process` sont identiques. Le test de la vue de jeu est en
  `1400f0d60`. Il diffère en un point : **l'état courant y est en `+0x5400`, pas en `+0x5420`**
  (gestionnaire en `1412c3c88`, chargement en `+0x14f0`, jeu en `+0x14f8`). C'est pourquoi le
  proxy appelle la fonction du jeu plutôt que de lire ces champs. Au démarrage, les octets sont
  vérifiés pour chaque exe, et `cargo test` vérifie que chaque table correspond à son exe installé,
  et à lui seul.

  Pas de réglage : c'est un correctif de bug pur. `actif=off` ne pose rien. **Pas encore essayé en
  partie** : il faut qu'un client finisse de charger après l'hôte, avec un écart plus petit que le
  temps de transfert de l'instantané (fin de partie, lien lent). Le journal du proxy doit alors
  montrer `snapshots: snapshot of … bytes received in the game view without being asked for,
  ignored`, et `net_message_debug.log` ne doit montrer aucun second `LoadInProgress`.
- **Côté hôte (non retenu)** : ne pas renvoyer l'instantané à un joueur qui a reçu l'envoi groupé du début,
  c'est-à-dire lever `+0x1b` pour tous les destinataires de l'envoi groupé. Ça marcherait aussi dans
  le cas du 2026-11-13, et une seule machine protégerait toute la partie. La reconnexion ne pose pas
  de problème (nouveau `FNetPlayer`, `+0x1b = 0`). Mais il faut accrocher l'envoi groupé, et la
  garde dépend alors de l'hôte seul.
