# Plan : afficher le vrai taux de succès des espions

Plan d'intégration du 2026-11-23, rien n'est encore implémenté. Il s'appuie sur l'analyse
[README.md](README.md). Les fichiers Lua d'espionnage du jeu manquaient sur la machine de rédaction :
les étapes qui les lisent restent à faire sur une machine qui a le jeu.

## Objectif

Le pourcentage affiché au choix d'une mission devient :

```
affiché = P(succès non détecté) + P(succès, doit s'échapper) × P(évasion)
```

Ce chiffre ne tient pas compte du contre-espion du défenseur : le joueur n'est pas censé savoir s'il
y en a un. Aujourd'hui, l'interface affiche `P(succès non détecté) + P(succès, doit s'échapper)`.

## Choix retenus

- **Itinéraire de fuite : la meilleure sortie.** L'itinéraire se choisit après la détection.
  L'affichage suppose que le joueur prendra la sortie que la police devine le moins : le
  centre-ville, de poids 1. La police la devine donc avec la probabilité `1 / W`, où `W` est la
  somme des poids des sorties de la ville. Une ville réduite à son centre a `W = 1` : la police
  devine la sortie à coup sûr.
- **Toutes les missions.** La formule s'applique à toutes les missions, y compris celles sans butin
  (sabotages, agitation, scandale, neutralisation, partisans, barrage), dont l'effet est appliqué
  même si l'espion est pris ensuite. Le chiffre y mesure donc le succès avec retour de l'espion.
- **Ajout d'interface réglable.** C'est un nouveau réglage `ui_espions=on|off`, actif par défaut. Il
  va dans `NOUVEAUTES_UI` et porte `nouveaute_ui = true` dans l'onglet. Il n'est pas masqué par
  l'option de partie « Disable Advanced UI indicators (BGE) » : il corrige un chiffre déjà affiché, il
  n'ajoute pas d'information.

## Calcul

`UnitManager.GetResultProbability(op, espion, case)` rend déjà les six issues avec un contre-espion
nul (`1800d14f0` → `180374000(…, 0, …)`). Le jet d'évasion (`180373ac0`) reçoit lui aussi un
contre-espion nul. Le calcul ci-dessous est donc exact sans contre-espion, et ne révèle rien :

```
P5 = résultat["ESPIONAGE_SUCCESS_UNDETECTED"]      (issue 5)
P4 = résultat["ESPIONAGE_SUCCESS_MUST_ESCAPE"]     (issue 4)
E  = BASE − LEVEL_BOOST × (niveau − 1) − bonus d'évasion
E_police = E − POLICE_CORRECT_MODIFIER          (le paramètre est négatif : −4 attendu)
évadé(e) = Σ table3d6[max(e, 3) .. 18] / 216
p_police = 1 / W
P(évasion) = p_police × évadé(E_police) + (1 − p_police) × évadé(E)
affiché = P5 + P4 × P(évasion)
```

- **Niveau.** C'est `GetExperience():GetLevel()`, soit 1 pour une recrue.
- **Paramètres globaux.** Ils se lisent dans `GameInfo.GlobalParameters`, plutôt que d'être écrits
  en dur : `BASE` = `ESPIONAGE_ESCAPE_BASE_CHANCE`, `LEVEL_BOOST` =
  `ESPIONAGE_ESCAPE_LEVEL_BOOST`, `POLICE_CORRECT_MODIFIER` =
  `ESPIONAGE_ESCAPE_POLICE_CORRECT_MODIFIER`. Le quatrième,
  `ESPIONAGE_ESCAPE_COUNTERSPY_LEVEL_MODIFIER`, ne sert pas : le contre-espion passé est nul.
- **Police.** Le terme `E_police` s'applique quand la police devine la sortie
  (`ArePoliceInPosition`, `180371b10`). Les poids des sorties, `max − TravelTime + 1`, se calculent
  avec la règle exacte relevée à l'étape 2 :
  - `max` est le plus grand `TravelTime` de toutes les lignes de `GameInfo.Districts`, et non
    seulement des quartiers de la ville ;
  - chaque ligne de `GameInfo.Districts` dont le `TravelTime` est positif compte une fois si la
    ville a un quartier terminé de ce type ou un quartier qui le remplace. Un quartier unique qui a
    son propre `TravelTime` compte donc deux fois : pour sa ligne et pour celle qu'il remplace ;
  - un quartier pillé compte, s'il est terminé.
