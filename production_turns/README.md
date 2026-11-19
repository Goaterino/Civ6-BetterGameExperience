# Compteur de tours de production : bonus plats de gouvernement oubliés

Gêne observée en partie : avec le fascisme, une unité affichée à 6 tours passe à 4 tours restants
après le passage de tour. Le +50 % de production d'unités (`GOVERNMENTBONUS_UNIT_PRODUCTION`) est bien
appliqué à la progression, mais pas au calcul du nombre de tours affiché. Le proxy corrige
l'affichage depuis le 30 octobre 2026 (voir plus bas).

## Côté Lua (2026-10-30)

L'interface ne calcule rien. `ProductionPanel.lua`, `CitySupport.lua` et `CityBannerManager.lua` (jeu,
Gathering Storm), ainsi que `productionpanel_bbg.lua` (BBG, Workshop 2865001760), appellent seulement
`buildQueue:GetTurnsLeft(hash[, formation])`. Corriger le natif suffit donc, quels que soient les mods
d'interface chargés.

## Côté GameCore : le calcul affiché

Projet Ghidra `civ6xp2` (`GameCore_XP2_FinalRelease.dll` d'origine, SHA-256 `324c51e9…c98c`, base
0x180000000). La copie installée sous ce nom est le proxy : il faut analyser
`GameCore_XP2_FinalRelease_orig.dll`. Les valeurs sont en virgule fixe 24.8 : `>> 8` divise par 256.

| adresse | rôle |
|---|---|
| `18068ca80` | binding Lua `GetTurnsLeft` du cache (`Cache_Lua_ICityBuildQueue`). Selon le nombre d'arguments, appelle `18006c880` (élément courant), `18006c650` (type) ou `18006c750` (type + formation) |
| `18006c880`, `18006c650`, `18006c750` | `Cache_City_BuildQueue`. Si le verrou GameCore est pris (virtuelles `+0x48` / `+0x38` de `DAT_180b8aa80`), ils appellent la file réelle de la ville (`city+0x10d0`). Sinon, ils renvoient la valeur en cache (`cache+0x28`), et seulement pour l'élément courant ; pour les autres, 0 |
| `1800fe300` | `City_BuildQueue::GetTurnsLeft(hash, formation)` : trouve le type (bâtiment, unité, district, projet) et appelle le prédicteur de ce type |
| `1800fe9b0` | même chose pour l'élément en tête de file (`queue+0x1d0`, type en `+0x30`) |
| `1800fe8b0` | **prédicteur des unités** : `taux = rendement × (mod_unité[queue+0x2c0] + bonus joueur par unité) >> 8` ; le coût vient de `1800feb80` (formation comprise) |
| `1800fe420` | prédicteur des bâtiments et merveilles (multiplicateurs `+0x308`, `+0x30c`, `+0x318`, `1800fc770`, `1800ff8e0`) |
| `1800fe610` | prédicteur des districts (`+0x310`, `1800ffae0`, `1800fc870`, `+0x2d8`) |
| `1800fe780` | prédicteur des projets |
| `180605dd0` | `(coût, &progression, &taux)` : renvoie `ceil((coût×256 − progression) / taux)`, 1 si c'est déjà payé, −1 si le taux est nul. La progression comprend le débordement (`queue+500`) |
| `1801334c0` | rendement de production de la ville pour une catégorie (hash en 4ᵉ argument : `0x15d11e18` pour les unités) |

## Côté GameCore : la progression réelle

