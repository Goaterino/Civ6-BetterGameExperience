# Vision des cités-États reprises après une guerre

**Corrigé pour la 0.15.x** (2026-11-24, [proxy/src/vision_guerre.rs](../proxy/src/vision_guerre.rs)),
correctif POL. Vérifié hors jeu (octets des sites sur 1.0.12.58 et .74, détours posés par
`check_hooks.py`) et essayé en partie le 2026-11-24 sur une sauvegarde touchée : la vision revient.
Au changement de suzeraineté, la cité-État (joueur 12) a fait la paix avec 12 joueurs dont les
entrées avaient des compteurs de 8 à 36, toutes retirées.

Symptôme, surtout en Teamer (deux équipes en guerre permanente) : quand un joueur reprend la
suzeraineté d'une cité-État avec laquelle il a été en guerre, il ne voit plus les unités de la
cité-État. Le premier suzerain de la partie, qui n'a jamais été en guerre avec elle, la voit.

## En bref

- **Partage de vision.** Chaque instance de visibilité d'un joueur tient deux listes à compteur :
  les joueurs avec qui elle partage sa vision (`+0x368`) et ceux pour qui ce partage est suspendu
  par une guerre (`+0x340`). Le partage n'ajoute la vision que si le destinataire n'est pas dans la
  seconde (`180357b30`) ; chaque déplacement d'unité est filtré de même (`18035a7f0`).
- **Suzeraineté.** `1802edf10` (mise à jour du suzerain) fait faire la paix à la cité-État avec les
  ennemis de l'ancien suzerain, retire la vision à l'ancien (`18035d4e0`), fait déclarer la guerre
  aux ennemis du nouveau (type `0x5064ce64`), puis lui donne la vision (`180359f00`).
- **Défaut.** L'état guerre/paix entre deux joueurs (`1801d31b0`) met à jour la liste suspendue
  par trois appels `(p3,p2)`, `(p3,p2)`, `(p2,p3)`, à la guerre (`18035a4b0`) comme à la paix
  (`18035d8c0`). Une inscription reste après la paix :
  - quand la paix est faite par le camp qui n'a pas déclaré la guerre (déclaration
    `1802a7100` : `(déclarant, cible)` ; paix `1802a7710` : `(celui qui fait la paix, l'autre)`) ;
  - quand la guerre a été déclarée plusieurs fois. `CanDeclareWar` (`1802a0180`) ne teste pas
    « déjà en guerre » pour le type suzerain, et la branche équipes de `1802a1f90` déclare à chaque
    membre : en Teamer, la cité-État déclare la guerre à chaque ennemi autant de fois que l'équipe
    a de membres, et ne fait la paix qu'une fois au changement de suzeraineté.
- **Conséquence.** Le nouveau suzerain reste inscrit dans la liste suspendue de la cité-État :
  `180359f00` l'ajoute à la liste de partage sans lui donner la vision, pour toujours.

Les listes suspendues ne servent qu'à la guerre (seul `1801d31b0` y écrit). Le même défaut coupe
aussi la vision entre alliés militaires ou membres d'une urgence après une ancienne guerre.

## Correctif

Deux détours, actifs si `pol::actifs()` :
- **Retrait** (`18035d8c0`) : à la paix, l'entrée est retirée en entier. Le retrait d'origine est
  appelé autant de fois que son compteur ; le dernier appel la retire et rend la vision, comme le
  jeu pour une entrée à 1.
- **Début de tour** (`BeginTurn`, `180049390`) : avant le tour, toute entrée suspendue entre deux
  joueurs vivants qui ne sont pas en guerre (`1801d1ed0`) est retirée de la même façon. Ça répare
  les parties déjà touchées. Un rechargement n'y suffirait pas : un joueur qui revient charge seul,
  sans l'hôte, et la visibilité fait partie du contrôle de synchro.

Seules les fonctions du jeu écrivent, dans le même ordre sur chaque machine. Journal :
`war suspension: peace, entry … had count N, removed whole` et
`war suspension: turn start, N stale entr(ies) between players at peace removed: …`.

Non couvert : une cité-État vraiment en guerre avec son nouveau suzerain. `1802edf10` ne fait pas
la paix avec un majeur qui lui a déclaré la guerre lui-même ; la vision revient à la paix.

## Adresses

| Rôle | 1.0.12.58 | 1.0.12.74 |
|---|---|---|
| Mise à jour du suzerain | `1802edf10` | `1802edf20` |
| Partage / retrait de vision | `180359f00` / `18035d4e0` | `1803597c0` / `18035cda0` |
| Guerre et paix (listes suspendues) | `1801d31b0` | `1801d3870` |
| Inscription / retrait (liste suspendue) | `18035a4b0` / `18035d8c0` | `180359d70` / `18035d180` |
| `BeginTurn` | `180049390` | `1800493a0` |
| En guerre / diplomatie du jeu | `1801d1ed0` / `1801d0220` | `1801d2590` / `1801d08e0` |

Instance de visibilité : gestionnaire global `180b8e1f8` (`+8` indice par joueur, `+0x30`
instances) ; liste suspendue en `+0x350` (entrées `{i32 joueur, i32 compteur}`), nombre en
`+0x360`. Joueur vivant : `Player+0x258`.

## Vérifier en partie

En 2v2 (hotseat ou FireTuner) : A1 suzerain, B1 lui prend la cité-État, A1 la reprend. Sans le
correctif, A1 ne voit pas les unités de la cité-État ; avec, il les voit dès la reprise. Sur une
sauvegarde déjà touchée, la vision revient au début du tour suivant, avec la ligne du journal.
