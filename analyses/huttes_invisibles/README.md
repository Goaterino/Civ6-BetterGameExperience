# Villages tribaux invisibles après un rechargement

**Corrigé pour la 0.15.0** (2026-11-24, `proxy/src/partie_chargee.rs`) : pendant la passe
post-chargement, l'écriture de la mémoire d'un joueur a lieu, le temps que l'étape relise celle du
joueur local et lève l'affichage, puis elle est défaite à la case suivante et à la fin de la passe.
Les couches restent celles de la sauvegarde (synchro inchangée), l'affichage redevient celui du jeu
d'origine. Le journal dit désormais `… remembered-improvement write(s) of the players undone after
the display check`. Vérifié hors jeu (octets du getter sur 1.0.12.58 et .74) ; pas encore en partie.

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
été retrouvées par leur comportement, puis **retrouvées et revérifiées le 2026-11-23 sur les
GameCore 1.0.12.58 (même code que la 1.0.12.72, adresses du proxy) et 1.0.12.74**. Chaque fonction
de la .74 est identique à celle de la .58, au déplacement des adresses relatives près.

| Rôle | Build 5ee10d21 | 1.0.12.58 / .72 (proxy) | 1.0.12.74 |
|---|---|---|---|
| Passe post-chargement (4 étapes) | `18004ef70` | `18063def0` (appelle l'étape par `180053c40`) | `18063df70` |
| Étape des améliorations | `180248ba0` | `18025cb50` | `18025cb30` |
| Setter de la mémoire d'amélioration | `180341ed0` | `18035de10` | `18035d6d0` |
| Getter de la mémoire d'amélioration | `18033f610` | `18035b4a0` | `18035ad60` |
| Amélioration posée sur la carte | `1802489a0` | `18025c950` | `18025c930` |
| Changement de visibilité d'une case | `180340000` | `18035c020` | `18035b8e0` |

Constats sur la .58 :
- le setter n'a que deux appelants, `18025c950` (deux appels) et l'étape (deux appels), comme en
  2020 ;
- dans l'étape, après le second appel au setter, le getter est appelé avec le joueur local
  (`180044df0`) et l'indice de la case (`18005f2d0`), et comparé à l'amélioration de la case ;
  `0xf391d7b3` n'est levé (`18025cd62`) que s'ils sont égaux. L'état transmis vaut 2 si la case est
  visible pour le joueur local (couche de `18035b620`, `+0x120`), sinon 1 si elle est révélée
  (`+0x138`) ;
- `18035c020` teste le drapeau 8 (`18035c54f`), puis `case+0x44 != -1`, et lève `0xf391d7b3`
  (`18035c56b`) sans lire la mémoire.

Le correctif se sert du setter et du getter, tous deux dans `patch::MOVED`.

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
