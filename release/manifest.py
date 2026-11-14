"""Manifeste signé d'une release de BetterGameDLLs.

Le manifeste liste chaque fichier de la release (nom, dossier de destination dans le jeu, taille,
SHA-256). Sa signature ed25519 (manifest.json.sig, en base64) porte sur ses octets exacts : l'updater
vérifie la signature avec la clé publique release/public_key.txt, puis le SHA-256 de chaque fichier
téléchargé. La clé privée vit hors du dépôt et dans le secret GitHub BGD_SIGNING_KEY.

    python release/manifest.py keygen --out <chemin hors du dépôt>
    python release/manifest.py build --version 0.2.0 --out dist/manifest.json dist/
    python release/manifest.py sign dist/manifest.json          (clé dans $BGD_SIGNING_KEY)
    python release/manifest.py verify dist/manifest.json
"""

import argparse
import base64
import hashlib
import json
import os
import re
import sys
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey, Ed25519PublicKey

ROOT = Path(__file__).resolve().parent.parent
PUBLIC_KEY = Path(__file__).resolve().parent / "public_key.txt"

# Fichiers d'une release, avec leur dossier dans le jeu (relatif à la racine de Civ6).
FILES = {
    "BGD_GameCore_XP2.dll": "DLC/Expansion2/Binaries/Win64",
    "winmm.dll": "Base/Binaries/Win64Steam",
}


def lobby_version():
    """Version annoncée dans le salon multijoueur (proxy/src/version.rs)."""
    src = (ROOT / "proxy/src/version.rs").read_text(encoding="utf-8")
    return re.search(r'pub const VERSION: &str = "([^"]+)";', src).group(1)


def keygen(args):
    out = Path(args.out)
    if out.exists():
        sys.exit(f"{out} existe déjà : on n'écrase pas une clé.")
    if ROOT in out.resolve().parents:
        sys.exit("La clé privée ne doit pas être dans le dépôt.")
    key = Ed25519PrivateKey.generate()
    seed = key.private_bytes_raw()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(base64.b64encode(seed).decode() + "\n")
    pub = key.public_key().public_bytes_raw()
    PUBLIC_KEY.write_text(base64.b64encode(pub).decode() + "\n")
    print(f"Clé privée : {out}\nClé publique : {PUBLIC_KEY}")


def build(args):
    if not re.fullmatch(r"\d+\.\d+\.\d+", args.version):
        sys.exit(f"Version invalide : {args.version} (attendu X.Y.Z)")
    dist = Path(args.dist)
    files = []
    for name, dest in FILES.items():
        data = (dist / name).read_bytes()
        files.append({"name": name, "dest": dest, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
    manifest = {
        "version": args.version,
        "lobby_version": lobby_version(),
        "commit": os.environ.get("GITHUB_SHA", ""),
        "files": files,
    }
    Path(args.out).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(Path(args.out).read_text(encoding="utf-8"))


def sign(args):
    seed = os.environ.get("BGD_SIGNING_KEY")
    if not seed:
        sys.exit("BGD_SIGNING_KEY absente.")
    key = Ed25519PrivateKey.from_private_bytes(base64.b64decode(seed.strip()))
    if key.public_key().public_bytes_raw() != base64.b64decode(PUBLIC_KEY.read_text().strip()):
        sys.exit(f"BGD_SIGNING_KEY ne correspond pas à {PUBLIC_KEY}.")
    data = Path(args.manifest).read_bytes()
    Path(args.manifest + ".sig").write_text(base64.b64encode(key.sign(data)).decode() + "\n")
    print(f"Signé : {args.manifest}.sig")


def verify(args):
    pub = Ed25519PublicKey.from_public_bytes(base64.b64decode(PUBLIC_KEY.read_text().strip()))
    manifest = Path(args.manifest)
    data = manifest.read_bytes()
    pub.verify(base64.b64decode(Path(args.manifest + ".sig").read_text().strip()), data)
    for f in json.loads(data)["files"]:
        if hashlib.sha256((manifest.parent / f["name"]).read_bytes()).hexdigest() != f["sha256"]:
            sys.exit(f"SHA-256 différent : {f['name']}")
    print("Signature et SHA-256 valides.")


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(required=True)
    p = sub.add_parser("keygen")
    p.add_argument("--out", required=True)
    p.set_defaults(func=keygen)
    p = sub.add_parser("build")
    p.add_argument("--version", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("dist")
    p.set_defaults(func=build)
    p = sub.add_parser("sign")
    p.add_argument("manifest")
    p.set_defaults(func=sign)
    p = sub.add_parser("verify")
    p.add_argument("manifest")
    p.set_defaults(func=verify)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
