# Installeur : BetterGameDLLs.exe

Installe, met à jour, répare ou retire les deux proxys à partir de la dernière release publiée sur
GitHub (voir [release/README.md](../release/README.md)). Il est publié avec chaque release.

## Utilisation

- **Double-clic** : une fenêtre (TaskDialog de Windows) affiche le dossier du jeu, la dernière
  version et l'état de l'installation (pas installé, à jour, mise à jour disponible, à réparer), le
  détail par DLL sous « Détails », et propose installer, mettre à jour ou réparer, et désinstaller.
  Après une installation, elle donne l'option de lancement Steam, avec un bouton pour la copier.
  Chaque fenêtre à choix propose aussi la langue (boutons radio, voir Langues).
- **`--game-dir "<dossier>"`** : même fenêtre, pour un jeu hors des bibliothèques Steam.
- **Options de lancement Steam** (Propriétés > Général) :
  ```
  "%LOCALAPPDATA%\BetterGameDLLs\BetterGameDLLs.exe" %command%
  ```
  À chaque lancement, l'installeur vérifie la dernière release (environ 1 s, sans fenêtre si tout
  est à jour), installe si besoin derrière une fenêtre de progression, puis lance le jeu et attend
  sa fin, pour que Steam le voie en cours. Depuis la release qui suit la v0.3.0, Steam ne retire plus les proxys :
  cette option ne sert plus qu'aux mises à jour de BetterGameDLLs. Un échec (GitHub injoignable, droits refusés) est affiché,
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
- **Téléchargement** : par le `curl.exe` de Windows, dans `%TEMP%\BetterGameDLLs`. Le manifeste doit
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
  en administrateur (`--apply`, invite UAC, sans fenêtre), qui revérifie signature et SHA-256 avant
  de copier.
- **Emplacement fixe** : à l'ouverture de sa fenêtre et après chaque installation, l'installeur se copie dans
  `%LOCALAPPDATA%\BetterGameDLLs\BetterGameDLLs.exe`, s'il n'y tourne pas déjà. La notification du
  jeu le lance là, et l'option de lancement Steam proposée le désigne : peu importe où le joueur a
  téléchargé l'exe, et la copie suit le dernier installeur lancé.
- **Pas de console** : l'exe est une application fenêtrée, et `curl.exe` et `tasklist` sont lancés
  sans fenêtre (`CREATE_NO_WINDOW`).
- **Fenêtres** : le TaskDialog vient de comctl32 v6, qu'on active d'habitude par un manifeste
  embarqué dans l'exe. La chaîne GNU n'ayant pas `windres`, `gui.rs` l'active au démarrage par un
  contexte d'activation, à partir d'un manifeste écrit dans `%TEMP%\BetterGameDLLs`.
- **À jour** : le SHA-256 de la DLL installée est celui du manifeste. L'installeur ne se fie pas au
  numéro de release gravé dans les DLL : une compilation locale n'en a pas.

## Langues

[src/lang.rs](src/lang.rs). Français et anglais ; chaque texte est un tableau à une entrée par
langue de `LANGUAGES`, dans le même ordre, si bien qu'ajouter une langue fait échouer la compilation
de chaque texte qui ne l'a pas encore. Le choix du joueur, fait par les boutons radio des fenêtres,
réaffiche aussitôt la fenêtre et est gardé dans `%LOCALAPPDATA%\BetterGameDLLs\langue.txt` ; sans
choix, la langue de Windows si elle est connue, sinon l'anglais.

- Les libellés portent un drapeau en emoji (🇫🇷, 🇬🇧), mais Windows n'a pas de glyphe pour les
  drapeaux : il les montre en petites lettres (« FR », « GB »).
- Les boutons standard de Windows (« Fermer », « OK ») suivent la langue de Windows, pas ce choix.

## Tester sans GitHub

`BGD_DIST` remplace GitHub par un dossier local qui contient une release signée (`manifest.json`,
`manifest.json.sig` et les DLL, produits par `release/manifest.py`) :
```
$env:BGD_DIST = "C:\...\dist"; .\target\release\BetterGameDLLs.exe --game-dir "C:\...\faux_jeu"
```
