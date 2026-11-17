"""Analyse des piles CPU d'une capture de chargements (rollover/capture.ps1 -Chargement).

La trace complète (plusieurs Go) se relit en plusieurs minutes : `extrait` n'en garde qu'une fois les
échantillons du jeu autour de chaque chargement, avec leurs piles, dans `<capture>/extrait.pkl`.
Les autres commandes lisent cet extrait.

    python -I analyses/chargement/piles.py extrait   <capture>
    python -I analyses/chargement/piles.py chrono    <capture>
        seconde par seconde, pour chaque chargement : CPU des trois threads les plus chargés et leurs
        modules dominants (temps propre)
    python -I analyses/chargement/piles.py zones     <capture>
        zones de l'exe (profil_zones=on) par phase et par thread, temps propre réel
    python -I analyses/chargement/piles.py phases    <capture> <threads> [<chargements> [<phases>]]
        par phase : modules (propre), adresses inclusives (fonctions GameCore nommées, offsets
        ailleurs) et appelant hors système des échantillons passés dans le noyau ou ntdll
    python -I analyses/chargement/piles.py chaine    <capture> <chargement> <de s> <à s> <threads>
        inclusif de l'exe, feuilles et chemins d'appel (6 dernières frames de l'exe) sur une fenêtre
        en secondes depuis le début du chargement
    python -I analyses/chargement/piles.py appelants <capture> <offset exe>
        chaînes d'appel au-dessus d'un offset de l'exe, par chargement et par thread
    python -I analyses/chargement/piles.py rythme    <capture>
        par phase : images du thread principal, temps de présentation, étapes de
        WorldView_PostLoadUpdate et CPU du jeu hors réseau Steam (attente de la VSync ou calcul)

<threads> : identifiants séparés par des virgules. <chargements> : numéros (1, 2…), séparés par des
virgules. <phases> : noms de LoadGameViewState.log (CONFIGURE_CONTENT…), ou « avant » pour les 6 s qui
précèdent l'écran de chargement. Les offsets de l'exe s'ajoutent à 0x140000000 dans Ghidra.
"""
import bisect
import collections
import datetime as dt
import json
import pathlib
import pickle
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "rollover"))
import chargement  # noqa: E402

SYSTEME = {"noyau", "ntdll.dll", "kernelbase.dll", "kernel32.dll", "ucrtbase.dll", "vcruntime140.dll",
           "msvcp140.dll"}
GAMECORE = "gamecore_xp2_finalrelease.dll"


def extrait(cap):
    import analyze
    tr = analyze.load_trace(cap / "trace.etl")
    start_utc = dt.datetime(1601, 1, 1, tzinfo=dt.timezone.utc) + dt.timedelta(microseconds=tr.start_filetime / 10)
    pid = json.loads((cap / "capture.json").read_text(encoding="utf-8-sig"))["pid"]
    blocs = chargement.blocs(cap)
    f = tr.perf_freq

    def qpc(w):
        return tr.first_qpc + (w.astimezone(dt.timezone.utc) - start_utc).total_seconds() * f

    wins = [(qpc(b["debut"] - dt.timedelta(seconds=8)), qpc((b["fin"] or b["debut"]) + dt.timedelta(seconds=3)))
            for b in blocs]
    samples = [s for s in tr.samples if tr.thread_pid.get(s[1]) == pid and any(a <= s[0] < b for a, b in wins)]
    stacks = {(s[0], s[1]): tr.stacks[(s[0], s[1])] for s in samples if (s[0], s[1]) in tr.stacks}
    out = dict(samples=samples, stacks=stacks, images=[i for i in tr.images if i[0] == pid], freq=f,
               first_qpc=tr.first_qpc, start_utc=start_utc, wins=wins, blocs=blocs, pid=pid)
    with (cap / "extrait.pkl").open("wb") as fh:
        pickle.dump(out, fh, protocol=pickle.HIGHEST_PROTOCOL)
    print(f"{len(samples)} échantillons, {len(stacks)} piles → {cap / 'extrait.pkl'}")


