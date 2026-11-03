"""Textes des mods d'interface repris par le proxy (voir mods_ui/README.md).

Les mods ajoutent leurs textes à la base de localisation ; repris dans le proxy, sans le mod, ces clés
n'existent plus. Ce script lit les XML de textes des mods et écrit :
- src/mods/textes.lua : la table BGD_TEXTES { clé = { langue = texte } }, lue par BGD_Lookup
  (src/mods/commun.lua) ;
- src/mods/textes.tsv : clé, langue, texte, lus par le proxy pour traduire les String="LOC_…" des XML
  embarqués (mods_ui::traduire_xml).

    .venv/Scripts/python.exe -I proxy/gen_textes.py "<dossier Workshop 289070>" proxy/src/mods
"""
import os
import sys
import xml.etree.ElementTree as ET

# (mod, fichiers de textes relatifs au dossier du mod)
SOURCES = [
    ("1312585482", ["Config/BRS_Config_Text.xml", "Text/BetterReportScreen_Text.xml"]
     + [f"Text/BetterReportScreen_Text_{l}.xml" for l in
        ["de_DE", "es_ES", "fr_FR", "it_IT", "ja_JP", "ko_KR", "pl_PL", "pt_BR", "ru_RU", "zh_Hans_CN", "zh_Hant_HK"]]),
    ("2997927787", ["TPE/BaseText.xml", "TPE/TopPanel_Text.xml"]),
]

# Sections dont les lignes sans attribut Language sont en anglais.
ENGLISH_SECTIONS = {"BaseGameText", "EnglishText"}


def read(path, out):
    root = ET.parse(path).getroot()
    for section in root:
        for row in section:
            tag = row.get("Tag")
            if not tag:
                continue
            lang = row.get("Language") or ("en_US" if section.tag in ENGLISH_SECTIONS else None)
            if lang is None:
                continue
            text = row.get("Text")
            node = row.find("Text")
            if node is not None:
                text = node.text or ""
            if text is None:
                continue
            out.setdefault(tag, {})[lang] = text.strip()


def lua_string(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\r", "").replace("\n", "\\n") + '"'


def main():
    workshop, dest = sys.argv[1], sys.argv[2]
    texts = {}
    for mod, files in SOURCES:
        for f in files:
            read(os.path.join(workshop, mod, f), texts)
    lua = [
        "-- Généré par proxy/gen_textes.py, ne pas modifier à la main.",
        "-- Textes des mods d'interface repris par le proxy : Better Report Screen (Infixo) et",
        "-- TopPanel Extension Pro (号码菌 / Azukii). Lu par BGD_Lookup (commun.lua).",
        "BGD_TEXTES = {",
    ]
    tsv = []
    for tag in sorted(texts):
        langs = texts[tag]
        lua.append(f"\t{tag} = {{ " + ", ".join(f"{l} = {lua_string(langs[l])}" for l in sorted(langs)) + " },")
        for l in sorted(langs):
            tsv.append(f"{tag}\t{l}\t{langs[l].replace(chr(9), ' ').replace(chr(13), '').replace(chr(10), ' ')}")
    lua.append("};")
    with open(os.path.join(dest, "textes.lua"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lua) + "\n")
    with open(os.path.join(dest, "textes.tsv"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(tsv) + "\n")
    print(f"{len(texts)} clés, {len(tsv)} textes")


if __name__ == "__main__":
    main()
