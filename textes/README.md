# Textes traduits

Tous les textes que BetterGameDLLs affiche, en jeu et dans l'installeur, dans les 11 langues de
Better Balanced Game : en_US, fr_FR, de_DE, es_ES, it_IT, ja_JP, ko_KR, pl_PL, pt_BR, ru_RU,
zh_Hans_CN. Les textes des mods d'interface repris (Better Report Screen…) restent ceux de leurs
auteurs : voir [proxy/gen_textes.py](../proxy/gen_textes.py).

## Les fichiers

| fichier | rôle |
|---|---|
| `<langue>.xml` | **la source** : un fichier par langue, au format des textes de Civ6 (`<Row Tag="LOC_BGD_…" Language="…"><Text>…</Text></Row>`), le format que connaissent les traducteurs de BBG |
| [gen_textes.py](gen_textes.py) | vérifie les XML et écrit les deux fichiers générés |
| `bgd_textes.lua` | généré : la table `BGD_TXT { clé = { langue = texte } }` des blocs Lua |
| `../updater/src/textes.rs` | généré : les textes de l'installeur (clés `LOC_BGD_INST_*`), une constante par clé |
| [bgd_texte.lua](bgd_texte.lua) | `BGD_Txt`, `BGD_TxtAnglais`, `BGD_TxtChat` (voir plus bas) |
| [test_textes.lua](test_textes.lua) | test hors jeu des trois fonctions, dans chaque langue |

Après toute modification d'un XML :

```bash
.venv/Scripts/python.exe -I textes/gen_textes.py --strict
```

```bash
lua textes/test_textes.lua
```

Le workflow de release refuse un fichier généré périmé, une clé absente d'une langue, ou une
traduction dont les `{1}`, `{2}`… ne sont pas ceux de l'anglais (`--check --strict`).

## Règles pour traduire

- `{1}`, `{2}`… sont remplacés par des valeurs (noms de joueurs, versions, nombres). Chacun doit
  apparaître dans la traduction, dans l'ordre que la langue demande.
- `[NEWLINE]` est un saut de ligne en jeu ; `\n` en est un dans l'installeur (clés `LOC_BGD_INST_*`).
  Dans l'installeur, la première ligne d'un bouton (avant `\n`) est son titre, la suite sa description.
- On garde tels quels : BetterGameDLLs, POL, « No First/Last moves (BGD) », les noms de mods, de
  fichiers et de réglages, `Lua.log`, `%LOCALAPPDATA%…`.
- Les termes du jeu prennent le nom officiel de la langue (gouverneur, personnage illustre, combat
  rapide, Médina…), relevé dans les textes du jeu (`Base/Assets/Text`).
- Une clé absente d'une langue prend l'anglais en jeu et dans l'installeur.

## En jeu

Les proxys placent `bgd_textes.lua` puis `bgd_texte.lua` devant chaque bloc Lua qui affiche du texte
(`concat!` des `include_str!`). Chaque contexte d'interface a ses globales : le premier bloc d'un
contexte définit les fonctions.

- `BGD_Txt(clé, …)` : le texte dans la langue du jeu (`Locale.GetCurrentLanguage`), à défaut une
  langue de même base (pt_PT → pt_BR), puis l'anglais, puis la clé elle-même : un texte brut (« x5 »)
  passe tel quel.
- **Messages du chat.** Les messages `BGD -` que les machines s'envoient (salon, départ simultané,
  arrivée d'un joueur) partent en anglais (`BGD_TxtAnglais`) : un joueur sans BetterGameDLLs les lit.
  `BGD_TxtChat` reconnaît, à l'arrivée, un message construit sur une clé `LOC_BGD_CHAT_*` (le texte
  anglais sert de modèle, `{n}` capturés) et le réécrit dans la langue du jeu. Il est appelé par
  l'enveloppe d'`OnChat` du salon (`startup_proxy/src/salon.lua`) et de la partie
  (`proxy/src/ui_depart_chat.lua`). Un message d'une autre version, qui ne correspond à aucun
  modèle, s'affiche tel quel.
- Une nouvelle clé `LOC_BGD_CHAT_*` doit donc garder un texte anglais qu'aucun modèle plus court ne
  prend en entier (les modèles sont essayés du plus long au plus court).

## Dans l'installeur

`updater/src/lang.rs` : `LANGUAGES` dans l'ordre de `LANGUES` du générateur (un test le vérifie). Le
choix se fait par les liens en bas des fenêtres ; sans choix, la langue de Windows, sinon l'anglais.
`lang::f` remplace les `{n}` par les arguments, dans n'importe quel ordre.

## Relecture

Les traductions autres que l'anglais et le français sont un premier jet, fait avec la terminologie
officielle du jeu, sans relecture par des locuteurs natifs. Les corrections des joueurs se font dans
le XML de leur langue, puis le générateur.
