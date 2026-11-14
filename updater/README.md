# Installeur : `BetterGameExperience_V<version>.exe`

Installe, met à jour, répare ou retire les deux proxys et les médias des signaux d'équipe à partir de la dernière release publiée sur
GitHub (voir [release/README.md](../release/README.md)). Il est publié avec chaque release, sous
le nom `BetterGameExperience_V<version>.exe` (`BetterGameExperience_V0.6.0.exe`) ; compilé, et à son
emplacement fixe, il s'appelle `BetterGameExperience.exe`.

## Utilisation

- **Double-clic** : une fenêtre (TaskDialog de Windows) affiche le dossier du jeu, la dernière
  version et l'état de l'installation (pas installé, à jour, mise à jour disponible, à réparer), le
  détail par DLL sous « Détails », et propose installer, mettre à jour ou réparer, et désinstaller.
  Après une installation, elle dit que tout est en place et qu'on peut la fermer et lancer le jeu.
  Chaque fenêtre à choix propose aussi la langue (liens en bas de la fenêtre, voir Langues).
- **Pré-release** : la case à cocher de la fenêtre principale choisit le canal. Cochée, l'installeur
  vise la release publiée au plus grand numéro, pré-releases comprises (liste de l'API GitHub) ;
  décochée, la dernière release (`releases/latest`, qui ne désigne jamais une pré-release). Décochée
  alors qu'une pré-release est posée, la fenêtre propose « Revenir à la version … », la dernière
  release. Le choix est gardé dans `%LOCALAPPDATA%\BetterGameExperience\canal.txt` : l'option de
  lancement Steam et `--after-game` le suivent (en canal release, une pré-release posée y est
  remplacée par la dernière release), tout comme la notification du jeu
  ([startup_proxy](../startup_proxy/README.md#la-notification-de-mise-à-jour)), qui dit si elle
  propose une release ou une pré-release. La version posée est gardée dans `installe.txt`, à côté.
- **`--game-dir "<dossier>"`** : même fenêtre, pour un jeu hors des bibliothèques Steam.
- **Options de lancement Steam** (Propriétés > Général), que l'installeur ne propose plus depuis la
  release qui suit la v0.5.0 (les joueurs croyaient l'installation inachevée ; la notification du
  jeu suffit aux mises à jour), mais qui reste reconnue pour ceux qui l'ont posée :
  ```
  "%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe" %command%
  ```
  À chaque lancement, l'installeur vérifie la dernière release (environ 1 s, sans fenêtre si tout
  est à jour), installe si besoin derrière une fenêtre de progression, puis lance le jeu et attend
  sa fin, pour que Steam le voie en cours. Depuis la release qui suit la v0.3.0, Steam ne retire plus les proxys :
  cette option ne sert plus qu'aux mises à jour de BetterGameExperience. Un échec (GitHub injoignable, droits refusés) est affiché,
  et le jeu se lance quand même.
- **`--after-game [<dossier>]`** : lancé par la notification de mise à jour du jeu
  ([startup_proxy](../startup_proxy/README.md#la-notification-de-mise-à-jour)), qui se ferme
  aussitôt. Attend la fin du jeu (une minute au plus), installe la dernière release, puis propose
  de relancer Civilization VI par Steam (`steam://rungameid/289070`). Le dossier, celui de l'exe du
  jeu, sert à retrouver le jeu même hors des bibliothèques Steam.

## Fonctionnement

- **Jeu** : le dossier qui contient `DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll`,
  cherché parmi les parents de l'exe lancé par Steam, sinon dans les bibliothèques de
  `steamapps/libraryfolders.vdf` (dossier de Steam lu dans `HKCU\Software\Valve\Steam`).
- **Téléchargement** : par WinHTTP, la pile HTTP de Windows ([src/http.rs](src/http.rs), proxy du
  système, deux nouveaux essais sur une erreur réseau ou serveur passagère), dans
  `%TEMP%\BetterGameExperience`. Le manifeste doit
  porter une signature valide pour la clé publique embarquée, et chaque DLL le SHA-256 du manifeste.
- **Pose**, comme les `install.ps1` : à côté des fichiers du jeu, sans en remplacer aucun
  ([proxy/src/noms.rs](../proxy/src/noms.rs)) : `winmm.dll` dans `Base/Binaries/Win64Steam`,
  `BGD_GameCore_XP2.dll` dans `DLC/Expansion2/Binaries/Win64`. Une vérification des fichiers ou une
  mise à jour du jeu par Steam ne les retire pas. Un proxy se reconnaît à une chaîne qu'il contient
  (`StartupProxyStatus`, `RolloverProxyStatus`). Une `winmm.dll` qui n'est pas la nôtre (un autre
  mod) n'est pas écrasée : l'installation s'arrête et le dit. `rollover_proxy.cfg` n'est jamais
  touché.
- **Anciennes installations** (jusqu'à la v0.3.0, chaque proxy à la place d'un original,
  renommé `<nom>_orig.dll`) : l'installation et la désinstallation remettent l'original en place.
  Un `_orig` à côté d'un original remis par Steam est périmé : il est retiré. La fenêtre les montre
  (« ancien proxy à la place de l'original ») et propose la mise à jour.
- **Compatibilité** : un installeur de la v0.3.0 ou d'avant refuse les releases suivantes (fichiers
  inconnus) et demande de télécharger le nouvel installeur depuis la page de la release.
- **Droits** : si le dossier du jeu n'est pas modifiable (Program Files), seule la pose est relancée
  en administrateur (`--apply`, invite UAC, sans fenêtre). Ce processus vérifie d'abord que le
  dossier reçu est bien celui du jeu (sinon il s'arrête sans rien écrire), puis revérifie la
  signature du manifeste et le SHA-256. Chaque DLL est lue une seule fois dans
  `%TEMP%\BetterGameExperience`, contrôlée en mémoire, et ce sont ces octets-là qui sont écrits :
  un autre processus de l'utilisateur ne peut pas la remplacer entre le contrôle et la copie. La
  pose sans administrateur fait de même.
- **Emplacement fixe** : à l'ouverture de sa fenêtre et après chaque installation, l'installeur se copie dans
  `%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe`, s'il n'y tourne pas déjà. La notification du
  jeu le lance là, et l'option de lancement Steam le désigne : peu importe où le joueur a
  téléchargé l'exe, et la copie suit le dernier installeur lancé. Elle garde ce nom fixe, sans
  version : l'option de lancement d'un joueur la désigne encore.
- **Anciens installeurs** : à l'ouverture de sa fenêtre, un installeur de release retire, dans son
  propre dossier (d'habitude les téléchargements), ceux téléchargés avant lui :
  `BetterGameExperience.exe` et `BetterGameDLLs.exe` (noms d'avant la version dans le nom), et les
  `BetterGameExperience_V<version>.exe` de même version ou plus anciens, doublons « (1) » du
  navigateur compris. Jamais un plus récent, ni rien à l'emplacement fixe ; une compilation locale
  (sans `BGD_RELEASE`) ne retire rien.
- **Pas de console, pas de processus caché** : l'exe est une application fenêtrée, et ne lance
  aucun programme sans fenêtre. Les téléchargements passent par WinHTTP, la détection du jeu en
  cours par la liste des processus (Toolhelp), la relance du jeu par le shell (`steam://`). Un
  `curl.exe` ou un `tasklist` lancé en cachette est l'un des comportements que les antivirus
  prêtent aux logiciels malveillants.
- **Ressources** ([build.rs](build.rs), [ressources/](ressources/)) : un manifeste (pas d'élévation
  au lancement, `asInvoker` ; comctl32 v6 ; Windows 7 à 11 ; DPI du système) et les informations de
  version (Propriétés > Détails : description, éditeur, version de la release). La release compile
  l'installeur avec la chaîne MSVC, dont `rc.exe` compile les ressources, CRT liée statiquement
  ([.cargo/config.toml](.cargo/config.toml)) : `cargo +stable-x86_64-pc-windows-msvc build --release`,
  avec les Build Tools de Visual Studio. La chaîne GNU compile aussi l'installeur, sans ressources si
  elle n'a pas `windres`.
- **Fenêtres** : le TaskDialog vient de comctl32 v6, qu'active le manifeste embarqué. Sans lui
  (compilation GNU sans `windres`), `gui.rs` l'active au démarrage par un contexte d'activation, à
  partir d'un manifeste écrit dans `%TEMP%\BetterGameExperience`.
- **À jour** : le SHA-256 de la DLL installée est celui du manifeste. L'installeur ne se fie pas au
  numéro de release gravé dans les DLL : une compilation locale n'en a pas.

## Langues

[src/lang.rs](src/lang.rs). Les 11 langues de Better Balanced Game. Les textes viennent de
`textes/<langue>.xml` (clés `LOC_BGD_INST_*`), générés dans [src/textes.rs](src/textes.rs) par
`textes/gen_textes.py` (voir [textes/README.md](../textes/README.md)) : un tableau `Texts` par texte,
une entrée par langue de `LANGUAGES`, dans le même ordre. Les paramètres sont numérotés (`{1}`, `{2}`),
qu'une traduction peut placer dans un autre ordre.

- Le choix se fait par une ligne de liens en bas des fenêtres à choix (pied de page du TaskDialog,
  `TDF_ENABLE_HYPERLINKS`), la langue actuelle sans lien : onze boutons radio prenaient trop de
  place. Il réaffiche aussitôt la fenêtre et est gardé dans
  `%LOCALAPPDATA%\BetterGameExperience\langue.txt` ; sans choix, la langue de Windows si elle est connue,
  sinon l'anglais.
- Les boutons standard de Windows (« Fermer », « OK ») suivent la langue de Windows, pas ce choix.

## Tester sans GitHub

`BGD_DIST` remplace GitHub par un dossier local qui contient une release signée (`manifest.json`,
`manifest.json.sig` et les DLL, produits par `release/manifest.py`) :
```
$env:BGD_DIST = "C:\...\dist"; .\target\release\BetterGameExperience.exe --game-dir "C:\...\faux_jeu"
```
