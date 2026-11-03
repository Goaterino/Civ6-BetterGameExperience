# Chances de succès des espions

Question : pourquoi les missions d'espionnage échouent-elles bien plus souvent que le pourcentage
annoncé par l'interface ? Analyse statique du 2026-11-03 sur `GameCore_XP2_FinalRelease.dll` (projet
Ghidra `~/Tools/ghidra-projects/civ6xp2`, base 0x180000000) et sur les scripts Lua et XML de Base, XP1
et XP2. Rien n'a encore été vérifié en partie : voir [À vérifier](#à-vérifier).

Les décompilations sont dans `~/Tools/ghidra-projects/spy_dump` à `spy_dump4`, et les appelants dans
`spy_callers*.tsv`. Les fonctions sont dans `Rules_Espionage.cpp` (`180371800..1803755c0`) et dans les
`Unit_Operations_Spy*.cpp`. Le calcul des tableaux ci-dessous est dans [proba.py](proba.py).

## Conclusion

**Le tirage est honnête, mais le pourcentage affiché ne tient pas compte du contre-espion du
défenseur, et il compte comme un succès l'issue « succès, mais l'espion doit s'échapper ».**

1. **Contre-espion absent du calcul de l'interface.** `UnitManager.GetResultProbability` appelle le
   calcul avec un contre-espion nul. Le tirage réel cherche le contre-espion du propriétaire de la
   case, à 1 case au plus de l'espion. S'il y en a un, il ajoute 3 + (niveau − 1) au jet à atteindre
   sur 3d6. Une mission annoncée à 50 % passe à 16 % contre une recrue, et à 9 % contre un agent. L'IA
   évalue ses missions avec la même fonction, donc avec la même erreur.
