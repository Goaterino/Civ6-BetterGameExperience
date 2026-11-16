# Installeur macOS de test

De quoi tester une installation macOS de zéro, sans la clé privée des releases ni release Mac
publiée.

## Contenu

| fichier | rôle |
|---|---|
| `public_key.txt`, `cle_privee_de_test.txt` | **clé de test**, publique exprès : rien de ce qu'elle signe ne doit partir chez un joueur |
| `dylib_test.c` | fausse dylib : la marque que cherche l'installeur, et une ligne dans `injection.log` quand elle est chargée |
| `faux_jeu.c` | faux `Civ6_Exe_Child`, que Steam lancerait par `%command%` |
| `fausse_release.sh` | compile les fausses dylibs et le faux jeu, écrit `manifest-macos.json` (version 9.9.9) et le signe avec la clé de test |
| `essai.sh` | l'installation de test, pas à pas (voir plus bas) |

L'installeur de test est le lanceur compilé avec la feature `cle-de-test` (`BGE_TEST=1 bash
updater/macos/bundle.sh`) : il ne fait confiance qu'à cette clé de test, et sa fenêtre l'annonce
(« INSTALLEUR DE TEST »). Le workflow de release ne l'active jamais.

## Lancer l'essai

Depuis la CI : télécharger l'artefact **BetterGameExperience-macOS-test** du run de la branche,
l'extraire, puis dans un terminal :

```bash
bash essai.sh
```

En local :

```bash
BGE_TEST=1 bash updater/macos/bundle.sh
bash updater/macos/test/fausse_release.sh
bash updater/target/macos-test/essai.sh
```

`essai.sh` travaille dans `essai/`, à côté de lui :

1. extrait l'installeur, comme un téléchargement ;
2. l'ouvre avec `BGD_DIST` (la fausse release au lieu de GitHub), `BGE_DATA_DIR` et un faux `HOME`
   dont le dossier Steam contient des **copies** de tes `localconfig.vdf` et le faux jeu. Clique
   **Installer**. Si ton Steam est ouvert, **Redémarrer Steam** ferme puis rouvre ton vrai Steam,
   mais l'option de lancement est écrite dans les copies ;
3. vérifie les dylibs posées (SHA-256 du manifeste), la copie du lanceur et l'option de lancement
   écrite pour chaque compte ;
4. lance le faux jeu par le lanceur installé, comme Steam, et montre ce que les dylibs ont noté en se
   chargeant.

Rien n'est écrit dans `~/Library` ni dans la vraie configuration de Steam. Hors de cet essai, ne pas
ouvrir l'installeur de test d'un double-clic : sans ces variables, il chercherait la release sur
GitHub et configurerait le vrai Steam.
