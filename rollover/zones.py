"""Zones du profileur intégré de l'exe, notées par le proxy de démarrage (`profil_zones=on`,
startup_proxy/src/zones.rs) dans `%LOCALAPPDATA%\\Civ6RolloverProxy\\zones\\zones-<pid>-<t>.bin`.

Chaque zone porte son thread, son nom et ses instants de début et de fin en QPC, l'horloge des
traces WPR lues par etl.py : on peut donc attribuer chaque échantillon CPU à la zone ouverte sur
son thread à cet instant.

Usage seul :
    python -I rollover/zones.py <fichier .bin>
"""
import bisect
import collections
import struct
import sys

MAGIC = b"BGDZONE1"


class Zones:
    def __init__(self):
        self.freq = self.pid = None
        self.names = {}             # identifiant -> nom
        self.zones = []             # (thread, identifiant du nom, début, fin)
        self.lost = 0


def read(path):
    z = Zones()
    data = open(path, "rb").read()
    if data[:8] != MAGIC:
        raise ValueError(f"{path} : pas un fichier de zones")
    z.freq, z.pid, _ = struct.unpack_from("<QII", data, 8)
    i, n = 24, len(data)
    zone = struct.Struct("<IIQQ")
    while i < n:
        tag = data[i]
        i += 1
        if tag == 2:
            if i + zone.size > n:
                break  # dernier enregistrement coupé : le jeu tournait encore
            z.zones.append(zone.unpack_from(data, i))
            i += zone.size
        elif tag == 1:
            if i + 5 > n:
                break
            ident, length = struct.unpack_from("<IB", data, i)
            z.names[ident] = data[i + 5:i + 5 + length].decode("utf-8", "replace")
            i += 5 + length
        elif tag == 3:
            if i + 8 > n:
                break
            z.lost += struct.unpack_from("<Q", data, i)[0]
            i += 8
        else:
            raise ValueError(f"{path} : enregistrement inconnu {tag} à l'octet {i - 1}")
    return z


def stats(z, lo=None, hi=None):
    """Par nom de zone : nombre, durée totale, durée propre (sans les zones filles du même thread),
    durée max, en ms, des zones fermées entre lo et hi (QPC)."""
    ms = 1000 / z.freq
    by_thread = collections.defaultdict(list)
    for tid, ident, a, b in z.zones:
        if (lo is None or b >= lo) and (hi is None or b <= hi):
            by_thread[tid].append((a, -b, ident))
    out = collections.defaultdict(lambda: [0, 0.0, 0.0, 0.0, set()])  # nombre, total, propre, max, threads
    for tid, zs in by_thread.items():
        zs.sort()
        stack = []  # [fin, identifiant, temps des filles]
        def close(entry):
            end, ident, child, start = entry
            d = (end - start) * ms
            s = out[ident]
            s[0] += 1
            s[1] += d
            s[2] += d - child
            s[3] = max(s[3], d)
            s[4].add(tid)
            if stack:
                stack[-1][2] += d
        for a, nb, ident in zs:
            b = -nb
            while stack and stack[-1][0] <= a:
                close(stack.pop())
            stack.append([b, ident, 0.0, a])
        while stack:
            close(stack.pop())
    return out


def attribute(z, samples):
    """Attribue les échantillons CPU [(qpc, thread)] aux zones : propre (zone la plus intérieure
    ouverte sur le thread) et inclusif (toutes les zones ouvertes). Rend deux Counter par nom, et le
    nombre d'échantillons hors de toute zone, par thread."""
    by_thread = collections.defaultdict(list)
    for tid, ident, a, b in z.zones:
        by_thread[tid].append((a, b, ident))
    self_c, incl_c, outside = collections.Counter(), collections.Counter(), collections.Counter()
    per_thread = collections.defaultdict(list)
    for q, tid in samples:
        per_thread[tid].append(q)
    for tid, qs in per_thread.items():
        zs = sorted(by_thread.get(tid, []), key=lambda x: (x[0], -x[1]))
        qs.sort()
        stack, j = [], 0
        for q in qs:
            while j < len(zs) and zs[j][0] <= q:
                while stack and stack[-1][1] < zs[j][0]:
                    stack.pop()
                stack.append(zs[j])
                j += 1
            while stack and stack[-1][1] < q:
                stack.pop()
            # Les zones encore sur la pile mais déjà fermées sous une plus récente : on les filtre.
            open_ = [x for x in stack if x[1] >= q]
            if not open_:
                outside[tid] += 1
                continue
            self_c[open_[-1][2]] += 1
            for ident in {x[2] for x in open_}:
                incl_c[ident] += 1
    return self_c, incl_c, outside


def report(z, lines, lo=None, hi=None, samples=None, sample_ms=1.0, threads=None, top=30):
    """Section de rapport. `samples` : [(qpc, thread)] du jeu dans la fenêtre ; `threads` : noms à
    afficher par thread (thread principal, GameCore…)."""
    st = stats(z, lo, hi)
    if not st:
        lines.append("### Zones du profileur de l'exe\n\nAucune zone dans la fenêtre.\n")
        return
    span = ((hi - lo) / z.freq) if lo is not None and hi is not None else None
    lines.append("### Zones du profileur de l'exe (proxy de démarrage, `profil_zones=on`)\n")
    lines.append(f"- {sum(s[0] for s in st.values())} zones de {len(st)} noms"
                 + (f" sur {span:.1f} s" if span else "") + (f" ; {z.lost} perdues (tampon plein)" if z.lost else "")
                 + ".")
    frames = next((s[0] for i, s in st.items() if z.names.get(i) == "Civ6App_UpdateFrame"), None)
    if frames:
        lines.append(f"- `Civ6App_UpdateFrame` : {frames} images.")
    lines.append("\nTemps réel passé dans chaque zone, propre (sans ses zones filles), tous threads.\n")
    lines.append("| zone | nombre | total (ms) | propre (ms) | moyenne (ms) | max (ms) | threads |")
    lines.append("|---|---:|---:|---:|---:|---:|---|")
    for ident, s in sorted(st.items(), key=lambda kv: -kv[1][2])[:top]:
        th = ", ".join(threads.get(t, str(t)) if threads else str(t) for t in sorted(s[4])[:3])
        lines.append(f"| `{z.names.get(ident, ident)}` | {s[0]} | {s[1]:.0f} | {s[2]:.0f} | {s[1] / s[0]:.3f} | "
                     f"{s[3]:.1f} | {th}{' …' if len(s[4]) > 3 else ''} |")
    if samples:
        self_c, incl_c, outside = attribute(z, samples)
        n = len(samples)
        lines.append(f"\nCPU attribué par les échantillons, par CPU propre ({n} sur les threads qui ont des zones ; "
                     f"{sum(outside.values())} hors de toute zone).\n")
        lines.append("| zone | CPU propre (ms) | CPU inclusif (ms) |")
        lines.append("|---|---:|---:|")
        for ident, k in sorted(self_c.items(), key=lambda kv: -kv[1])[:top]:
            lines.append(f"| `{z.names.get(ident, ident)}` | {k * sample_ms:.0f} | {incl_c[ident] * sample_ms:.0f} |")
    lines.append("")


if __name__ == "__main__":
    z = read(sys.argv[1])
    lines = []
    report(z, lines)
    print("\n".join(lines))