class Extrait:
    def __init__(self, cap):
        with (cap / "extrait.pkl").open("rb") as fh:
            self.__dict__.update(pickle.load(fh))
        self.mods = sorted({(b, s, n) for _, b, s, n in self.images})
        self.starts = [m[0] for m in self.mods]
        self._funcs = None

    def mod(self, ip):
        """(module, offset) d'une adresse."""
        if ip >= 0xFFFF800000000000:
            return ("noyau", 0)
        i = bisect.bisect_right(self.starts, ip) - 1
        if i >= 0 and ip < self.mods[i][0] + self.mods[i][1]:
            return (self.mods[i][2], ip - self.mods[i][0])
        return ("?", ip)

    def qpc(self, w):
        return self.first_qpc + (w.astimezone(dt.timezone.utc) - self.start_utc).total_seconds() * self.freq

    def wall(self, q):
        return (self.start_utc + dt.timedelta(seconds=(q - self.first_qpc) / self.freq)).astimezone().replace(tzinfo=None)

    @property
    def funcs(self):
        if self._funcs is None:
            import analyze
            self._funcs = analyze.Functions(analyze.FUNCTIONS_TSV)
        return self._funcs

    def gc_func(self, off):
        import analyze
        r = self.funcs.find(off + analyze.GAMECORE_BASE)
        return r["adresse"] if r else None

    def label(self, m, off):
        if m == GAMECORE:
            a = self.gc_func(off)
            return self.funcs.label(a) if a else f"gc+{off:x}"
        return f"{m.split('.')[0]}+{off:x}"


def phases_de(b):
    """(nom, début, fin) en heure locale : chaque phase finit à sa ligne du journal (bornes à la
    seconde, d'où la seconde ajoutée) et commence à la fin de la précédente."""
    out, prev = [("avant", b["debut"] - dt.timedelta(seconds=6), b["debut"])], b["debut"]
    for p in b["phases"]:
        out.append((p["nom"], prev, p["t"] + dt.timedelta(seconds=1)))
        prev = p["t"] + dt.timedelta(seconds=1)
    return out


