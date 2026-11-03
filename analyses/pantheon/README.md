# Obtention du panthéon

Question : quand deux joueurs atteignent le coût du panthéon (12 de foi en vitesse en ligne) au même
moment, lequel peut le fonder en premier ? Et pourquoi un joueur doit-il parfois attendre un tour ?
Analyse statique du 2026-10-31, sur `GameCore_XP2_FinalRelease.dll` (projet Ghidra
`~/Tools/ghidra-projects/civ6xp2`, base 0x180000000), et les scripts Lua de Base. Rien n'a encore été
vérifié en partie : voir [À vérifier](#à-vérifier).

Les décompilations sont dans `~/Tools/ghidra-projects/pantheon_dump`, avec les appelants dans
`callers.tsv`. REA n'a pas pu ouvrir la DLL : son import Ghidra dépasse le délai de démarrage (330 s),
sur la Release comme sur la FinalRelease. Le projet déjà analysé a donc été relu en
`-noanalysis -readOnly` avec `DumpAddrs.java` et `CallersOf.java`.

## Conclusion

**Il n'y a pas de course libre : à chaque tour, un seul joueur est désigné pour le panthéon, et seule
l'interface de ce joueur peut le fonder.** La désignation va au joueur sans panthéon qui a le plus de
foi. Elle est recalculée une fois par tour, au début du tour de jeu, et **elle ne peut pas aller deux
tours de suite au même joueur**, sauf s'il est le seul candidat. C'est la restriction d'un tour.

- En cas d'égalité, le premier joueur dans la liste des majeurs vivants l'emporte, puisque la
  comparaison est stricte. Il y a une exception : si ce joueur était déjà le désigné du tour précédent,
  il est exclu.
- La foi utilisée pour désigner est celle d'avant le gain du tour.
- Le serveur ne revérifie ni la désignation ni la foi quand il exécute l'opération `FOUND_PANTHEON`.
  Le verrou n'existe que dans l'interface (et dans le choix de l'IA).

## Désignation : `FUN_1802429f0`

`Game_Religion` (instance renvoyée par `180242b20`) garde un `SyncedValue` à `+0xf0`, dont la valeur à
`+0x100` est le **joueur du panthéon**. `FUN_1802429f0` le recalcule :

1. il retient l'ancien désigné (`prev`), puis écrit -1 ;
2. il parcourt la liste des majeurs vivants (`Game+0xb50..0xb58`, celle de
   `PlayerManager.GetAliveMajors`, `1806f4730`). Un joueur devient le nouveau désigné si :
   - il n'a pas de panthéon (`1803158a0` : `PlayerReligion+0xe0 != -1`) ;
   - son identifiant n'est pas `prev` ;
   - c'est le premier candidat rencontré, **ou** sa foi (`PlayerReligion+0xb0`, en 1/256) est
     **strictement** supérieure à celle du désigné provisoire ;
3. si aucun candidat n'a été retenu et que `prev` n'a toujours pas de panthéon, `prev` est conservé.

Le coût n'entre pas dans ce choix : le premier candidat est retenu même à 0 de foi, et le désigné peut
être sous le coût.

### Moment du calcul

`FUN_1802429f0` est appelé par `1807507f0`, la méthode `+0x28` de la vtable du jeu (`180a45160`, fichier
`Target_Game_Instance`). `Game::Processor::BeginTurn` (`180049390`) l'appelle juste après avoir
incrémenté le tour, puis `ActivatePlayersAtTurnStart` (`180049170`).

L'activation (`18005de40`) ne fait que lever un indicateur par joueur (`DAT_180b8adb0[joueur]`). Le
`DoTurn` du joueur (`18005ca80`) tourne ensuite, au premier `Player::Processor::Update` (`18005e2d0`).
Il appelle `PlayerReligion::DoTurn` (`180314c80`), qui ajoute la foi du tour (`180313ee0`).

La désignation du tour T voit donc la foi stockée à la fin du tour T-1, sans le gain du tour T.

## Vérification : `CanCreatePantheon`, `FUN_180313e30`

`PlayerReligion::CanCreatePantheon(bCheckFaith)` est vrai si, dans l'ordre :

1. la capacité `0xb9fc347c` est active (son nom n'a pas été retrouvé) ;
2. le joueur n'a pas de panthéon (`+0xe0 == -1`) ;
3. il n'a pas fondé de religion (`180243410` : pas d'entrée positive dans le vecteur de
   `Game_Religion+0xb8`, pas `0x90`) ;
4. **`Game_Religion+0x100` est son identifiant** ;
5. si `bCheckFaith` : `coût × 256 <= foi`. Le coût est `Game_Religion+0xe8`, renvoyé par
   `GetMinimumFaithNextPantheon` (`18009bf70`). La foi est lue en direct.

Qui s'en sert :

- l'interface : `LaunchBar.lua` (ouverture du sélecteur), `ReligionScreen.lua`, et la notification
  `CHOOSE_PANTHEON` (`180652010` la garde tant que le joueur est humain, sans panthéon et que
  `CanCreatePantheon(true)` est vrai) ;
- l'IA : `PlayerReligion::DoTurn` refait le même test juste après le gain de foi. Pour un joueur non
  humain (`1802f9820`), il lève l'indicateur de choix de l'IA (`1802f80e0(...)+0xe8`). Une IA désignée
  et assez riche fonde donc dès le début de son tour, alors qu'un humain doit cliquer pendant le sien.

## Fondation : `FoundPantheon`, `FUN_180242d00`

L'opération joueur `FOUND_PANTHEON` (hachage `-0x698e382f`) est exécutée par `18030d330`
(`Player_Operations_Manager.cpp`). Le cas appelle directement `FUN_180242d00` **sans
`CanCreatePantheon`**. La validation préalable `18030c100` ne regarde que l'état du tour du joueur
(tour actif, `Player+0x288`).

`FUN_180242d00(joueur, croyance)` vérifie seulement :

- que la croyance est valide (`1801b0220`) et le joueur aussi ;
- que la croyance n'est dans aucun panthéon (`1802436b0`, sur tous les joueurs ayant existé,
  `Game+0x5d0`) ;
- que le joueur n'a pas de panthéon.

Puis il pose la croyance, ajoute `-coût × 256` à la foi **sans plancher** (`180313ee0` ne borne qu'aux
limites d'un `int`), applique la croyance aux villes et journalise `Found Pantheon` puis
`Adopt Pantheon` (`180243d90`). Il ne touche pas au joueur désigné, qui reste fixé jusqu'au prochain
début de tour. Une fois que le désigné a fondé, plus personne ne peut passer `CanCreatePantheon`
avant le tour suivant.

Les fonctions Lua `Game.GetReligion():FoundPantheon(joueur, index)` (`1806e0dd0`) et
`FoundPantheonHash` (`1806e0e30`) appellent la même fonction, elles aussi sans vérification.

## Conséquences

### Égalité à 12 de foi

Si A et B ont tous les deux 12 de foi au début du tour T, sans panthéon :

- A est désigné s'il est avant B dans la liste des majeurs vivants et qu'il n'était pas le désigné
  de T-1 ;
- s'il l'était, B est désigné, même avec moins de foi que A ;
- l'autre est désigné au tour T+1 (le premier a un panthéon, ou a été désigné en T).

### Alternance

Avec au moins deux joueurs sans panthéon, le plus riche n'est désigné qu'un tour sur deux. Avec
A > B > C en foi, la suite est A, B, A, B… et C n'est jamais désigné tant que A et B n'ont pas de
panthéon.

Si A est désigné au tour T alors qu'il lui manque de la foi, le tour T+1 va à B, même si A atteint 12
entre-temps. A ne peut fonder qu'au tour T+2.

### Contournements

1. **Être le seul candidat.** Si tous les autres majeurs vivants ont déjà un panthéon, ou s'il n'y en a
   pas d'autre, le désigné est conservé d'un tour sur l'autre.
2. **Atteindre le coût pendant son tour désigné.** La foi de `CanCreatePantheon` est lue en direct,
   donc un gain de foi immédiat avant la fin du tour suffit.
3. **Envoyer l'opération sans passer par l'interface normale.** Une interface modifiée qui appelle
   `UI.RequestPlayerOperation(joueur, PlayerOperations.FOUND_PANTHEON, ...)`, ou un script de gameplay
   qui appelle `FoundPantheon`, fonde pendant n'importe quel tour actif, sans être désigné et sans la
   foi : le coût est retiré quand même, et la foi devient négative. L'opération étant exécutée de la
   même façon chez tous, cela ne désynchronise pas.

## À vérifier

- L'ordre de la liste des majeurs vivants (`Game+0xb50`) : très probablement l'ordre des identifiants
  de joueur, mais la fonction qui la construit n'a pas été lue.
- Le nom de la capacité `0xb9fc347c`.
- En partie : deux joueurs à 12 de foi au même tour, puis un joueur désigné sous le coût au tour T qui
  atteint le coût en T, pour observer l'attente jusqu'à T+2. `Game_Religion+0x100` peut être lu depuis
  le proxy ou par Frida.