2. **« Succès, doit s'échapper » compté comme un succès.** L'interface additionne
   `SUCCESS_UNDETECTED` et `SUCCESS_MUST_ESCAPE`. Pour les trois missions de butin (vol de boost
   technologique, siphonnage de fonds, vol d'œuvre), le butin n'est versé que si l'évasion réussit.
   Pour les autres missions, l'effet est appliqué, mais l'espion risque ensuite d'être capturé ou tué.
3. **Évasion souvent ratée.** Elle se joue elle aussi sur 3d6, avec un seuil de 10 − (niveau − 1),
   soit 62,5 % de réussite pour une recrue. Si la police devine l'itinéraire, le seuil monte de 4 et
   la réussite tombe à 16 %. La police choisit un itinéraire même sans commissariat, et elle privilégie
   les sorties rapides.

Le pourcentage affiché est donc un plafond, valable seulement sans contre-espion. Le tirage lui-même
est correct : même table, générateur uniforme, sélection pondérée sans biais (à 0,3 % près).

## Le jet à atteindre : `ComputeNeededDieRoll`, `FUN_180372bf0`

`FUN_180372bf0(règles, opération, espion, contre-espion, case)` renvoie le jet N à atteindre sur
3d6. Les champs de la définition d'opération sont rangés par ordre alphabétique : `+0x1c`
BaseProbability, `+0x20` EnemyLevelProbChange, `+0x24` EnemyProbChange, `+0x28` LevelProbChange,
`+0x2c` Turns. Les paramètres globaux suivent le même ordre (`DAT_180b8aa70+0x2bc` =
`ESPIONAGE_BONUS_GAIN_SOURCES`, `+0x2c0` = `ESPIONAGE_ESCAPE_BASE_CHANCE`, etc.).

```
N = BaseProbability − bonus
bonus = (niveau espion − 1) × LevelProbChange
      + bonus de l'espion pour cette opération        (Unit_Espionage, 180395e00)
      + ESPIONAGE_BONUS_GAIN_SOURCES (2)              si la ville a été « sourcée » (1802d1220)
      + bonus du joueur (Player_Espionage+0xa8)
      + bonus de ses espions placés chez lui (+0x938, 1802d06c0)
      − [contre-espion] EnemyProbChange + bonus du contre-espion pour l'opération
                        + (niveau du contre-espion − 1) × EnemyLevelProbChange
      − défense du joueur cible (+0x93c de ses espions chez lui, 1802d05f0)
      − défense de la ville (+0xc0)
      − unités de la cible à 1 case avec une aura (+0x428)
      − défense du quartier visé (+0x15e0)
```

Le niveau est celui du jeu (`GetExperience():GetLevel()`, `GetSpyRankNameByLevel`) : 1 pour une
recrue sans promotion, puis 2 (agent), 3 (agent secret) et 4 (maître espion), une promotion par
niveau. Le « niveau 0 » du jargon des joueurs est donc le niveau 1 ici, et le terme `niveau − 1` est
le nombre de promotions.

Un contre-espion à 1 case de la cible gagne en plus `+0x944` niveaux (probablement une promotion).

Valeurs XML : BaseProbability vaut 13 à 16 selon la mission (13 pour siphonnage, agitation et
scandale, 14 pour vol de boost, sabotage et neutralisation, 15 pour vol d'œuvre, perturbation et
barrage, 16 pour partisans). LevelProbChange et EnemyLevelProbChange valent 1, EnemyProbChange vaut
3. Le scandale n'a ni EnemyProbChange ni EnemyLevelProbChange : un contre-espion ne le gêne que par
son bonus d'opération.

## Les six issues : `FUN_180374180`

`FUN_180373f10(lo, hi)` additionne la table `DAT_180b64c70`, qui est celle de 3d6 :
`0,0,0,1,3,6,10,15,21,25,27,27,25,21,15,10,6,3,1` (indices 0 à 18, total 216). Chaque issue reçoit
une tranche de jets, bornée :

| issue | indice | jets (3d6) |
|---|---:|---|
| succès non détecté | 5 | ≥ clamp(N, 8, 18) |
| succès, doit s'échapper | 4 | ≥ clamp(N−2, 7, 17), sous la tranche précédente |
| échec non détecté | 3 | ≥ clamp(N−3, 6, 16) |
| échec, doit s'échapper | 2 | ≥ clamp(N−5, 5, 15) |
| capturé | 1 | ≥ clamp(N−7, 4, 14) |
| tué | 0 | le reste, à partir de 3 |

Les bornes plafonnent le succès affiché à 90,7 % (N ≤ 8) et le laissent à 1,9 % au moins (N ≥ 19).

| N | tué | capturé | échec + fuite | échec | succès + fuite | succès | affiché |
|---:|---:|---:|---:|---:|---:|---:|---:|
| ≤ 8 | 0,5 | 1,4 | 2,8 | 4,6 | 6,9 | 83,8 | 90,7 |
| 9 | 0,5 | 1,4 | 2,8 | 4,6 | 16,7 | 74,1 | 90,7 |
| 10 | 0,5 | 1,4 | 7,4 | 6,9 | 21,3 | 62,5 | 83,8 |
| 11 | 0,5 | 4,2 | 11,6 | 9,7 | 24,1 | 50,0 | 74,1 |
| 12 | 1,9 | 7,4 | 16,7 | 11,6 | 25,0 | 37,5 | 62,5 |
| 13 | 4,6 | 11,6 | 21,3 | 12,5 | 24,1 | 25,9 | 50,0 |
| 14 | 9,3 | 16,7 | 24,1 | 12,5 | 21,3 | 16,2 | 37,5 |
| 15 | 16,2 | 21,3 | 25,0 | 11,6 | 16,7 | 9,3 | 25,9 |
| 16 | 25,9 | 24,1 | 24,1 | 9,7 | 11,6 | 4,6 | 16,2 |
| 17 | 37,5 | 25,0 | 21,3 | 6,9 | 7,4 | 1,9 | 9,3 |
| 18 | 50,0 | 24,1 | 16,7 | 4,6 | 4,2 | 0,5 | 4,6 |

## Le tirage réel : `FUN_180373cf0`

Chaque fin de mission (`1806764d0` vol d'œuvre, `180679d20` vol de boost, `180678e50` siphonnage,
`180678060`, `180674aa0`, `180716c70`, `180717aa0`, `180719060`, `180719d50`, `18074dbe0`,
`18067b590`) fait :

1. `FUN_180373920(case de l'espion)` : cherche un contre-espion. C'est une unité du propriétaire de
   la case, en opération `SPY_COUNTERSPY`, à 1 case au plus de l'espion (ou, avec le drapeau
   `+0x958`, dans le même quartier) ;
