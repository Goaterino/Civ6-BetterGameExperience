"""Manifeste signé d'une release de BetterGameDLLs.

Le manifeste liste chaque fichier de la release (nom, dossier de destination dans le jeu, taille,
SHA-256). Sa signature ed25519 (manifest.json.sig, en base64) porte sur ses octets exacts : l'updater
vérifie la signature avec la clé publique release/public_key.txt, puis le SHA-256 de chaque fichier
téléchargé. La clé privée vit hors du dépôt et dans le secret GitHub BGD_SIGNING_KEY.

    python release/manifest.py keygen --out <chemin hors du dépôt>
    python release/manifest.py build --version 0.2.0 --out dist/manifest.json dist/
    python release/manifest.py sign dist/manifest.json          (clé dans $BGD_SIGNING_KEY)
    python release/manifest.py verify dist/manifest.json
    python release/manifest.py rehash dist/manifest.json        (après la signature Authenticode)
    python release/manifest.py sign-file dist/BetterGameExperience-linux-x86_64   (installeur Linux)
    python release/manifest.py verify-file dist/BetterGameExperience-linux-x86_64
    python release/manifest.py public-pem                       (clé publique embarquée par install.sh)
"""

import argparse
import base64
import hashlib
import json
import os
import re
import sys
from pathlib import Path

from cryptography.hazmat.primitives import serialization
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


def rehash(args):
    """Recalcule taille et SHA-256 des fichiers d'un manifeste existant, à côté de lui, sans toucher
    à version, lobby_version ni commit : la signature Authenticode (release/signer.ps1) change les
    octets des DLL, pas ce qu'elles sont. La signature ed25519 est à refaire ensuite."""
    path = Path(args.manifest)
    manifest = json.loads(path.read_text(encoding="utf-8"))
    for f in manifest["files"]:
        data = (path.parent / f["name"]).read_bytes()
        f["size"] = len(data)
        f["sha256"] = hashlib.sha256(data).hexdigest()
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(path.read_text(encoding="utf-8"))


def sign(args):
    data = Path(args.manifest).read_bytes()
    Path(args.manifest + ".sig").write_text(base64.b64encode(_signing_key().sign(data)).decode() + "\n")
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


def _signing_key():
    seed = os.environ.get("BGD_SIGNING_KEY")
    if not seed:
        sys.exit("BGD_SIGNING_KEY absente.")
    key = Ed25519PrivateKey.from_private_bytes(base64.b64decode(seed.strip()))
    if key.public_key().public_bytes_raw() != base64.b64decode(PUBLIC_KEY.read_text().strip()):
        sys.exit(f"BGD_SIGNING_KEY ne correspond pas à {PUBLIC_KEY}.")
    return key


def sign_file(args):
    """Signature ed25519 d'un fichier hors manifeste (l'installeur Linux), dans `<fichier>.sig` en
    base64 : install.sh la vérifie avec openssl avant de lancer le binaire. Le manifeste ne peut pas
    lister ce fichier : un installeur déjà distribué refuse tout manifeste qui liste plus que les deux
    DLL."""
    data = Path(args.file).read_bytes()
    Path(args.file + ".sig").write_text(base64.b64encode(_signing_key().sign(data)).decode() + "\n")
    print(f"Signé : {args.file}.sig")


def verify_file(args):
    pub = Ed25519PublicKey.from_public_bytes(base64.b64decode(PUBLIC_KEY.read_text().strip()))
    pub.verify(base64.b64decode(Path(args.file + ".sig").read_text().strip()), Path(args.file).read_bytes())
    print(f"Signature valide : {args.file}")


def public_pem(_args):
    """La clé publique au format PEM (SubjectPublicKeyInfo), celle qu'install.sh embarque."""
    pub = Ed25519PublicKey.from_public_bytes(base64.b64decode(PUBLIC_KEY.read_text().strip()))
    print(pub.public_bytes(serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo).decode(), end="")


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
    p = sub.add_parser("rehash")
    p.add_argument("manifest")
    p.set_defaults(func=rehash)
    p = sub.add_parser("sign")
    p.add_argument("manifest")
    p.set_defaults(func=sign)
    p = sub.add_parser("verify")
    p.add_argument("manifest")
    p.set_defaults(func=verify)
    p = sub.add_parser("sign-file")
    p.add_argument("file")
    p.set_defaults(func=sign_file)
    p = sub.add_parser("verify-file")
    p.add_argument("file")
    p.set_defaults(func=verify_file)
    p = sub.add_parser("public-pem")
    p.set_defaults(func=public_pem)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
