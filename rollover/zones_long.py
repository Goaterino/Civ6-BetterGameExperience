"""Dépouillement d'un fichier de zones de toute une partie (plusieurs Go), sans le charger en mémoire.

zones.read garde chaque zone en mémoire : bien pour une capture d'une minute, impossible pour une
partie de trois heures (150 millions de zones). Ici, une seule lecture en flux, qui cumule :
- par zone et par seconde : nombre, durée totale, durée max. Les phases (passage de tour ou non)
  et les tranches de temps se calculent ensuite sur ces secondes ;
- les accrocs du thread principal : chaque `Civ6App_UpdateFrame` de plus de `ACCROC_MS`, avec ses
  zones filles les plus longues. Une zone est écrite à sa fermeture, donc ses filles la précèdent
  dans le fichier : on garde les dernières zones de chaque thread.

    .venv\\Scripts\\python.exe -I rollover\\zones_long.py rollover\\captures\\<dossier>

Le résultat va dans zones_long.pkl, relu par analyze.py pour une capture légère (capture.ps1 -Light).
"""
import collections
import mmap
import pathlib
import pickle
import struct
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import zones as zones_reader  # noqa: E402

ACCROC_MS = 50
RUN = 1 << 20  # zones lues d'un bloc entre deux enregistrements de nom
ZONE = struct.Struct("<IIQQ")


class Long:
    def __init__(self):
        self.freq = self.pid = None
        self.names = {}
        self.lost = 0
        self.first = None          # QPC de la première zone
        self.cells = {}            # (identifiant, seconde depuis first) -> [nombre, total QPC, max QPC]
        self.threads = collections.defaultdict(collections.Counter)  # identifiant -> {thread: nombre}
        self.hitches = []          # (début QPC, durée ms, thread, [(nom, durée ms)] des filles)


