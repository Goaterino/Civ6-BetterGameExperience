# Minimap et caméra bornées aux cases révélées

Symptôme : en début de partie, la minimap ne montre que la zone explorée et la caméra ne va pas
au-delà. Les deux s'agrandissent au fil de l'exploration. Pour s'en débarrasser, les joueurs posent
des épingles tout en haut et tout en bas de la carte : la caméra est débloquée et la minimap prend sa
taille finale.

**Corrigé dans le proxy** (réglage `ui_carte_entiere=on|off`, on par défaut,
[proxy/src/carte.rs](../proxy/src/carte.rs)) : la minimap et la caméra couvrent la carte entière dès
le premier tour. C'est purement local : rien ne change dans GameCore ni sur le réseau, un joueur peut
l'avoir sans les autres.

Analyse statique du 2026-11-08 sur `CivilizationVI_DX12.exe`, projet Ghidra
`~/Tools/ghidra-projects/civ6exe` en `-noanalysis -readOnly`. **Ce projet est d'un build antérieur à
l'exe installé** : les adresses ci-dessous sont celles du projet, sauf mention. Le site du correctif
est retrouvé par signature, vérifiée dans les deux exe installés (DX12 et DX11).

## Le Lua n'y est pour rien

- `MinimapPanel.lua` ne calcule aucune borne. Il donne son contrôle image à l'exe
  (`UI.SetMinimapImageControl`), redimensionne son cadre quand l'image change de taille
  (`OnMinimapImageSizeChanged` → `ResizeBacking`) et traduit les clics avec
  `UI.GetMinimapWorldRect()` (comme `FullscreenMapPopup.lua`).
- `UI.SetMinimapSize` (Options.lua) règle la taille **à l'écran** de la minimap (option 0..1), pas
  la zone du monde couverte.
- `UI.GetLockedWorldRect` (les bornes de la caméra) n'est lu par aucun Lua du jeu, et il n'existe
  aucune fonction Lua pour fixer ces bornes.

## Le calcul natif

- `UI.GetLockedWorldRect` (`1401a6e60`) rend 4 flottants en `vue + 0xca4` ; la vue de jeu
  (GameViewState) est `[0x1412fa2c8] + 0x14f8`.
- `UI.GetMinimapWorldRect` (`1401a7020` → `1401f1af0`) rend `142310aa8`, que le rendu de la minimap
  (`Minimap_Render` `1401f1bb0`, `Minimap_RenderFullscreen` `1401f2390`, via `1401f1410` et
  `1401f0ba0`) tire du rectangle `vue + 0x908`, mis au rapport largeur/hauteur de l'image.
- **Les deux viennent de `14007c1b0`, recalculée à chaque image** par
  `GameViewState_PreAsyncSetup` (`14007e870`) : le résultat va en `vue + 0xca4` (caméra), puis
  `140080e50` le recopie en `vue + 0x908` (minimap). L'initialisation de la vue (`14007f840`) fait
  de même. Pour la caméra, la largeur est ensuite plafonnée à `vue+0xb08 × vue+0xa54` (bouclage).

### `14007c1b0`

1. **Épingles.** Pour le joueur local (`1400f1810`), elle parcourt les épingles des 22 premières
   configurations de joueur : le vecteur d'entrées de 32 octets en `config + 0x20 / 0x28`, celui que
   reconstruit `1401162b0` (voir [map_pins](../map_pins/README.md)). Elle garde celles que le joueur
   voit (`140114f50`) : les siennes, et celles des autres partagées à tous (visibilité -2) ou à son
   équipe (-3). Chacune donne sa colonne et sa ligne.
2. **Cases révélées.** La carte de brouillard du rendu (`1412fb5a8` : largeur et hauteur ;
   `1412fb5b0` : un octet par case, bits `& 3`) marque chaque colonne qui a une case révélée **ou une
   épingle**. Sans bouclage (`vue + 0xfe0 == 0`) : de la première à la dernière colonne marquée.
   Avec bouclage : tout sauf le plus grand trou de colonnes non marquées. Les lignes vont du Y
   minimal au Y maximal des cases révélées et des épingles.
