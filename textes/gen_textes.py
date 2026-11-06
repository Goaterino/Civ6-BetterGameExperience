"""Textes traduits de BetterGameDLLs (voir textes/README.md).

Lit textes/<langue>.xml, au format des textes de Civ6, et écrit :
- textes/bgd_textes.lua : la table BGD_TXT { clé = { langue = texte } } des blocs Lua des proxys,
  lue par BGD_Txt (textes/bgd_texte.lua) ;
- updater/src/textes.rs : les textes de l'installeur (clés LOC_BGD_INST_*), une constante `Texts`
  par clé, dans l'ordre de `LANGUES` (celui de updater/src/lang.rs).

Vérifie que chaque clé existe en anglais, et que chaque traduction a les mêmes {1}, {2}… que
l'anglais. Un texte absent d'une langue prend l'anglais (signalé ; erreur avec --strict).

    .venv/Scripts/python.exe -I textes/gen_textes.py           # régénère
    .venv/Scripts/python.exe -I textes/gen_textes.py --check   # échoue si les fichiers générés sont périmés
"""
import os
import re
import sys
import xml.etree.ElementTree as ET

RACINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOSSIER = os.path.join(RACINE, "textes")
LUA = os.path.join(DOSSIER, "bgd_textes.lua")
RUST = os.path.join(RACINE, "updater", "src", "textes.rs")

# Les langues de Better Balanced Game, dans l'ordre de l'installeur (updater/src/lang.rs).
LANGUES = ["en_US", "fr_FR", "de_DE", "es_ES", "it_IT", "ja_JP", "ko_KR", "pl_PL", "pt_BR", "ru_RU", "zh_Hans_CN"]
INSTALLEUR = "LOC_BGD_INST_"
PARAM = re.compile(r"\{\d+\}")


def lire(langue):
    chemin = os.path.join(DOSSIER, langue + ".xml")
    textes = {}
    if not os.path.exists(chemin):
        return textes
    for row in ET.parse(chemin).getroot().iter("Row"):
        cle, lang = row.get("Tag"), row.get("Language")
        if lang != langue:
            raise SystemExit(f"{langue}.xml : {cle} en {lang}")
        node = row.find("Text")
        if cle in textes:
            raise SystemExit(f"{langue}.xml : {cle} en double")
        textes[cle] = (node.text or "") if node is not None else ""
    return textes


def lua_string(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\r", "").replace("\n", "\\n") + '"'


def rust_string(s):
    s = s.replace("\\n", "\n")
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def generer(strict):
    tout = {l: lire(l) for l in LANGUES}
    anglais = tout["en_US"]
    erreurs, manques = [], []
    for l in LANGUES[1:]:
        for cle, texte in tout[l].items():
            if cle not in anglais:
                erreurs.append(f"{l} : {cle} n'existe pas en anglais")
            elif sorted(PARAM.findall(texte)) != sorted(PARAM.findall(anglais[cle])):
                erreurs.append(f"{l} : {cle} n'a pas les paramètres de l'anglais")
        for cle in anglais:
            if cle not in tout[l]:
                manques.append(f"{l} : {cle}")
    if erreurs:
        raise SystemExit("\n".join(erreurs))
    if manques:
        print(f"{len(manques)} textes non traduits (l'anglais les remplace) :", file=sys.stderr)
        print("\n".join(manques[:40]), file=sys.stderr)
        if strict:
            raise SystemExit(1)

    lua = [
        "-- Généré par textes/gen_textes.py depuis textes/<langue>.xml, ne pas modifier à la main.",
        "-- Textes des blocs Lua des proxys, lus par BGD_Txt (textes/bgd_texte.lua).",
        "BGD_TXT = {",
    ]
    rust = [
        "//! Généré par textes/gen_textes.py depuis textes/<langue>.xml, ne pas modifier à la main.",
        "//! Textes de l'installeur, un par clé `LOC_BGD_INST_*`, dans l'ordre de `LANGUES`.",
        "#![allow(dead_code)]",
        "",
        "use crate::lang::Texts;",
        "",
        "/// Codes des langues, dans l'ordre de chaque `Texts` (et de `lang::LANGUAGES`).",
        "pub const LANGUES: [&str; " + str(len(LANGUES)) + "] = [" + ", ".join(f'"{l}"' for l in LANGUES) + "];",
    ]
    for cle in sorted(anglais):
        if cle.startswith(INSTALLEUR):
            valeurs = [tout[l].get(cle, anglais[cle]) for l in LANGUES]
            rust.append("")
            rust.append(f"pub const {cle[len(INSTALLEUR):]}: Texts = [")
            rust.extend(f"    {rust_string(v)}," for v in valeurs)
            rust.append("];")
        else:
            parts = [f"{l} = {lua_string(tout[l][cle])}" for l in LANGUES if cle in tout[l]]
            lua.append(f"\t{cle} = {{ " + ", ".join(parts) + " },")
    lua.append("};")
    return "\n".join(lua) + "\n", "\n".join(rust) + "\n"


def main():
    check = "--check" in sys.argv
    lua, rust = generer("--strict" in sys.argv)
    sorties = [(LUA, lua), (RUST, rust)]
    if check:
        perimes = []
        for chemin, contenu in sorties:
            actuel = open(chemin, encoding="utf-8").read() if os.path.exists(chemin) else None
            if actuel is None or actuel.replace("\r\n", "\n") != contenu:
                perimes.append(os.path.relpath(chemin, RACINE))
        if perimes:
            raise SystemExit("à régénérer (textes/gen_textes.py) : " + ", ".join(perimes))
        print("textes générés à jour")
        return
    for chemin, contenu in sorties:
        with open(chemin, "w", encoding="utf-8", newline="\n") as f:
            f.write(contenu)
    print(f"{lua.count(chr(10)) - 4} textes des blocs Lua, {rust.count('pub const') - 1} de l'installeur")


if __name__ == "__main__":
    main()