- **Bonus d'évasion** (`m_iEscapeBoost`, `+0x934` de l'espion). Seul l'effet
  `EFFECT_ADJUST_UNIT_ESCAPE_BOOST` l'écrit. On additionne l'`Amount` des modificateurs de cet effet
  (`DynamicModifiers` → `Modifiers`) attachés aux promotions de l'espion, lus dans `GameInfo`
  (aujourd'hui, la seule source connue est `PROMOTION_SPY_ACE_DRIVER`, + 4). C'est une solution Lua
  pure. Elle ne voit pas un modificateur de cet effet qui viendrait d'ailleurs (doctrine, gouverneur,
  mod). Le bloc note dans `Lua.log`, une fois, chaque modificateur de cet effet qui n'est attaché à
  aucune promotion, pour repérer ce cas. L'autre solution, une fonction
  `BGD_BonusEvasion(joueur, unité)` qui lit `+0x934` dans GameCore, demanderait une fonction qui
  donne l'unité à partir de son identifiant. Elle n'est pas dans `MOVED`, et il faudrait la
  ré-identifier sur la 1.0.12.74. Elle n'est à faire que si un tel modificateur apparaît.
- **Arrondi.** `GetResultProbability` tronque chaque issue en virgule fixe sur 8 bits, avec une perte
  de 0,4 % au plus. L'affichage garde l'arrondi actuel du jeu.

Exemple pour une mission à `BaseProbability` 14 (vol de boost, sabotage), sans autre modificateur ni
As du volant :

| niveau | affiché aujourd'hui | nouveau, 4 sorties (W = 10) | nouveau, centre + plateforme (W = 3) | nouveau, centre seul (W = 1) |
|---:|---:|---:|---:|---:|
| 1 | 37,5 | 28,5 | 26,2 | 19,7 |
| 2 | 50,0 | 42,6 | 39,9 | 32,2 |
| 3 | 62,5 | 57,3 | 54,6 | 46,9 |
| 4 | 74,1 | 70,9 | 68,6 | 62,0 |

## Contraintes

- **Interface seule.** Rien ne change dans la simulation : pas de correctif POL, pas de changement de
  `VERSION`, pas de risque de désynchro. Le joueur avec le proxy voit le chiffre corrigé, les autres
  voient celui du jeu.
- **Ne jamais appeler `180373ac0` ni `180373cf0` depuis l'interface.** Ces fonctions appellent
  `GetRand` sur le générateur synchronisé : l'appel ferait avancer le générateur et désynchroniserait
  la partie.
- **Ne jamais détourner `180374000` ni `180372bf0`.** L'IA s'en sert pour évaluer ses missions
  (`18045d280`, `18045ea50`). Les modifier changerait la simulation.
- Si un détour GameCore devenait nécessaire, il ne viserait que le chemin de l'interface
  (`1800d14f0` ou `1806c2740`).

## Étapes

1. **Lire les fichiers Lua d'espionnage du jeu** (Base et XP2), sur une machine qui a le jeu :
   `EspionageSupport.lua`, `EspionagePopup.lua`, `EspionageChooser.lua` et `EspionageEscape.lua`.
   - Recenser chaque appel à `GetResultProbability` et chaque endroit qui affiche le pourcentage, pour
     que tous les affichages changent ensemble.
   - Vérifier si BBG ou MPH remplacent ces fichiers : `ui_journal=on` note les scripts lus.
   - Noter le texte exact auquel le bloc s'accroche, et la façon de récupérer la ville visée depuis
     la case.
2. **Vérifications statiques sur le GameCore 1.0.12.58 : faites le 2026-11-23**, voir
   [Vérifications faites](#vérifications-faites).
3. **Étendre [proba.py](proba.py)** : nouvelles valeurs affichées par niveau, `W` et bonus
   d'évasion. Ce sont les valeurs de référence des tests.
4. **Écrire `proxy/src/ui_espions.lua`**, avec la formule dans une fonction pure, et l'ajouter aux
   scripts trouvés à l'étape 1 depuis [ui.rs](../../proxy/src/ui.rs), comme les autres blocs
   (`ui_logement`, `ui_debordement`).
5. **Ajouter le réglage `ui_espions`.**
   - Le lire dans [lib.rs](../../proxy/src/lib.rs) et l'ajouter à `NOUVEAUTES_UI`.
   - Ajouter sa clé dans l'onglet d'options du proxy de démarrage (test
     `cles_communes_avec_le_proxy`), avec `nouveaute_ui = true`.
   - Ajouter le libellé et l'infobulle dans les 11 langues de [textes](../../textes/README.md).
     L'infobulle précise que le chiffre suppose la meilleure sortie et ne compte pas un éventuel
     contre-espion.
   - Ajouter `-UiEspions` à `install.ps1`.
6. **Écrire le test hors jeu `proxy/tests/test_ui_espions.lua`.** Il compare la formule aux valeurs
   de `proba.py` (niveaux 1 à 4, `W` de 1 à 10, avec et sans As du volant). Si les fichiers du jeu
   sont présents, il vérifie aussi l'accroche dans le vrai script, et il est sauté sans eux.
7. **Mettre à jour la documentation** : [README.md](README.md) de l'analyse,
   [proxy/README.md](../../proxy/README.md), [SOMMAIRE.md](../../SOMMAIRE.md), `CHANGELOG.md` et ses
   traductions.
8. **Vérifier en partie.** Sans contre-espion, relever au tirage « Rolling Espionage Result »
   (`180373cf0`) les six poids, puis le seuil E et la sortie devinée à l'évasion, et les comparer au
   chiffre affiché. Il faut faire la même chose avec un espion As du volant, et dans une ville
   réduite à son centre.

## Vérifications faites

Le 2026-11-23, sur `binaires_jeu/1.0.12.58/GameCore_XP2_FinalRelease.dll`, désassemblé par objdump.
La logique a d'abord été lue dans le build de 2020 du projet Ghidra (`binaires/`, horodatage
`5ee10d21`), puis confirmée instruction par instruction sur la .58. Les adresses de
[README.md](README.md) sont celles de la .58, qui sont aussi celles du build du 2026-11-02 dont
partent les tables de [patch.rs](../../proxy/src/patch.rs). Le build de 2020 a d'autres adresses et
d'autres décalages de structures (`+0x904` au lieu de `+0x934`, par exemple) : ne jamais en
reprendre.

