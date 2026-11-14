"""Stage the exact ping media into the signed release alongside its DLLs."""
from pathlib import Path
import shutil
import sys

source = Path(__file__).resolve().parents[1] / "ping_assets"
destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
for asset in sorted(source.rglob("*")):
    if asset.is_file():
        shutil.copyfile(asset, destination / asset.name)
