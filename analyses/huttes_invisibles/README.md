# Villages tribaux invisibles après un rechargement

**Pas encore corrigé** (2026-11-23) : cause identifiée, correctif à écrire.

Rapports BGE-2C7D19, BGE-E8B107 et BGE-93D8D4, tous en 0.14.0. Symptôme : après un relobby, un
rechargement contre une désynchro ou une resynchro, des villages tribaux (huttes) ne sont plus
affichés sur la carte.

## En bref

- **Cause.** C'est le correctif POL « améliorations mémorisées » de la 0.14.0
  (`proxy/src/partie_chargee.rs`, `visibilite_chargee`, PR #16). Pendant la passe post-chargement,
  `set_remembered_hook` saute toutes les écritures d'amélioration mémorisée des joueurs. Or c'est
  de cette écriture que dépend, juste après, l'événement qui affiche l'amélioration pour le joueur
  local.
- **Aucun contrôle de synchro ne peut le voir.** Depuis la 0.13, toutes les machines, l'hôte
  compris, rechargent la même sauvegarde et sautent les mêmes écritures. Le défaut est purement
  visuel, et l'hôte le subit aussi.
- **Piste de correctif.** Garder les couches des joueurs telles que sauvegardées (ce qui préserve
  la synchro), et lever quand même l'événement d'affichage pour le joueur local. Il serait calculé
  à partir de l'amélioration réelle de la case et de son état de visibilité, sans écrire la
  mémoire.

## Rapports

| Rapport | Partie | Machine | Dernier chargement avant le rapport |
|---|---|---|---|
| BGE-2C7D19 | A, 11 humains, tour 36 | client | 61 écritures sautées |
| BGE-E8B107 | A (mêmes horodatages de chargement) | client | 61 écritures sautées |
| BGE-93D8D4 | B, 8 humains, tour 9 | hôte | 28 écritures sautées |

Cela fait deux parties indépendantes. Les journaux du proxy montrent la ligne
`loaded game: post-load pass done, N remembered-improvement write(s) of the players skipped` à
chaque chargement. Elle ne s'écrit que si les correctifs POL et `visibilite_chargee` sont actifs.
Aucune réparation en place de la 0.14.0 n'a eu lieu entre le dernier chargement et le rapport.

## Code du jeu (analyse statique)

L'analyse porte sur `GameCore_XP2_FinalRelease.dll` du 2020-06-10 (horodatage PE `0x5ee10d21`,
SHA-256 `a4600a61…1d5a`), une build **antérieure** aux versions prises en charge. Les fonctions ont
été retrouvées par leur comportement. **À revérifier sur 1.0.12.72/74** avant d'écrire le
correctif.

| Rôle | Build 5ee10d21 | 1.0.12.72 (proxy) |
|---|---|---|
| Passe post-chargement (4 étapes) | `18004ef70` | `18063def0` |
| Étape des améliorations | `180248ba0` | `18025cb50` |
| Setter de la mémoire d'amélioration | `180341ed0` | `18035de10` |
| Getter de la mémoire d'amélioration | `18033f610` | ? |
| Amélioration posée sur la carte | `1802489a0` | ? |
| Changement de visibilité d'une case | `180340000` | ? |

1. **Setter (`180341ed0`).** Il écrit un `short` dans `Visibility::Instance.m_aExplicitLayers`
   (la couche indexée par `mgr+0x18`) et renvoie vrai si la valeur a changé. Il ne lève aucun
   événement.
2. **Étape des améliorations de la passe post-chargement (`180248ba0`).** Pour chaque case qui
   porte une amélioration (`plot+0x44 != -1`) :
   - elle lève `0xe5b20b82` (amélioration sur la carte, avec son propriétaire) ;
   - elle appelle le setter pour chaque joueur qui a révélé la case, puis pour l'instance
     partagée (1000) ;
   - elle relit la mémoire du **joueur local** avec le getter, et **ne lève `0xf391d7b3`, la
     visibilité de l'amélioration pour l'affichage, que si cette mémoire est égale à l'amélioration
     de la case**. L'état transmis vaut 2 si la case est visible, 1 si elle est seulement révélée.
3. **Où va `0xf391d7b3`.** `Cache_Map_EventStream` (`18000c070`) l'enregistre, avec le
   gestionnaire `18000b300`, filtré sur le joueur local. C'est le cache de carte que lit le rendu.
   Ce cache n'est pas sauvegardé : après un chargement, il n'est reconstruit que par ces passes.
4. **Dans une partie en cours, la mémoire d'amélioration d'un joueur n'est écrite qu'à deux
   endroits.** Le setter n'a que deux appelants :
   - `1802489a0` : une amélioration est posée sur la carte, et la mémoire n'est écrite que pour les
     joueurs qui ont **déjà** révélé la case ;
   - la passe post-chargement.

   Une hutte posée à la génération de la carte, avant toute révélation, n'entre donc jamais dans la
   mémoire d'un joueur. En cours de partie, elle s'affiche quand même par un autre chemin,
   `180340000` (changement de visibilité d'une case, avec le drapeau 8). Ce chemin lève
   `0xf391d7b3` dès que la case porte une amélioration, sans lire la mémoire.

### Conséquence

La mémoire est bien sérialisée et restaurée, mais dans une partie en cours elle ne contient pas les
huttes posées avant la révélation de leur case. Le moteur s'en passe tant que le cache d'affichage
vit. Après un chargement, la passe post-chargement remplit d'abord cette mémoire, puis s'en sert
pour décider d'afficher. Avec le saut : `getter(joueur local) == -1 ≠ hutte`, donc pas de
`0xf391d7b3`, donc la hutte est absente du cache.

Les deux constats sont cohérents :
- celui de la PR #16 : les archives de visibilité diffèrent après un rechargement ;
- celui des joueurs : les huttes sont invisibles.

## Attendu en partie (non vérifié)

1. Seules les huttes en brouillard, sur des cases déjà révélées, manquent après le chargement. Les
   huttes visibles passent par le chemin de visibilité, si le chargement le déclenche.
2. Une hutte disparue réapparaît quand une unité revoit la case (`180340000`).
3. Rien de tout ça dans une partie sans POL, ni avec un build `diag` réglé sur
   `visibilite_chargee=off`.