def chrono(cap):
    x = Extrait(cap)
    for b in x.blocs:
        lo, hi = b["debut"] - dt.timedelta(seconds=6), (b["fin"] or b["debut"]) + dt.timedelta(seconds=2)
        sec = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
        for q, tid, ip in x.samples:
            w = x.wall(q)
            if lo <= w < hi:
                sec[int((w - b["debut"]).total_seconds() // 1)][tid][x.mod(ip)[0]] += 1
        print(f"\n=== chargement {b['rang'] + 1}, {b['debut']:%H:%M:%S}")
        for p in b["phases"]:
            print(f"   {p['nom']} {p['d']:.2f} s, finit à {(p['t'] - b['debut']).total_seconds():.0f} s")
        print("   s  thread:CPU (ms) [modules, temps propre]")
        for s in sorted(sec):
            parts = []
            for tid, c in sorted(sec[s].items(), key=lambda kv: -sum(kv[1].values()))[:3]:
                n = sum(c.values())
                if n < 150:
                    continue
                top = ", ".join(f"{m.split('.')[0][:12]} {v}" for m, v in c.most_common(3))
                parts.append(f"{tid}:{n} [{top}]")
            print(f"{s:4d}  " + " | ".join(parts))


def zones(cap):
    import zones as zr
    x = Extrait(cap)
    z = zr.read(next(cap.glob("zones-*.bin")))
    f = x.freq
    for b in x.blocs:
        print(f"\n=== chargement {b['rang'] + 1}, {b['debut']:%H:%M:%S}")
        for nom, a, c in phases_de(b):
            lo, hi = x.qpc(a), x.qpc(c)
            by_t = collections.defaultdict(list)
            for t, n, s, e in z.zones:
                if e > lo and s < hi:
                    by_t[t].append((max(s, lo), min(e, hi), n))
            prop = collections.Counter()
            for t, zs in by_t.items():
                zs.sort(key=lambda r: (r[0], -r[1]))
                pile = []
                for s, e, n in zs:
                    while pile and pile[-1][1] <= s:
                        pile.pop()
                    if pile:
                        prop[(t, pile[-1][2])] -= e - s
                    prop[(t, n)] += e - s
                    pile.append((s, e, n))
            print(f"--- {nom} ({(c - a).total_seconds():.0f} s)")
            for (t, n), d in prop.most_common(12):
                if d / f < 0.08:
                    break
                print(f"   {d / f:6.2f} s  thread {t}  {z.names.get(n, n)}")


def phases(cap, threads, voulus=None, noms=None):
    x = Extrait(cap)
    for b in x.blocs:
        if voulus and b["rang"] + 1 not in voulus:
            continue
        print(f"\n=== chargement {b['rang'] + 1}, threads {sorted(threads)}")
        for nom, a, c in phases_de(b):
            if noms and nom not in noms:
                continue
            lo, hi = x.qpc(a), x.qpc(c)
            ss = [s for s in x.samples if s[1] in threads and lo <= s[0] < hi]
            if not ss:
                continue
            propre = collections.Counter(x.mod(s[2])[0] for s in ss)
            incl = collections.defaultdict(collections.Counter)
            appelant_sys = collections.Counter()
            for q, tid, ip in ss:
                fr = x.stacks.get((q, tid))
                if not fr:
                    continue
                vus, appelant = set(), None
                for a_ in fr:  # de la feuille vers la racine
                    m, off = x.mod(a_)
                    if m in SYSTEME or m == "?":
                        continue
                    if appelant is None:
                        appelant = (m, off)
                    vus.add((m, x.gc_func(off) or off) if m == GAMECORE else (m, off))
                for m, k in vus:
                    incl[m][k] += 1
                if x.mod(ip)[0] in SYSTEME and appelant:
                    appelant_sys[(x.mod(ip)[0], appelant)] += 1
            print(f"--- {nom} ({(c - a).total_seconds():.0f} s, {len(ss)} ms de CPU)")
            print("    propre : " + ", ".join(f"{m.split('.')[0]} {n}" for m, n in propre.most_common(7)))
            for m, c_ in sorted(incl.items(), key=lambda kv: -max(kv[1].values())):
                top = [(k, n) for k, n in c_.most_common(10) if n >= 50]
                if not top or m in SYSTEME:
                    continue
                print(f"    inclusif {m.split('.')[0]} :")
                for k, n in top:
                    nom_k = x.funcs.label(k) if isinstance(k, str) else f"+{k:x}"
                    print(f"       {n:6d}  {nom_k[:140]}")
            if appelant_sys:
                print("    système appelé depuis :")
                for (sm, (m, off)), n in appelant_sys.most_common(6):
                    print(f"       {n:6d}  {sm.split('.')[0]} <- {x.label(m, off)[:120]}")


def chaine(cap, rang, de, a, threads):
    x = Extrait(cap)
    b = x.blocs[rang - 1]
    lo, hi = x.qpc(b["debut"] + dt.timedelta(seconds=de)), x.qpc(b["debut"] + dt.timedelta(seconds=a))
    ss = [s for s in x.samples if s[1] in threads and lo <= s[0] < hi]
    incl, feuilles, chemins = collections.Counter(), collections.Counter(), collections.Counter()
    for q, tid, ip in ss:
        fr = x.stacks.get((q, tid))
        if not fr:
            continue
        exe = [x.mod(a_)[1] for a_ in fr if x.mod(a_)[0].startswith("civilizationvi")]
        for o in set(exe):
            incl[o] += 1
        m, o = x.mod(ip)
        feuilles[(m.split(".")[0], o if m.startswith("civ") else 0)] += 1
        chemins[tuple(f"{e:x}" for e in reversed(exe[:6]))] += 1
    print(f"{len(ss)} échantillons")
    print("inclusif exe : " + ", ".join(f"{o:x}:{n}" for o, n in incl.most_common(40)))
    print("feuilles : " + ", ".join(f"{m}+{o:x}:{n}" for (m, o), n in feuilles.most_common(20)))
    print("chemins (6 dernières frames de l'exe, racine > feuille) :")
    for p, n in chemins.most_common(12):
        print(f"  {n:6d}  {' > '.join(p)}")


def appelants(cap, cible):
    x = Extrait(cap)
    par_bloc, par_thread, chaines = collections.Counter(), collections.Counter(), collections.Counter()
    for q, tid, ip in x.samples:
        fr = x.stacks.get((q, tid))
        if not fr:
            continue
        exe = [x.mod(a)[1] for a in fr if x.mod(a)[0].startswith("civilizationvi")]
        if cible not in exe:
            continue
        k = exe.index(cible)
        chaines[tuple(f"{e:x}" for e in exe[k:k + 9])] += 1
        par_bloc[next((i + 1 for i, (a, b) in enumerate(x.wins) if a <= q < b), None)] += 1
        par_thread[tid] += 1
    print("ms par chargement :", dict(par_bloc))
    print("ms par thread :", par_thread.most_common(8))
    for c, n in chaines.most_common(10):
        print(f"{n:6d}  {' < '.join(c)}")


def rythme(cap):
    """Par phase : images du thread principal, temps passé dans la présentation, étapes de
    WorldView_PostLoadUpdate, et CPU du jeu hors thread réseau de Steam (en cœurs occupés). Une phase
    qui compte beaucoup d'images à faible CPU attend le rythme des images (VSync), pas un calcul."""
    import zones as zr
    x = Extrait(cap)
    z = zr.read(next(cap.glob("zones-*.bin")))
    f = x.freq
    ids = {n: i for i, n in z.names.items()}
    frame, present, postload = (ids.get(n) for n in ("Civ6App_UpdateFrame", "Graphics_SubmissionIncFrame",
                                                     "WorldView_PostLoadUpdate"))
    par_t = collections.Counter(t for t, n, s, e in z.zones if n == frame)
    principal = par_t.most_common(1)[0][0] if par_t else None
    # Thread réseau de Steam : un cinquième au moins de ses piles passent par steamclient64 (les autres,
    # dans le noyau, sont souvent sans pile).
    steam, tous = collections.Counter(), collections.Counter()
    for q, tid, ip in x.samples:
        tous[tid] += 1
        if any(x.mod(a)[0] == "steamclient64.dll" for a in x.stacks.get((q, tid), ())):
            steam[tid] += 1
    reseau = {t for t, n in tous.items() if steam[t] * 5 >= n and par_t[t] == 0}
    qs = sorted((q, tid) for q, tid, ip in x.samples if tid not in reseau)
    cles = [q for q, _ in qs]
    print(f"thread principal {principal}, threads réseau exclus du CPU : {sorted(reseau)}")
    for b in x.blocs:
        print(f"\n=== chargement {b['rang'] + 1}, {b['debut']:%H:%M:%S}")
        print(f"   {'phase':32} {'durée':>6} {'images':>7} {'img/s':>6} {'présent.':>9} {'postload':>9} {'CPU':>6}")
        for nom, a, c in phases_de(b):
            lo, hi = x.qpc(a), x.qpc(c)
            img = sum(1 for t, n, s, e in z.zones if t == principal and n == frame and lo <= s < hi)
            pres = sum(min(e, hi) - max(s, lo) for t, n, s, e in z.zones
                       if t == principal and n == present and e > lo and s < hi) / f
            pl = sum(1 for t, n, s, e in z.zones if n == postload and lo <= s < hi)
            d = (c - a).total_seconds()
            cpu = (bisect.bisect_left(cles, hi) - bisect.bisect_left(cles, lo)) / 1000 / d
            print(f"   {nom:32} {d:5.1f}s {img:7d} {img / d:6.0f} {pres:8.2f}s {pl:9d} {cpu:6.2f}")


def ints(s):
    return {int(v) for v in s.split(",")} if s else None


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd, cap = argv[0], pathlib.Path(argv[1])
    sys.stdout.reconfigure(encoding="utf-8")
    if cmd == "extrait":
        extrait(cap)
    elif cmd == "chrono":
        chrono(cap)
    elif cmd == "zones":
        zones(cap)
    elif cmd == "phases":
        phases(cap, ints(argv[2]), ints(argv[3]) if len(argv) > 3 else None,
               set(argv[4].split(",")) if len(argv) > 4 else None)
    elif cmd == "chaine":
        chaine(cap, int(argv[2]), float(argv[3]), float(argv[4]), ints(argv[5]))
    elif cmd == "appelants":
        appelants(cap, int(argv[2], 16))
    elif cmd == "rythme":
        rythme(cap)
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
