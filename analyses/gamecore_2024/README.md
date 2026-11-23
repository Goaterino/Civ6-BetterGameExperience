# GameCore du 2024-06-26 refusé avec l'exe 1.0.12.58

**Corrigé en local** (2026-11-23), pas encore publié ni essayé chez le joueur : voir
[Correctif](#correctif).

Signalement d'un joueur après le passage à la 0.14.0 : BGE est installé et activé, mais en partie
rien ne fonctionne, et les signaux du départ simultané (`.bgd_depart 1`, `.bgd_depart 2`…)
s'affichent dans le chat. Journaux reçus : `RolloverProxy.log` (de la 0.6.0 à la 0.14.0) et le
dossier `Logs` du jeu (dernière session seulement).

## En bref

- **Cause.** La 0.14.0 (commit `77c432d`, support de 1.0.12.58) a rendu strict le contrôle du build
  dans `patch::detect` ([proxy/src/patch.rs](../../proxy/src/patch.rs)) : `BUILDS` n'accepte qu'un
  GameCore par version d'exe. Pour 1.0.12.58, c'est celui du 2023-11-11 (`0x654ecd4e`). Ce joueur a
  un exe 1.0.12.58 DX11 (reconnu par les empreintes des modules réseau) et un GameCore lié le
  **2024-06-26** (`0x667c6f5b`). Le proxy de GameCore ne pose donc plus rien :

  ```
  GameCore 0x667c6f5b n'est pas celui du jeu 1.0.12.58 (0x654ecd4e) : aucun correctif posé, GameCore d'origine
  ```

- **Régression, pas une installation ratée.** Jusqu'à la 0.13.0, tout GameCore autre que 1.0.12.74
  était pris comme le build du 2026-11-02. Avec ce GameCore, les détours se posaient (octets
  attendus de chaque site vérifiés) et le proxy marchait depuis la 0.6.0. Dernière session avec
  « détours posés » : 0.13.0, 2026-11-20 23:32. Premier refus : 0.14.0, 2026-11-21 19:28.
- **Ce build de 2024 est celui de départ du projet** : SHA-256 `324c51e9…c98c`, cité par
  [docs/gamecore_xp2](../../docs/gamecore_xp2/README.md), `production_turns`, `citoyens`,
  `wonder_queue` et `unites_superposees`. C'était le GameCore de Steam jusqu'à la mise à jour du
  2026-11-05.

## Symptômes expliqués

- **Ce qui marche encore** vient du proxy de démarrage (`winmm.dll`) : onglet d'options, options
  de partie BGE, annonce dans le salon, BGE Check, bouton « Signaler un bug ».
- **Ce qui manque** vient du proxy de GameCore : correctifs POL, mods d'interface repris, côté
  client du départ simultané, et le filtre du chat (`proxy/src/ui_depart_chat.lua`, ajouté à
  `ChatPanel.lua` par le proxy de GameCore). Sans ce filtre, les signaux `.bgd_` que l'hôte envoie
  s'affichent en clair.

## Second défaut : correctifs POL annoncés à tort

À la première partie après chaque lancement, le proxy de GameCore n'est pas encore chargé.
`pol_indisponible` ([startup_proxy/src/salon.rs](../../startup_proxy/src/salon.rs)) prédit alors
la disponibilité des correctifs POL à partir du fichier de réglages et de la version du fichier
seulement, sans regarder le build de GameCore. Dans le journal :

- la machine s'annonce et l'hôte rend « All players have BetterGameExperience 0.14.0: POL fixes
  enabled » (1795448258–263) ;
- BGE Check répond « 0.14.0 (actif) » (1795448585), et la bascule FMA3 de POL est activée ;
- puis le proxy de GameCore refuse le build et ne pose aucun correctif.

Cette machine joue alors sans les correctifs POL, alors que les autres les ont. C'est un risque de
désynchro pour toute la partie, pas seulement pour ce joueur. Ce n'est qu'à la réouverture du salon
que le bit `ST_POL` est lu et que l'annonce s'arrête. Aucune désynchro n'apparaît dans les journaux
réseau reçus, mais ils ne couvrent que la dernière session.

## Comparaison avec le GameCore 1.0.12.58 (2026-11-23)

Le fichier du joueur est rangé dans `binaires_jeu/1.0.12.58_gamecore_2024-06-26/` (ignoré par
git). SHA-256 `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`, date d'édition de
liens `0x667c6f5b` : c'est bien le build de départ du projet.

- `.text`, `.data`, `.pdata`, `.tls`, `.rsrc` et `.reloc` sont **identiques octet pour octet** à
  ceux du GameCore 1.0.12.58 du 2023-11-11 (mêmes adresses, mêmes tailles).
- Les seules différences : la date d'édition de liens (en-tête et répertoire de débogage), la somme
  de contrôle et le répertoire de sécurité de l'en-tête, l'empreinte (GUID) du PDB, un horodatage de
  `.rdata`, et une signature Authenticode ajoutée en fin de fichier (14 944 octets).
- `proxy/tests/check_hooks.py` passe dessus (« jeu 1.0.12.58 (GameCore 0x667c6f5b) », OK), avec
  les mêmes détours posés que sur le GameCore du 2023-11-11.

## Correctif

Fait en local le 2026-11-23 :

1. **`BUILDS` accepte plusieurs GameCore par version d'exe** ([patch.rs](../../proxy/src/patch.rs)) :
   ligne `("1.0.12.58", 0x667c_6f5b, false)`, et `known_build` cherche la ligne qui correspond à
   la fois à la version et à la date d'édition de liens. Test `builds_of_a_game_version`.
2. **La prédiction du proxy de démarrage lit la date d'édition de liens du GameCore sur le disque**
   (`gamecore_refuse` dans [salon.rs](../../startup_proxy/src/salon.rs)) et la passe au même
   `patch::known_build` (`patch.rs` est commun aux deux proxys). Une machine dont le GameCore ne
   sera pas reconnu ne s'annonce plus avec les correctifs POL. Test `gamecores_connus`.

Reste : publier, puis vérifier dans le journal du joueur « jeu 1.0.12.58 : tables d'adresses
connues » et les détours posés.

## Ressources (reçues le 2026-11-23)

- **Le GameCore de ce joueur** : `DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll`,
  lié le 2024-06-26 (`0x667c6f5b`). Il n'est **pas** dans le dépôt ni sur la machine de
  développement : le GameCore 1.0.12.58 de `binaires_jeu/1.0.12.58` (et le `.rar` reçu avec ce
  signalement) est celui du 2023-11-11 (`0x654ecd4e`, SHA-256 `8e699090…`).
- Avec ce fichier : vérifier que son SHA-256 est bien `324c51e9…c98c`, passer
  `proxy/tests/check_hooks.py` dessus avec `BGE_VERSION_JEU=1.0.12.58`, et comparer `.text` et
  `.data` à ceux du build du 2026-11-02. Les sessions de la 0.13.0 sont un indice fort de
  compatibilité, mais les adresses de données n'y étaient pas contrôlées octet par octet.
- Les exe 1.0.12.58 de `binaires_jeu/1.0.12.58` suffisent pour les modules réseau : ils ont été
  reconnus chez ce joueur (« DX11 1.0.12.58 »).