- **Seuil d'évasion** (`180372b40`, appelé par `180373c30`, qui construit les poids de
  `180373ac0`) :
  `E = [P+0x2c0] − ([P+0x2c8] × (niveau − 1) + espion+0x934 + [contre-espion] + [police ? P+0x2cc])`,
  où `P` = `DAT_180b8aa70`. Les paramètres sont rangés par ordre alphabétique :
  `+0x2c0` `ESPIONAGE_ESCAPE_BASE_CHANCE`, `+0x2c4` `ESPIONAGE_ESCAPE_COUNTERSPY_LEVEL_MODIFIER`,
  `+0x2c8` `ESPIONAGE_ESCAPE_LEVEL_BOOST`, `+0x2cc` `ESPIONAGE_ESCAPE_POLICE_CORRECT_MODIFIER`. Les
  tranches sont : évadé `[E, 18]`, capturé `[E−2, E−1]`, tué `[3, E−3]`.
- **Sorties de la police** (`180371b10`) :
  - `max` = le plus grand `TravelTime` (`+0x4c` de la définition) de toutes les définitions de
    quartier ;
  - pour chaque définition dont le `TravelTime` est positif, `18011ee20(quartiers de la ville,
    type, 1, 1)` teste si la ville a ce quartier. Le premier drapeau exige `+0x3f8` du quartier,
    qui est `m_bIsComplete` (lu dans la liste des champs sérialisés). Le second accepte un quartier
    qui remplace ce type. Le pillage n'est pas testé ;
  - le poids d'une sortie est `max − TravelTime + 1`. La police tire sa sortie par « Police Exit
    Covered » et devine si elle tombe sur la sortie de l'espion, `+0x948`.
- **Choix de la sortie.** Il se fait après le tirage de la mission, par la notification
  `SPY_CHOOSE_ESCAPE_ROUTE`, qui bloque la fin de tour, et la commande `SET_ESCAPE_ROUTE`. Au moment
  du choix de la mission, la sortie n'est donc pas connue : l'hypothèse de la meilleure sortie tient.
- **Bonus d'évasion** `+0x934` (`m_iEscapeBoost` dans la chaîne de débogage de l'espion). Il n'est
  écrit que par l'effet `EFFECT_ADJUST_UNIT_ESCAPE_BOOST`.
- **Capture changée en mort** : la condition non identifiée de [README.md](README.md) ne change
  que l'issue d'un échec d'évasion. Elle ne touche pas la chance d'évasion.

## Ce qu'il faut re-identifier sur la 1.0.12.74

La 1.0.12.74 n'est pas disponible localement. [patch.rs](../../proxy/src/patch.rs) la décrit comme le
même code et les mêmes données que la 1.0.12.58, à d'autres adresses.

- **Avec la solution Lua seule (retenue) : aucune adresse.** Il faut seulement confirmer que les
  règles n'ont pas changé, en comparant les fonctions suivantes à la .58, au déplacement près :
  - `180375100` (évasion), `180373ac0`, `180373c30` (tranches évadé, capturé, tué) et `180372b40`
    (seuil E, décalages `+0x2c0` à `+0x2cc` des paramètres et `+0x934` de l'espion) ;
  - `180371b10` (choix de la sortie par la police, poids, `+0x948`) et `18011ee20` (quartier
    terminé `+0x3f8`, remplacement accepté) ;
  - `180374180` (tranches des six issues), `180374000`, puis `1800d14f0` et `1806c2740` (l'interface
    passe toujours un contre-espion nul) ;
  - la table 3d6 `180b64c70`.
  Il faut aussi relire les fichiers Lua d'espionnage installés avec la .74, si le bloc s'accroche à
  leur texte.
- **Avec la lecture de `+0x934` dans GameCore (seulement si l'étape 2 l'impose), en plus :**
  - l'adresse .74 de la fonction qui donne l'unité, à ajouter dans `MOVED` ;
  - les octets `expected` et les `FIXUPS` de son prologue, si elle est appelée ou recopiée ;
  - le décalage `+0x934` lui-même, confirmé dans le code .74 qui l'écrit.