def read(path):
    out = Long()
    with open(path, "rb") as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as data:
        if data[:8] != zones_reader.MAGIC:
            raise ValueError(f"{path} : pas un fichier de zones")
        out.freq, out.pid, _ = struct.unpack_from("<QII", data, 8)
        frame_id = None
        recent = collections.defaultdict(lambda: collections.deque(maxlen=4000))
        cells, threads = out.cells, out.threads
        i, n = 24, len(data)
        while i < n:
            tag = data[i]
            if tag == 2:
                # Suite d'enregistrements de zone (1 + 24 octets) jusqu'au prochain qui n'en est pas un.
                tags = data[i:i + 25 * RUN:25]
                k = len(tags) - len(tags.lstrip(b"\x02"))
                k = min(k, (n - i) // 25)
                if k == 0:
                    break  # dernier enregistrement coupé
                block = data[i:i + 25 * k]
                for j in range(k):
                    tid, ident, start, end = ZONE.unpack_from(block, 25 * j + 1)
                    if out.first is None:
                        out.first = start
                        sec_q = out.freq
                    d = end - start
                    key = (ident, (start - out.first) // sec_q)
                    c = cells.get(key)
                    if c is None:
                        cells[key] = [1, d, d]
                    else:
                        c[0] += 1
                        c[1] += d
                        if d > c[2]:
                            c[2] = d
                    threads[ident][tid] += 1
                    r = recent[tid]
                    if ident == frame_id and d * 1000 > ACCROC_MS * out.freq:
                        kids = [(z[0], (z[2] - z[1]) * 1000 / out.freq) for z in r if z[1] >= start]
                        kids.sort(key=lambda z: -z[1])
                        out.hitches.append((start, d * 1000 / out.freq, tid, kids[:6]))
                    r.append((ident, start, end))
                i += 25 * k
            elif tag == 1:
                ident, length = struct.unpack_from("<IB", data, i + 1)
                out.names[ident] = data[i + 6:i + 6 + length].decode("utf-8", "replace")
                if out.names[ident] == "Civ6App_UpdateFrame":
                    frame_id = ident
                i += 6 + length
            elif tag == 3:
                out.lost += struct.unpack_from("<Q", data, i + 1)[0]
                i += 9
            else:
                raise ValueError(f"{path} : enregistrement inconnu {tag} à l'octet {i}")
    out.threads = {k: dict(v) for k, v in out.threads.items()}
    out.hitches = [(s, d, t, [(out.names.get(a, a), b) for a, b in kids]) for s, d, t, kids in out.hitches]
    return out


def report(z, lines, passages, wall, slice_min=30, top=15):
    """Section de rapport. `passages` : [(début, fin)] en QPC ; `wall` : QPC -> heure locale."""
    q = z.freq
    in_passage = set()
    for a, b in passages:
        in_passage.update(range(int((a - z.first) // q), int((b - z.first) // q) + 1))
    names = z.names
    frame = next((i for i, n in names.items() if n == "Civ6App_UpdateFrame"), None)
    tot = collections.defaultdict(lambda: [[0, 0, 0], [0, 0, 0]])  # identifiant -> [hors passage, passage]
    secs = [set(), set()]
    for (ident, sec), (n, d, m) in z.cells.items():
        p = int(sec in in_passage)
        t = tot[ident][p]
        t[0] += n
        t[1] += d
        t[2] = max(t[2], m)
        secs[p].add(sec)
    ms = 1000 / q
    lines.append("### Zones du profileur de l'exe, toute la partie\n")
    lines.append(f"- {sum(c[0] for c in z.cells.values())} zones de {len(names)} noms ; {z.lost} perdues. "
                 f"{len(secs[0])} s hors passage, {len(secs[1])} s en passage de tour.\n")
    for p, title in ((0, "hors passage"), (1, "passage de tour")):
        frames = tot[frame][p][0] if frame is not None else 0
        if not frames:
            continue
        lines.append(f"#### {title.capitalize()} : {frames} images, temps réel total (filles comprises)\n")
        lines.append("| zone | nombre | total (s) | par image (ms) | moyenne (ms) | max (ms) | threads |")
        lines.append("|---|---:|---:|---:|---:|---:|---|")
        for ident, v in sorted(tot.items(), key=lambda kv: -kv[1][p][1])[:top]:
            n, d, m = v[p]
            th = len(z.threads.get(ident, {}))
            lines.append(f"| `{names.get(ident, ident)}` | {n} | {d / q:.0f} | {d * ms / frames:.2f} | "
                         f"{d * ms / n:.3f} | {m * ms:.0f} | {th} |")
        lines.append("")
    # Évolution du coût par image du thread principal, par tranche de temps.
    watch = [i for i, n in names.items() if n in (
        "Civ6App_UpdateFrame", "Civ6App_UpdateNetwork", "Civ6App_SteamAPI_RunCallbacks", "WorldView_Render")]
    slices = collections.defaultdict(lambda: collections.defaultdict(lambda: [0, 0]))
    for (ident, sec), (n, d, m) in z.cells.items():
        if ident in watch and sec not in in_passage:
            s = slices[sec // (slice_min * 60)][ident]
            s[0] += n
            s[1] += d
    if frame is not None and slices:
        lines.append(f"#### Hors passage, par tranche de {slice_min} min : ms par image\n")
        order = sorted(watch, key=lambda i: names[i] != "Civ6App_UpdateFrame")
        lines.append("| début | images/s | " + " | ".join(f"`{names[i]}`" for i in order) + " |")
        lines.append("|---|---:|" + "---:|" * len(order))
        for k in sorted(slices):
            row = slices[k]
            frames = row[frame][0]
            if not frames:
                continue
            span = sum(1 for s in secs[0] if s // (slice_min * 60) == k)
            lines.append(f"| {wall(z.first + k * slice_min * 60 * q):%H:%M} | {frames / max(span, 1):.0f} | "
                         + " | ".join(f"{row[i][1] * ms / frames:.2f}" for i in order) + " |")
        lines.append("")
    if z.hitches:
        lines.append(f"#### Accrocs : images de plus de {ACCROC_MS} ms ({len(z.hitches)})\n")
        cause = collections.Counter()
        cause_ms = collections.Counter()
        for _, d, _, kids in z.hitches:
            name = kids[0][0] if kids else "?"
            cause[name] += 1
            cause_ms[name] += d
        lines.append("Zone fille la plus longue de chaque accroc :\n")
        lines.append("| zone | accrocs | durée moyenne de l'image (ms) |")
        lines.append("|---|---:|---:|")
        for name, n in cause.most_common(12):
            lines.append(f"| `{name}` | {n} | {cause_ms[name] / n:.0f} |")
        lines.append("\nLes 10 plus longs :\n")
        for s, d, _, kids in sorted(z.hitches, key=lambda h: -h[1])[:10]:
            lines.append(f"- {wall(s):%H:%M:%S} : {d:.0f} ms ; "
                         + ", ".join(f"`{k}` {v:.0f}" for k, v in kids[:3]))
        lines.append("")


def main():
    folder = pathlib.Path(sys.argv[1])
    files = sorted(folder.glob("zones-*.bin")) + sorted(folder.glob("zones-*.big"))
    if not files:
        sys.exit(f"Aucun fichier de zones dans {folder}")
    import zones_long  # la classe Long sous le nom du module, pas __main__, pour que analyze.py relise
    z = zones_long.read(files[-1])
    with (folder / "zones_long.pkl").open("wb") as f:
        pickle.dump(z, f, protocol=pickle.HIGHEST_PROTOCOL)
    print(f"{sum(c[0] for c in z.cells.values())} zones, {len(z.names)} noms, {len(z.hitches)} accrocs, "
          f"{z.lost} perdues, écrit dans {folder / 'zones_long.pkl'}")


if __name__ == "__main__":
    main()