2. `FUN_180373cf0(op, espion, contre-espion)` : construit les six poids avec `FUN_180374180` et la
   case de l'espion (`+0xf8`), tire `GetRand(216, "Rolling Espionage Result")` sur le générateur
   synchronisé, puis prend la première issue dont la somme des poids dépasse le tirage.

Le générateur (`FUN_1805fa040`) est un LCG (`x × 0x41c64e6d + 0x3039`), réduit par
`(x >> 16) × 216 >> 16`. Les 216 valeurs sortent à 303 ou 304 chances sur 65 536 : l'écart est
inférieur à 0,3 %.

## Ce que montre l'interface : `GetResultProbability`, `FUN_1806c2740`

`EspionageSupport.lua` (choix de mission) et `EspionagePopup.lua` appellent
`UnitManager.GetResultProbability(op, espion, case visée)`. Le binding `1806c2740` passe par
`FUN_1800d14f0`, qui appelle `FUN_180374000(…, contre-espion = 0, case)`. Celle-ci renvoie pour
chaque issue `poids / 216`, en virgule fixe sur 8 bits, tronquée (perte de 0,4 % au plus).

L'interface affiche alors `SUCCESS_UNDETECTED + SUCCESS_MUST_ESCAPE`, arrondi. Les deux écarts avec
le tirage réel sont :

- **le contre-espion** : 0 côté interface, cherché côté tirage. Il n'est jamais pris en compte, même
  s'il est visible ;
- **la case** : la case visée côté interface (le quartier, ou le centre-ville), la case de l'espion
  côté tirage. C'est la même si l'espion se place sur le quartier pendant la mission (à vérifier).

L'IA (`18045d280`, `18045ea50`) évalue ses missions avec `FUN_180374000(…, 0, …)`, sans contre-espion
non plus.

## Succès, mais l'espion doit s'échapper

| mission | issue 4 |
|---|---|
| vol de boost (`180679d20`) | boost mis de côté (`+0x94c`), puis évasion |
| siphonnage (`180678e50`) | or mis de côté (`+0x950`), puis évasion |
| vol d'œuvre (`1806764d0`) | œuvre mise de côté (`+0x954`), puis évasion |
| sabotages, agitation, scandale, neutralisation, partisans, barrage | effet appliqué tout de suite, puis évasion |

Le butin mis de côté n'est versé que par `AwardLoot` (`180371d70`), qui n'est appelé que par
`ResolveEscape` (`180375100`) quand l'évasion réussit (issue 2). Si l'espion est capturé ou tué, le
butin est perdu.

## L'évasion : `ResolveEscape`, `FUN_180375100`

1. `ArePoliceInPosition` (`180371b10`, « Police Exit Covered ») : la police choisit une sortie parmi
   les quartiers de la ville qui ont un `TravelTime` : aérodrome 1, port 2, plateforme commerciale 3,
   centre-ville 4. Le poids d'une sortie est `max − TravelTime + 1` (aérodrome 4, port 3,
   plateforme 2, centre-ville 1). Rien ne demande de commissariat. La police a raison si elle choisit
   la sortie de l'espion (`+0x948`). Une ville réduite à son centre a une seule sortie, que la police
   devine à coup sûr.
2. `FUN_180373ac0(espion, contre-espion = 0, police)` : tirage sur 216 avec le seuil
   `E = 10 − [(niveau − 1) × 1 + bonus d'évasion (+0x934) + (police ? −4 : 0)]`. Les tranches sont :
   évadé pour E à 18, capturé pour E−2 à E−1, tué pour 3 à E−3.
