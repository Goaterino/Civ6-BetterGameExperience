"""Notes d'une release GitHub, tirées de CHANGELOG.md (voir release/README.md).

Écrit sur la sortie la section « ## BetterGameExperience <version> » de CHANGELOG.md, puis les liens vers
les changelogs traduits (changelog/<langue>.md) au commit du tag. Échoue si la section manque : une
release ne part pas sans son changelog.

    python release/notes.py 0.5.0 owner/repo v0.5.0 > notes.md
"""
import os
import sys

RACINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LANGUES = [
    ("fr", "Français"), ("de", "Deutsch"), ("es", "Español"), ("it", "Italiano"), ("ja", "日本語"),
    ("ko", "한국어"), ("pl", "Polski"), ("pt-BR", "Português (Brasil)"), ("ru", "Русский"), ("zh-CN", "简体中文"),
]


def section(texte, version):
    lignes = texte.splitlines()
    titre = f"## BetterGameExperience {version}"
    debut = next((i for i, l in enumerate(lignes) if l == titre or l.startswith(titre + ":") or l.startswith(titre + " ")), None)
    if debut is None:
        return None
    fin = next((i for i in range(debut + 1, len(lignes)) if lignes[i].startswith("## ")), len(lignes))
    return "\n".join(lignes[debut + 1:fin]).strip()


def main():
    version, repo, tag = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(os.path.join(RACINE, "CHANGELOG.md"), encoding="utf-8") as f:
        corps = section(f.read(), version)
    if corps is None:
        raise SystemExit(f"CHANGELOG.md n'a pas de section « ## BetterGameExperience {version} »")
    liens = [f"[{nom}](https://github.com/{repo}/blob/{tag}/changelog/{code}.md)"
             for code, nom in LANGUES if os.path.exists(os.path.join(RACINE, "changelog", code + ".md"))]
    sortie = corps
    if liens:
        sortie += "\n\n---\n\n" + " · ".join(liens) + "\n"
    sys.stdout.reconfigure(encoding="utf-8")
    print(sortie)


if __name__ == "__main__":
    main()