3. Une case de marge, puis la conversion en coordonnées du monde (matrice en `vue + 0x240..0x274`) et
   l'agrandissement aux coins des hexagones.
4. **Carte entière** (l'AABB de la carte, `vue + 0x2c0` et `vue + 0x2cc`) seulement si rien n'est
   révélé, ou si `(largeur de la carte − largeur calculée) × 1/64 < 10.0` : le test ne regarde que la
   largeur.

**Pourquoi les épingles débloquent tout :** une épingle compte comme une case révélée. Une en haut
et une en bas étirent le rectangle sur toute la hauteur. Effet secondaire : une épingle partagée par
un autre joueur agrandit aussi votre minimap.

## Le correctif

La fin de l'étape 4, dans l'exe :

```
F3 41 0F 10 8E CC 02 00 00   movss  xmm1,[r14+2CC]
F3 41 0F 5C 8E C0 02 00 00   subss  xmm1,[r14+2C0]
F3 0F 10 45 20               movss  xmm0,[rbp+20]
F3 0F 5C 45 18               subss  xmm0,[rbp+18]
F3 0F 5C C8                  subss  xmm1,xmm0
F3 0F 59 0D ?? ?? ?? ??      mulss  xmm1,[1/64]
0F 2F 0D ?? ?? ?? ??         comiss xmm1,[10.0]
73 69                        jnc    -> garde le rectangle calculé    (signature + 0x2F)
EB 0E                        jmp    -> rectangle = carte entière
```

- Les 47 premiers octets (avec jokers) sont uniques dans les exe installés : `jnc` en `14007c4e2`
  (DX12) et `14007b992` (DX11) ; `14007c852` dans le projet Ghidra.
- Le proxy remplace `73 69` par `90 90` : le `jmp` qui suit mène toujours à la carte entière. La
  caméra (`0xca4`), la minimap, la carte plein écran et la vignette des sauvegardes suivent.
- Garde-fous : signature trouvée une seule fois, `EB 0E` attendu après, `73 69` ou `90 90` (proxy
  rechargé) au site ; sinon rien n'est écrit et le journal le dit (`carte : …`). Les octets d'origine
  sont remis au déchargement du proxy. Le réglage est lu au chargement du proxy de GameCore : un
  changement vaut pour la partie suivante (ou après redémarrage si une partie a déjà été lancée).
- Le proxy de GameCore n'étant chargé que pour Gathering Storm, le correctif ne vaut que là.

## Multijoueur

Aucun effet : l'exe dessine et borne la caméra sur la machine locale, GameCore et la configuration
des joueurs ne sont pas touchés, rien n'est diffusé. Le brouillard reste dessiné comme avant : on
voit ce que les épingles aux bords montraient déjà. En prime, moins d'épingles posées, c'est moins de
`BroadcastPlayerInfo` et de reconstructions chez les autres (voir [map_pins](../map_pins/README.md)).

## Écarté : des épingles automatiques en Lua

Le seul levier Lua serait de poser des épingles pour le joueur local aux bords de la carte. Elles
partiraient dans sa configuration à chaque diffusion (le lag de map_pins), s'afficheraient dans la
liste et sur la carte, resteraient dans la sauvegarde, et ne suffiraient pas en largeur sur une carte
qui boucle.

## À vérifier en jeu

- `RolloverProxy.log` : `carte : minimap et caméra sur la carte entière (exe+0x7c4e2)` en DX12.
- Premier tour : minimap à sa taille finale, caméra libre jusqu'aux bords, brouillard intact.
- Carte qui boucle (Continents…) et carte sans bouclage ; vue stratégique ; carte plein écran.
- Spectateur (joueur local -1).
- `ui_carte_entiere=off` : comportement d'origine.
