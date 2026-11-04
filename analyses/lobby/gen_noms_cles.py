"""Construit noms_cles.tsv : hachage -> nom des clés de configuration.

Le jeu journalise les clés qu'il ne connaît pas en `Unknown Variant Key(<hachage>)`. Le hachage est
celui de l'exe (1407a2330) : CRC32 réfléchi, polynôme 0xedb88320, valeur initiale 0xffffffff et
**sans** XOR final, soit ~zlib.crc32(nom). Ce script hache tous les identifiants trouvés dans les
fichiers de configuration du jeu, des DLC et des mods du Workshop, et ne garde que les hachages
présents dans les journaux donnés (la table complète fait 11 Mo).

    python -I gen_noms_cles.py net_message_debug.log [autre.log ...]
"""
import pathlib
import re
import sys
import zlib

STEAM = pathlib.Path(r"C:\Program Files (x86)\Steam\steamapps")
DOSSIERS = [STEAM / "common" / "Sid Meier's Civilization VI", STEAM / "workshop" / "content" / "289070"]
IDENT = re.compile(rb"[A-Za-z_][A-Za-z0-9_]{2,63}")


def cle(nom):
    return (~zlib.crc32(nom.encode())) & 0xFFFFFFFF


def main():
    vus = set()
    for j in sys.argv[1:]:
        vus.update(int(h) for h in re.findall(r"Unknown Variant Key\((\d+)\)", pathlib.Path(j).read_text(
            encoding="utf-8", errors="replace")))
    noms = set()
    for d in DOSSIERS:
        for f in d.rglob("*"):
            if f.suffix.lower() in (".xml", ".sql", ".lua", ".modinfo") and f.is_file():
                noms.update(m.group().decode() for m in IDENT.finditer(f.read_bytes()))
    table = {}
    for n in sorted(noms):
        if cle(n) in vus:
            table.setdefault(cle(n), []).append(n)
    out = pathlib.Path(__file__).with_name("noms_cles.tsv")
    with open(out, "w", encoding="utf-8", newline="\n") as w:
        for h in sorted(table):
            w.write(f"{h}\t{'|'.join(table[h])}\n")
    print(f"{len(vus)} clés inconnues dans les journaux, {len(table)} nommées -> {out}")


if __name__ == "__main__":
    main()