3. Le contre-espion est passé nul : `ESPIONAGE_ESCAPE_COUNTERSPY_LEVEL_MODIFIER` (−1 par niveau) ne
   sert jamais. Une capture peut être changée en mort sous une condition liée au type d'un quartier et
   à celui du joueur (`18005e770`, `1802f9890`). Cette condition n'a pas été identifiée.

Le bonus d'évasion `+0x934` vient de la promotion As du volant (`PROMOTION_SPY_ACE_DRIVER`,
`MODIFIER_PLAYER_UNIT_ESCAPE_BOOST`, `Amount` = 4) : « +4 niveaux » à l'évasion, soit E − 4. Avec
elle, E vaut 5 pour un espion à 1 promotion (98,1 % d'évasion), 4 à 2 promotions (99,5 %) et 3 à
3 promotions (100 %). Si la police devine la sortie, E vaut 9, 8 ou 7 (74,1 %, 83,8 %, 90,7 %).

| E | tué | capturé | évadé |
|---:|---:|---:|---:|
| 3 (As du volant, 3 promotions) | 0 | 0 | 100 |
| 5 (As du volant, 1 promotion) | 0 | 1,9 | 98,1 |
| 7 (espion de niveau 4) | 1,9 | 7,4 | 90,7 |
| 8 | 4,6 | 11,6 | 83,8 |
| 9 | 9,3 | 16,7 | 74,1 |
| 10 (recrue) | 16,2 | 21,3 | 62,5 |
| 13 (niveau 2, police) | 50,0 | 24,1 | 25,9 |
| 14 (recrue, police) | 62,5 | 21,3 | 16,2 |

## Ordres de grandeur

Mission à BaseProbability 14 (vol de boost, sabotage), sans autre modificateur. « Butin » est la
chance de toucher réellement le butin d'une mission de butin, sans contre-espion.

| niveau | affiché | contre-espion 1 | contre-espion 2 | contre-espion 4 | butin | butin, police devinée |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 37,5 | 9,3 | 4,6 | 1,9 | 29,5 | 19,7 |
| 2 | 50,0 | 16,2 | 9,3 | 1,9 | 43,8 | 32,2 |
| 3 | 62,5 | 25,9 | 16,2 | 4,6 | 58,4 | 46,9 |
| 4 | 74,1 | 37,5 | 25,9 | 9,3 | 71,8 | 62,0 |

Un contre-espion de niveau 1 ajoute 3 au jet à atteindre. Vers le milieu de la courbe de 3d6, cela
divise la chance de succès par deux à quatre.

## Pistes de correctif

- **Afficher la vraie probabilité.** Détourner `FUN_1800d14f0` pour passer
  `FUN_180373920(case)` au lieu de 0. Ce n'est que de l'interface, sans risque de désynchro, mais cela
  révèle la présence d'un contre-espion. Il faut donc un réglage, et sans doute un correctif POL en
  multijoueur (actif seulement si tous les joueurs ont le proxy).
- **Afficher les issues séparément.** Montrer le succès non détecté à part du succès avec fuite, et,
  pour les missions de butin, le succès multiplié par la chance d'évasion. Lua seul, sans fuite
  d'information.
- **Évasion contre le contre-espion.** Passer le contre-espion à `FUN_180373ac0` réactiverait
  `ESPIONAGE_ESCAPE_COUNTERSPY_LEVEL_MODIFIER`. Cela change le gameplay et durcit encore l'espionnage :
  à ne faire qu'en option.

## À vérifier

- En partie, lire N et les six poids au moment du tirage (`180373cf0`), avec et sans contre-espion,
  et les comparer à l'affichage. Un hook Frida ou le proxy suffit, et le journal du générateur écrit
  déjà `Rolling Espionage Result` quand il est actif.
- Que l'espion est bien sur la case du quartier visé pendant la mission, ce qui rendrait identiques
  la case de l'interface et celle du tirage.
- Le sens de `+0x944` (bonus de niveau du contre-espion proche) et de `+0x958` (contre-espion qui
  couvre tout le quartier), probablement des promotions.
- La condition qui transforme une capture en mort pendant l'évasion.
