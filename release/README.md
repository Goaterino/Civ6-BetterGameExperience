# Releases

Les releases sont construites par GitHub Actions ([release.yml](../.github/workflows/release.yml)),
jamais depuis un PC : n'importe qui peut relier un binaire publié au commit qui l'a produit.

## Contenu d'une release

| fichier | rôle |
|---|---|
| `BGD_GameCore_XP2.dll` | proxy de GameCore (`proxy/`), à poser dans `DLC/Expansion2/Binaries/Win64`, à côté de l'original |
| `winmm.dll` | proxy de démarrage (`startup_proxy/`), à poser dans `Base/Binaries/Win64Steam`, à côté de l'exe |
| `BetterGameExperience_V<version>.exe` | l'installeur ([updater/](../updater/README.md)), hors manifeste : c'est lui qui le lit |
| `manifest.json` | version, version du salon (`proxy/src/version.rs`), commit, et pour chaque fichier : dossier de destination, taille, SHA-256 |
| `manifest.json.sig` | signature ed25519 du manifeste, en base64 |

Les scripts d'interface sont compilés dans les DLL (`include_str!`).
`release/ping_assets.py dist` ajoute les neuf textures DDS, la banque audio, ses deux
fichiers de configuration et `BGEPings.modinfo` depuis `ping_assets/`.
Le manifeste signé les installe dans `DLC/BGEPings`. Le menu principal de BGE active
automatiquement ces médias locaux, y compris dans les profils déjà existants.

L'updater lit `releases/latest/download/manifest.json` (une redirection, sans l'API GitHub ni sa
limite de requêtes), vérifie la signature avec [public_key.txt](public_key.txt), puis télécharge
chaque fichier depuis `releases/download/v<version>/` et contrôle son SHA-256. Ces adresses ne
répondent que si le dépôt est public. Pour l'installeur, un
proxy installé est à jour si son SHA-256 est celui du manifeste.

Sur un tag, le workflow grave aussi la version dans les DLL (`BGD_RELEASE`, voir
[proxy/src/version.rs](../proxy/src/version.rs)) : le proxy de démarrage la compare au manifeste pour
proposer les mises à jour au menu principal. Une compilation locale n'en a pas, et ne vérifie rien.

`version` (le tag) et `lobby_version` sont distincts : une release qui ne touche ni la simulation ni
l'échange du salon garde la même `lobby_version` et reste compatible en multijoueur avec la précédente.

## Diagnostic

Les outils d'analyse ne partent pas en release. Ils sont derrière la feature Cargo `diag` des deux
proxys, activée par défaut : un `cargo build --release` local les contient, le workflow compile avec
`--no-default-features`. Sans elle :
- le proxy de démarrage ne pose pas le détour du profil des zones (`profil_zones`, `src/zones.rs`) ;
- le proxy de GameCore ne lit pas `ui_journal`, `ui_mesure_epingles` ni `ui_diag_accords`, qui
  restent sur off même s'ils sont dans `rollover_proxy.cfg` ;
- l'onglet BetterGameExperience ne montre pas ces quatre réglages (`diag = true` dans
  `startup_proxy/src/options_proxy.lua`).

`depart_signaux` (signaux du départ simultané) reste en release. Les scripts de `rollover/`,
`netcode/`… ne sont jamais dans une release. Un réglage d'analyse ajouté plus tard prend aussi
`#[cfg(feature = "diag")]` et `diag = true`. Le workflow lance les tests avec et sans la feature.

Une mise à jour par l'updater remplace un build local par celui de la release, sans diagnostic : il
faut recompiler et réinstaller pour les retrouver.

## Publier une release

1. Vérifier que tout est commité et poussé : la release ne compile que le contenu du tag.
   `CHANGELOG.md` doit avoir une section `## BetterGameExperience <version>` (le tag sans `v`) : le
   workflow en fait les notes de la release ([notes.py](notes.py)), avec les liens vers les
   traductions de `changelog/`, et échoue sans elle. Mettre aussi à jour `changelog/<langue>.md`.
2. Si la release change la simulation ou l'échange du salon, augmenter `VERSION` dans
   `proxy/src/version.rs` (X.Y.0). Jamais pour une release qui ne change ni l'une ni l'autre.
3. Choisir le tag : le X.Y du tag est celui de `VERSION`. Si `VERSION` vient de monter, `vX.Y.0`
   (`VERSION` 0.2.0 → `v0.2.0`) ; sinon, le patch suivant du dernier tag (`v0.2.0` → `v0.2.1`).
4. Taguer et pousser :
   ```
   git tag v0.2.0
   git push origin v0.2.0
   ```
5. Le workflow vérifie les textes traduits ([textes/](../textes/README.md)), teste, compile, signe et
   crée une release **en brouillon**. La relire sur GitHub, puis
   la publier : l'updater ne voit que les releases publiées.
   Pour une version de test, cocher *Set as a pre-release* : seuls les joueurs qui ont coché
   « Pré-release » dans l'installeur la reçoivent, par l'installeur comme par la notification du jeu.
   Garder un tag `vX.Y.Z` simple : la notification du jeu ne compare que ces numéros-là.

Lancé à la main (onglet Actions, *Run workflow*), le workflow compile et signe sans créer de release ;
les fichiers sont dans l'artefact du run.

## Clé de signature

- Clé publique : [public_key.txt](public_key.txt), dans le dépôt ; l'updater l'embarque.
- Clé privée : hors du dépôt (`%USERPROFILE%\.bgd\signing_key.txt`), et dans le secret GitHub
  `BGD_SIGNING_KEY` :
  ```
  Get-Content "$env:USERPROFILE\.bgd\signing_key.txt" | gh secret set BGD_SIGNING_KEY
  ```
  À sauvegarder ailleurs (gestionnaire de mots de passe) : perdue, il faudrait publier un updater
  avec une nouvelle clé publique.

Vérifier une release téléchargée (dossier contenant les 4 fichiers) :
```
python release/manifest.py verify <dossier>/manifest.json
```
