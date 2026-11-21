# Releases

Les releases sont compilées par GitHub Actions ([release.yml](../.github/workflows/release.yml)),
jamais depuis un PC. Seule la signature Authenticode des binaires se fait depuis un PC
([signer.ps1](signer.ps1), voir « Signature Authenticode ») : elle ajoute une signature aux
fichiers compilés par le workflow, sans les recompiler.

## Contenu d'une release

| fichier | rôle |
|---|---|
| `BGD_GameCore_XP2.dll` | proxy de GameCore (`proxy/`), à poser dans `DLC/Expansion2/Binaries/Win64`, à côté de l'original |
| `winmm.dll` | proxy de démarrage (`startup_proxy/`), à poser dans `Base/Binaries/Win64Steam`, à côté de l'exe |
| `BetterGameExperience_V<version>.exe` | l'installeur ([updater/](../updater/README.md)), hors manifeste : c'est lui qui le lit |
| `manifest.json` | version, version du salon (`proxy/src/version.rs`), commit, et pour chaque fichier : dossier de destination, taille, SHA-256 |
| `manifest.json.sig` | signature ed25519 du manifeste, en base64 |

Les scripts Lua et XML sont compilés dans les DLL (`include_str!`) : aucun autre fichier à installer.
Les images et sons des signaux d'équipe (`ping_assets/`) sont embarqués dans `winmm.dll`
(`include_bytes!`, startup_proxy/src/medias.rs), qui les pose dans `DLC/BGEPings` au lancement du
jeu. Le manifeste ne liste que les deux DLL : un installeur déjà distribué (0.10.0 et avant) refuse
tout manifeste qui en liste plus, et c'est lui que lance la mise à jour depuis le jeu. Ce mod local reste décoché (le menu
principal de BGE le décoche) et caché du Contenu additionnel : l'hôte ne l'ajoute à la
configuration de la partie qu'au lancement, quand tous les joueurs ont BGE (fin de
`startup_proxy/src/salon.lua`). Un joueur sans BGE peut donc toujours rejoindre le salon.

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
5. Le workflow vérifie les textes traduits ([textes/](../textes/README.md)), teste, compile, signe le
   manifeste et crée une release **en brouillon**, titrée « BetterGameExperience X.Y.Z (à signer) ».
6. Signer le brouillon depuis le PC qui a accès au certificat (« Signature Authenticode ») :
   ```
   .\release\signer.ps1 v0.2.0
   ```
   Le titre perd « (à signer) ». Ne jamais publier un brouillon qui le porte encore.
7. Relire le brouillon sur GitHub, puis le publier : l'updater ne voit que les releases publiées.
   Pour une version de test, cocher *Set as a pre-release* : seuls les joueurs qui ont coché
   « Pré-release » dans l'installeur la reçoivent, par l'installeur comme par la notification du jeu.
   Garder un tag `vX.Y.Z` simple : la notification du jeu ne compare que ces numéros-là.

Lancé à la main (onglet Actions, *Run workflow*), le workflow compile et signe le manifeste sans créer
de release ; les fichiers sont dans l'artefact du run. `.\release\signer.ps1 -Dossier <dossier>` signe
un tel artefact décompressé, sans passer par GitHub.

## Signature Authenticode

Les deux DLL et l'installeur sont signés avec un certificat de signature de code open source
(Certum). Windows affiche alors l'éditeur dans les propriétés du fichier et dans l'invite de
l'installeur, et la réputation SmartScreen et antivirus s'attache au certificat au lieu de repartir
de zéro à chaque binaire.

La clé du certificat est dans le coffre distant de Certum (SimpliSign). L'application de bureau
SimpliSign Desktop, connectée avec le code à usage unique de l'application mobile, la présente à
Windows comme une carte à puce. La CI ne peut donc pas signer : c'est [signer.ps1](signer.ps1), lancé
depuis le PC, qui :
1. télécharge les fichiers du brouillon et vérifie le manifeste signé par la CI ;
2. signe `BGD_GameCore_XP2.dll`, `winmm.dll` et `BetterGameExperience_V<version>.exe` (signtool,
   SHA-256, horodatage `http://time.certum.pl` : la signature reste valide après l'expiration du
   certificat) ;
3. recalcule le manifeste (`manifest.py rehash` : taille et SHA-256 changent, version et commit
   non), le re-signe en ed25519 et le vérifie ;
4. renvoie les fichiers dans le brouillon et retire « (à signer) » du titre.

Relancé sur un brouillon déjà signé, il ne re-signe pas les fichiers qui portent déjà la signature du
même certificat. Il refuse une release publiée : ses fichiers sont déjà chez les joueurs.

Prérequis, une fois par PC :
- SimpliSign Desktop, connecté, et le certificat importé (le `.pem` téléchargé chez Certum) : il
  doit apparaître, relié à sa clé, dans `Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert` ;
- signtool : Windows SDK, composant « Signing Tools for Desktop Apps » (le script le trouve hors du
  PATH) ;
- Python avec `cryptography`, `gh` connecté, et la clé du manifeste (« Clé de signature »).

Rien de personnel n'est dans le dépôt. Le script choisit le seul certificat de signature de code de
l'utilisateur qui a sa clé ; s'il y en a plusieurs, son empreinte se donne par `-Empreinte`,
`$env:BGE_EMPREINTE_SIGNATURE` ou `%USERPROFILE%\.bgd\empreinte_signature.txt`.

Le certificat se renouvelle chaque année chez Certum. Les binaires déjà publiés restent valides
grâce à l'horodatage.

## Clé de signature

- Clé publique : [public_key.txt](public_key.txt), dans le dépôt ; l'updater l'embarque.
- Clé privée : hors du dépôt (`%USERPROFILE%\.bgd\signing_key.txt`), et dans le secret GitHub
  `BGD_SIGNING_KEY` :
  ```
  Get-Content "$env:USERPROFILE\.bgd\signing_key.txt" | gh secret set BGD_SIGNING_KEY
  ```
  À sauvegarder ailleurs (gestionnaire de mots de passe) : perdue, il faudrait publier un updater
  avec une nouvelle clé publique.

Vérifier une release téléchargée (dossier contenant les 5 fichiers) :
```
python release/manifest.py verify <dossier>/manifest.json
Get-AuthenticodeSignature <dossier>\*.dll, <dossier>\*.exe
```
