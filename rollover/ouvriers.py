"""Temps des ouvriers du pool de tâches pendant une fenêtre de la trace, par catégorie : pause
(feuille dans receive_or_steal_task, exe e50da0..e51200), noyau sous receive_or_steal_task (cession
ou sommeil), reste (vraies tâches). Lit trace.pkl (créé par analyze.py).
    python -I rollover/ouvriers.py <capture> <début hh:mm:ss.fff> <fin> [<capture> <début> <fin>...]"""
import collections
import datetime as dt
import pathlib
import pickle
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import etl  # noqa: E402,F401  (classes du pickle)

RST = (0xe50da0, 0xe51200)       # receive_or_steal_task, DX12
args = sys.argv[1:]
for d, debut, fin in zip(args[0::3], args[1::3], args[2::3]):
    tr = pickle.load(open(pathlib.Path(d) / "trace.pkl", "rb"))
    # Heures locales du rapport : décalage de la machine qui analyse, celle de la capture.
    utc = dt.datetime(1601, 1, 1, tzinfo=dt.timezone.utc) + dt.timedelta(microseconds=tr.start_filetime / 10)
    s0 = utc.astimezone().replace(tzinfo=None)
    def q(hms):
        t = dt.datetime.combine(s0.date(), dt.time.fromisoformat(hms))
        return tr.first_qpc + (t - s0).total_seconds() * tr.perf_freq
    Q0, Q1 = q(debut), q(fin)
    tr.samples = [x for x in tr.samples if Q0 <= x[0] < Q1]
    tr.cswitches = [x for x in tr.cswitches if Q0 <= x[0] < Q1]
    civ = [i for i in tr.images if i[3].lower().endswith("civilizationvi_dx12.exe")]
    pid, base, size, _ = civ[0]
    mods = sorted((b, b + s, n.rsplit("\\", 1)[-1].lower()) for p, b, s, n in tr.images if p == pid)
    def mod(a):
        for b, e, n in mods:
            if b <= a < e:
                return n
        return "noyau" if a >= 0xFFFF800000000000 else "?"
    # ouvriers : threads dont la pile contient exe+e5278d (point d'entrée des ouvriers)
    par = collections.defaultdict(collections.Counter)
    ouvriers = set()
    for (q, tid, ip) in tr.samples:
        if tr.thread_pid.get(tid) != pid:
            continue
        st = tr.stacks.get((q, tid))
        if not st:
            continue
        rel = [a - base for a in st if base <= a < base + size]
        if not any(0xe52700 <= r < 0xe52800 for r in rel):
            continue
        ouvriers.add(tid)
        leaf = st[0]
        if base <= leaf < base + size and RST[0] <= leaf - base < RST[1]:
            cat = "pause"
        else:
            first_exe = rel[0] if rel else None
            if first_exe is not None and RST[0] <= first_exe < RST[1] and not (base <= leaf < base + size):
                cat = "cession/sommeil (" + mod(leaf) + ")"
            else:
                cat = "tâche"
        par[tid][cat] += 1
    tot = collections.Counter()
    for c in par.values():
        tot.update(c)
    n = sum(tot.values())
    print(f"== {pathlib.Path(d).name}: {len(ouvriers)} ouvriers, {n} ms")
    for k, v in tot.most_common():
        print(f"   {k:40s} {v:7d} ms  {100*v/n:5.1f} %")
    # changements de contexte des ouvriers : sorties
    sorties = collections.Counter(c[4] if False else c[3] for c in tr.cswitches if c[2] in ouvriers)
    print("   sorties (raison):", sorties.most_common(5), " total", sum(sorties.values()))

    feuilles = collections.Counter()
    for (q, tid, ip) in tr.samples:
        if tid in ouvriers:
            m = mod(ip)
            feuilles[f"exe+{(ip-base)&~0x3f:x}" if m.startswith("civilizationvi") else m] += 1
    print("   feuilles:", feuilles.most_common(14))

    # Réveils des ouvriers (ReadyThread vers un ouvrier), et temps noyau des autres threads du jeu.
    Rq = [r for r in tr.readies if Q0 <= r[0] < Q1]
    print("   réveils d'ouvriers :", sum(1 for r in Rq if r[2] in ouvriers),
          " dont par un non-ouvrier :", sum(1 for r in Rq if r[2] in ouvriers and r[1] is not None and r[1] not in ouvriers))
    autres = collections.defaultdict(collections.Counter)
    for (q, tid, ip) in tr.samples:
        if tr.thread_pid.get(tid) == pid and tid not in ouvriers:
            autres[tid]["noyau" if ip >= 0xFFFF800000000000 else "user"] += 1
    for tid, c in sorted(autres.items(), key=lambda x: -sum(x[1].values()))[:3]:
        print(f"   thread {tid}: {sum(c.values())} ms, noyau {c['noyau']} ms")