| adresse | rôle |
|---|---|
| `1800f5080` | traitement de la production du tour (journal « %d, %s, %3.1f, … ») : appelle l'ajout de progression du type, puis la fin de production (`1800fb520` pour une unité, `ORDER_TRAIN`) |
| `180105850` | ajout de progression d'une **unité** |
| `1801049b0` | ajout de progression d'un bâtiment ou d'une merveille |
| `180104f40` | ajout de progression d'un district |
| `180105270` | ajout de progression d'un projet |
| `1802610b0` | `Player_Bonuses` du joueur |
| `1802610f0` | `GetFlatBonus(bonuses, hash)` : lit le tableau `+0xb8`. Renvoie 0 sauf si `(joueur+0xd8)->+0x6f0->+0x108 < 0` (une condition d'état du gouvernement, à identifier) |
| `180261010` | `AddFlatBonus` : ajoute au tableau `+0xa8`. Appelé par l'effet `EFFECT_ADJUST_FLAT_BONUS` (`18089ba40`) |

**L'écart.** `180105850` calcule le même taux que `1800fe8b0`, puis le multiplie par :

```
facteur = 1.0 + 0.01 × (GetFlatBonus(0x3d5ff1a2) + GetFlatBonus(0xc1a005d2))
```

Le facteur passe en virgule fixe (×256) avant la multiplication. Les constantes sont lues dans le
binaire : `0.01` en `1809c8978`, `1.0` en `1809bd870`, `256.0` en `1809c1b78`. Aucun des quatre
prédicteurs n'appelle `1802610f0`.

## Tous les bonus plats sont concernés

Les quatre fonctions de progression lisent des bonus plats. Les hachages ne figurent pas dans la table
`Types` de `DebugGameplay.sqlite`, et ce n'est ni FNV-1, ni FNV-1a, ni CRC32 : les noms sont déduits
de quel type de production lit lequel. Les montants viennent de BBG 7.5.1.

| hash | lu par | nom probable | qui le donne |
|---|---|---|---|
| `0xc1a005d2` | les quatre types | `GOVERNMENTBONUS_OVERALL_PRODUCTION` | personne dans BBG |
| `0x3d5ff1a2` | unités | `GOVERNMENTBONUS_UNIT_PRODUCTION` | fascisme, +50 |
| `0x5953bf98` | merveilles seulement (`1801049b0`, branche « merveille ») | `GOVERNMENTBONUS_WONDER_CONSTRUCTION` | autocratie, +10 |
| `0x112d393` | districts | `GOVERNMENTBONUS_DISTRICT_PRODUCTION` | république marchande, +15 |
| `0xbcfe99b8` | projets (`180105270`), dans une boucle à part | `GOVERNMENTBONUS_DISTRICT_PROJECTS` ? | à identifier |

Les merveilles sous autocratie et les districts sous république marchande devraient donc afficher, eux
aussi, un nombre de tours trop élevé. L'écart est plus faible et passe plus facilement inaperçu.

**L'exemple du fascisme colle.** Prenons coût / taux affiché = 5,5. Le compteur affiche
`ceil(5,5) = 6`. Le jeu ajoute 1,5 taux : il en reste 4,0, et le compteur affiche 4 (5 sans bonus).

## Correctif dans le proxy (2026-10-30)

[proxy/src/tours.rs](../proxy/src/tours.rs), réglage `tours_production=on|off` (on par défaut).

**Pourquoi on ne détourne pas les prédicteurs.** Ils servent aussi à l'IA : `1800fe8b0` est appelé
par `1803e2aa0`, `1803fa5e0`, `1803fa8e0` et `180464600`, et `1800fe9b0` par `180552cc0`. Chaque
joueur recalcule l'IA de son côté. Modifier ces fonctions changerait ses choix chez qui a le proxy,
et le désynchroniserait des autres.

**Ce que lit l'interface.** Le binding Lua `GetTurnsLeft` passe par `Cache_City_BuildQueue` :
- sous verrou, `18006c880` appelle `1800fe9b0` (en `18006c8d3`), et `18006c650` / `18006c750`
  appellent `1800fe300` (en `18006c6ad` / `18006c7b5`) ;
- sans verrou, ils rendent `cache+0x28`. Cette valeur est remplie par `18006b860` (construction du
  cache), qui appelle `1800fe9b0` en `18006b8dc`.

**Le correctif.** Le proxy redirige ces quatre `call rel32`, et eux seuls, vers des relais posés à
moins de 2 Go de GameCore. Les relais mènent à une réécriture des deux répartiteurs. Celle-ci appelle
les prédicteurs d'origine en passant l'argument qu'ils offrent déjà (à 0 chez tous les appelants).
Sa valeur est choisie pour que le taux prédit contienne le facteur `F` :

| type | argument passé | effet sur le taux |
|---|---|---|
| unité | bonus additif `m·F >> 8 − m`, avec `m` = `queue+0x2c0` + bonus du joueur | `(m·F >> 8) · rendement >> 8`, comme `180105850` |
| bâtiment | bonus additif sur `queue+0x2a8`, `F` augmenté du bonus des merveilles pour une merveille | même facteur que `1801049b0` |
| district | multiplicateur final `F` | `F` appliqué en fin de chaîne |
| projet | `F − 256` (le prédicteur multiplie par `arg + 256`) | `F` appliqué en fin de chaîne |

`F` est converti du flottant en virgule fixe comme le fait le jeu : partie entière `<< 8`, fraction
`× 256` tronquée à 8 bits. Sans bonus, `F` = 256 et le résultat est celui de l'original.

**Ce n'est exact qu'aux arrondis près.** Pour les unités, le calcul est le même que celui de la
progression. Pour les bâtiments et les districts, les prédicteurs d'origine multiplient déjà dans un
autre ordre que la progression. Leurs troncatures `>> 8` diffèrent donc un peu, avec ou sans
correctif.

**DX11 et DX12.** Les deux exe chargent le même `GameCore_XP2_FinalRelease.dll` : le correctif ne
dépend pas de l'exe.

**Compatibilité 1.0.12.74.** Les quatre appels du cache et les prédicteurs de la file restent aux
mêmes adresses, mais leurs dépendances ont changé. Par exemple, le gestionnaire des joueurs passe
de `180306b80` à `180305dc0`, et la recherche d'un bâtiment de `1802209a0` à `1802210a0`.
Vérifier seulement les quatre appels du cache laissait donc installer un correctif dont les
adresses internes étaient périmées. `tours.rs` vérifie maintenant quinze appels natifs des chemins
de prévision et de progression, puis choisit les adresses de la version .72 ou .74. Les quinze
doivent appartenir à la même version. Sinon, le compteur du jeu est conservé.

**Le chemin `18058e4e0`.** Il appelle `1800fe300` directement quand il a la vraie file (sinon il
passe par le cache, qui est corrigé). Il reste non corrigé : on n'a pas établi s'il sert à
l'interface ou aux scripts de règles.

## Reste à vérifier

- La condition `+0x108 < 0` de `1802610f0` : anarchie, changement de gouvernement en cours ? (Le
  correctif appelle `1802610f0`, il suit donc cette condition comme la progression.)
- Les noms des hachages : à confirmer en lisant le chargement de `GovernmentBonusNames` (référence
  `?180bba490` à `1802610f0`).
- En jeu : unité sous fascisme, merveille sous autocratie, district sous république marchande, avec et
  sans débordement de production.

Sorties Ghidra de cette analyse : `DumpAddrs.java` et `CallersOf.java` (`~/tools/ghidra-scripts`),
lancés en headless sur `civ6xp2` en lecture seule. Les appelants, les sites d'appel et les prologues du
correctif ont été relevés avec REA (Ghidra 12.1.4) sur l'original, SHA-256 `324c51e9…c98c`.
