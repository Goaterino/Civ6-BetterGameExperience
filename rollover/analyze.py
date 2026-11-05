"""Ventilation du temps d'un passage de tour de Civ6.

Deux sources :
- les journaux du jeu (net_message_debug.log, Profile.csv), à la seconde près ;
- une trace WPR (trace.etl, voir capture.ps1), échantillonnée à la milliseconde avec les piles,
  lue directement par etl.py puis gardée en cache (trace.pkl) ; elle porte aussi les présentations
  DXGI (temps d'image) ;
- les relevés du GPU pris pendant la capture (gpu_moteurs.csv, gpu_nvidia.txt).

Sans passage de tour dans la capture (scénario à durée fixe, capture.ps1 -Duration), le rapport
couvre toute la capture.

Usage :
    python -I rollover/analyze.py <dossier de capture ou dossier Logs du jeu> [--tout]

Écrit rapport.md dans le dossier analysé (ou dans --out).
"""
import argparse
import bisect
import collections
import csv
import datetime as dt
import json
import pathlib
import pickle
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import etl as etl_reader  # noqa: E402
import zones as zones_reader  # noqa: E402
import zones_long  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parents[1]
FUNCTIONS_TSV = REPO / "docs" / "gamecore_xp2" / "functions.tsv"
GAMECORE = "gamecore_xp2_finalrelease.dll"
GAMECORE_ORIG = "gamecore_xp2_finalrelease_orig.dll"
PROXY = "proxy rollover"
GAMECORE_BASE = 0x180000000  # base Ghidra des adresses de functions.tsv
LOCAL_PLAYER = 0


# --- journaux du jeu ------------------------------------------------------------------------

LINE = re.compile(r"\[(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)\]\t,?\s*(.*)")


def read_log(path):
    for line in path.open(encoding="utf-8", errors="replace"):
        m = LINE.match(line.rstrip("\n"))
        if m:
            yield dt.datetime.strptime(m[1], "%Y-%m-%d %H:%M:%S"), m[2]


def turn_transitions(net_log):
    """Un passage par tour : fin du tour précédent, début du tour, activation de chaque joueur
    jusqu'au joueur local. Les horodatages du jeu sont à la seconde. En multijoueur, le joueur local
    est celui dont la machine envoie PlayerTurnComplete, et le passage commence quand le dernier
    joueur humain finit son tour."""
    global LOCAL_PLAYER
    for _, msg in read_log(net_log):
        m = re.match(r"GameCore SEND: PlayerTurnComplete: ePlayer=(\d+)", msg)
        if m:
            LOCAL_PLAYER = int(m[1])
            break
    out, cur = [], None
    end = click = None
    for t, msg in read_log(net_log):
        if msg.endswith("set TurnActive 0, TurnActiveCount: 0"):
            click = t  # le dernier joueur finit son tour : le passage commence
        elif msg.startswith("Game Turn Ended:"):
            end = t
        elif msg.startswith("Game Turn: "):
            cur = {"tour": int(msg.split()[-1]), "clic": click, "fin": end, "debut": t, "joueurs": []}
        elif cur:
            m = re.match(r"Player (\d+) set TurnActive 1", msg)
            if not m:
                continue
            player = int(m[1])
            cur["joueurs"].append((player, t))
            if player == LOCAL_PLAYER:
                cur["main"] = t
                out.append(cur)
                cur, end, click = None, None, None
    return out


def log_report(logs, lines):
    net = logs / "net_message_debug.log"
    if not net.exists():
        return []
    trans = turn_transitions(net)
    lines.append("## Passages de tour (net_message_debug.log, à la seconde)\n")
    lines.append("| tour | heure | fin → début | début → main | total | IA activées |")
    lines.append("|---:|---|---:|---:|---:|---:|")
    for tr in trans:
        fin = (tr["debut"] - tr["fin"]).total_seconds() if tr["fin"] else None
        total = (tr["main"] - tr["fin"]).total_seconds() if tr["fin"] else None
        lines.append(
            f"| {tr['tour']} | {tr['main']:%H:%M:%S} | {fmt(fin)} | "
            f"{(tr['main'] - tr['debut']).total_seconds():.0f} | {fmt(total)} | {len(tr['joueurs']) - 1} |")

    # Temps moyen entre deux activations : l'activation du joueur suivant marque la fin du tour
    # de l'IA précédente. Moyenné sur beaucoup de tours, l'arrondi à la seconde s'efface.
    per_player = collections.defaultdict(list)
    for tr in trans:
        prev = tr["debut"]
        for player, t in tr["joueurs"]:
            per_player[player].append((t - prev).total_seconds())
            prev = t
    if per_player:
        lines.append("\n## Délai moyen avant l'activation de chaque joueur\n")
        lines.append("Le délai avant le joueur N couvre le tour de l'IA précédente.\n")
        lines.append("| joueur | tours | moyenne (s) |")
        lines.append("|---:|---:|---:|")
        for player, v in sorted(per_player.items(), key=lambda kv: (kv[0] == LOCAL_PLAYER, kv[0])):
            lines.append(f"| {player} | {len(v)} | {sum(v) / len(v):.2f} |")

    prof = logs / "Profile.csv"
    if prof.exists():
        saves = [float(m[1]) for _, msg in read_log(prof)
                 if (m := re.match(r"DoGameStateSaveRequest, ([\d.]+) ms", msg))]
        stalls = [float(m[1]) for _, msg in read_log(prof)
                  if (m := re.search(r"stalled on GC lock for ([\d.]+) ms", msg))]
        lines.append("\n## Thread principal (Profile.csv)\n")
        if saves:
            lines.append(f"- Sauvegarde auto (`DoGameStateSaveRequest`, bloque le thread principal) : "
                         f"{len(saves)} fois, moyenne {sum(saves) / len(saves):.0f} ms, max {max(saves):.0f} ms.")
        lines.append(f"- Blocages sur le verrou GameCore : {len(stalls)}"
                     + (f", total {sum(stalls):.0f} ms." if stalls else "."))
    lines.append("")
    return trans


def fmt(x):
    return "" if x is None else f"{x:.0f}"


# --- trace WPR ------------------------------------------------------------------------------

def load_trace(etl):
    """Lit la trace une fois, puis la garde en cache (trace.pkl) : la relire prend plusieurs minutes."""
    cache = etl.with_suffix(".pkl")
    if cache.exists() and cache.stat().st_mtime >= etl.stat().st_mtime:
        with cache.open("rb") as f:
            tr = pickle.load(f)
        if hasattr(tr, "cswitches"):  # cache antérieur au décodage des changements de contexte : on relit
            if not hasattr(tr, "presents"):  # antérieur au profil Frames : ces captures n'en ont pas
                tr.presents = []
            if not hasattr(tr, "packets"):  # antérieur au trafic réseau
                tr.packets = []
            return tr
    tr = etl_reader.read(etl)
    with cache.open("wb") as f:
        pickle.dump(tr, f, protocol=pickle.HIGHEST_PROTOCOL)
    return tr


class Functions:
    def __init__(self, path):
        self.starts, self.rows = [], []
        with path.open(encoding="utf-8") as f:
            for row in csv.DictReader(f, delimiter="\t"):
                self.starts.append(int(row["adresse"], 16))
                self.rows.append(row)
        self.by_addr = {r["adresse"]: r for r in self.rows}

    def find(self, addr):
        i = bisect.bisect_right(self.starts, addr) - 1
        if i >= 0 and addr < self.starts[i] + int(self.rows[i]["octets"]):
            return self.rows[i]
        return None

    def label(self, a):
        r = self.by_addr[a]
        if r["fichier"]:
            where = r["fichier"].rsplit("\\", 1)[-1]
        else:
            where = "? " + r["bornes"].replace("\\", "/").rsplit("/", 1)[-1][:40]
        hint = r["indices_release"].split(" | ")[0][:60] if r["indices_release"] else ""
        lua = r["noms_lua"].split(",")[0] if r["noms_lua"] else ""
        return " ".join(x for x in (f"`{a}`", where, hint and f"« {hint} »", lua and f"lua:{lua}") if x)


def passages_report(game, trans, module, ms, freq, lines):
    """Durée de chaque passage à la milliseconde, d'après l'activité du thread GameCore : il est
    presque inactif quand le joueur a la main. Le passage va du premier échantillon qui suit la fin
    du tour du joueur local (journaux à la seconde) au dernier avant une seconde d'inactivité, une
    fois le joueur local réactivé, ou avant la seconde où il finit le tour suivant : s'il clique dès
    qu'il a la main, la fin du passage (mise en place de son tour) est coupée, à moins d'une seconde près."""
    by_gc = collections.Counter(s[1] for s in game if module(s[2]) == GAMECORE)
    if not by_gc:
        return []
    gc_tid = by_gc.most_common(1)[0][0]
    gct = [s for s in game if s[1] == gc_tid]
    idle = freq  # une seconde, en unités QPC
    rows = []
    starts = [tr["clic"] or tr["fin"] for tr in trans]
    for i, tr in enumerate(trans):
        lo = starts[i]
        # Un passage commencé avant la capture n'y est pas : sans ce test, le dernier passage des
        # journaux s'étendrait sur toute une capture prise ensuite, pendant le tour du joueur.
        if not lo or lo < game[0][3] - dt.timedelta(seconds=1):
            continue
        nxt = next((t for t in starts[i + 1:] if t), None)
        g = [s for s in gct if s[3] >= lo and (nxt is None or s[3] < nxt)]
        if not g:
            continue
        end, cut = None, nxt is None
        for x, y in zip(g, g[1:]):
            if y[0] - x[0] >= idle and x[3] >= tr["main"]:
                end, cut = x, False
                break
        if end is None:
            end = g[-1]
            if nxt is None:
                # Plus d'activité jusqu'à la fin de la capture : tronqué si elle s'arrête moins d'une seconde après.
                cut = game[-1][0] - g[-1][0] < idle
        a, b = g[0][0], end[0]
        cpu = sum(1 for s in game if a <= s[0] <= b)
        n_gc = sum(1 for s in gct if a <= s[0] <= b)
        rows.append((tr["tour"], g[0][3], (b - a) / freq * 1000, n_gc * ms, cpu * ms / 1000, cut, a, b))
    if not rows:
        return []
    lines.append(f"### Passages mesurés dans la trace (thread GameCore {gc_tid}, à la milliseconde)\n")
    lines.append("| tour | début | durée (ms) | CPU thread GameCore (ms) | CPU processus (s) |")
    lines.append("|---:|---|---:|---:|---:|")
    for tour, t, d, g, c, cut, _, _ in rows:
        note = " (tronqué : capture arrêtée avant la fin)" if cut else ""
        lines.append(f"| {tour} | {t:%H:%M:%S.%f}"[:-3] + f" | {d:.0f}{note} | {g:.0f} | {c:.1f} |")
    if len(rows) > 1:
        lines.append(f"| total | | {sum(r[2] for r in rows):.0f} | {sum(r[3] for r in rows):.0f} | "
                     f"{sum(r[4] for r in rows):.1f} |")
    lines.append("")
    return [(r[6], r[7]) for r in rows]


def pct(v, q):
    """Centile q (0 à 100) d'une liste triée."""
    return v[min(len(v) - 1, int(q / 100 * len(v)))] if v else None


def phase(t, passages):
    return "passage de tour" if any(a <= t <= b for a, b in passages) else "hors passage"


def frames_report(tr, pid, lo, hi, passages, wall, lines):
    """Temps d'image d'après les présentations DXGI du jeu, entre les QPC lo et hi, pendant les
    passages de tour et en dehors. Le temps bloqué dans Present (début → fin sur le même thread)
    s'allonge quand le CPU attend le GPU ou la synchro verticale."""
    if not tr.presents:
        lines.append("### Images\n\nAucune présentation DXGI dans la trace (capture sans le profil Frames).\n")
        return
    starts, blocked, open_ = [], [], {}
    for ts, _, tid, start, flags, sync in sorted(e for e in tr.presents if e[1] == pid):
        if start:
            if flags & 1:  # DXGI_PRESENT_TEST : aucune image n'est présentée
                continue
            open_[tid] = ts
            if lo <= ts <= hi:
                starts.append((ts, sync))
        elif tid in open_:
            t0 = open_.pop(tid)
            if lo <= t0 <= hi:
                blocked.append((t0, (ts - t0) / tr.perf_freq * 1000))
    if len(starts) < 2:
        lines.append("### Images\n\nMoins de deux présentations du jeu dans la fenêtre.\n")
        return
    seg, seg_blocked = collections.defaultdict(list), collections.defaultdict(list)
    for (a, _), (b, _) in zip(starts, starts[1:]):
        seg[phase(b, passages)].append((b - a) / tr.perf_freq * 1000)
    for t0, d in blocked:
        seg_blocked[phase(t0, passages)].append(d)
    syncs = collections.Counter(sync for _, sync in starts)
    lines.append("### Images (présentations DXGI)\n")
    lines.append(f"- {len(starts)} images de {wall(starts[0][0]):%H:%M:%S} à {wall(starts[-1][0]):%H:%M:%S} ; "
                 "intervalle de synchro : " + ", ".join(f"{k} ({v})" for k, v in syncs.most_common())
                 + " (0 : sans synchro verticale).\n")
    lines.append("| période | images | durée (s) | images/s | p50 (ms) | p95 (ms) | p99 (ms) | max (ms) "
                 "| > 33 ms | > 100 ms | bloqué dans Present, moyenne / p95 (ms) |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    for name in ("hors passage", "passage de tour"):
        v = sorted(seg.get(name, []))
        if not v:
            continue
        total = sum(v) / 1000
        bl = sorted(seg_blocked.get(name, []))
        bl_txt = f"{sum(bl) / len(bl):.1f} / {pct(bl, 95):.1f}" if bl else ""
        lines.append(f"| {name} | {len(v)} | {total:.1f} | {len(v) / total:.0f} | {pct(v, 50):.1f} | "
                     f"{pct(v, 95):.1f} | {pct(v, 99):.1f} | {v[-1]:.0f} | {sum(x > 33.3 for x in v)} | "
                     f"{sum(x > 100 for x in v)} | {bl_txt} |")
    lines.append("")


def read_gpu(folder, pid):
    """Relevés du GPU à la seconde : [(heure locale, {mesure: valeur})]. Deux sources, si présentes :
    les compteurs « GPU Engine » (utilisation par type de moteur, en %, le jeu à part) et nvidia-smi
    dmon (puissance, utilisation, fréquences de toute la carte)."""
    out = collections.defaultdict(dict)
    eng = folder / "gpu_moteurs.csv"
    if eng.exists() and pid:
        with eng.open(encoding="utf-8", errors="replace", newline="") as f:
            rows = list(csv.reader(f))
        cols = []
        for i, h in enumerate(rows[0][1:] if rows else [], 1):
            m = re.search(r"pid_(\d+)_.*engtype_([^)]+)\)", h)
            if m:
                cols.append((i, int(m[1]) == pid, m[2]))
        for r in rows[1:]:
            try:
                t = dt.datetime.strptime(r[0], "%m/%d/%Y %H:%M:%S.%f").replace(microsecond=0)
            except (ValueError, IndexError):
                continue
            for i, mine, kind in cols:
                try:
                    v = float(r[i])
                except (ValueError, IndexError):
                    continue
                if not 0 <= v <= 100:  # typeperf rend parfois une valeur aberrante (1e10)
                    continue
                if kind in ("3D", "Copy", "Compute", "VideoDecode"):
                    key = f"{'jeu' if mine else 'autres'} {kind} (%)"
                    out[t][key] = out[t].get(key, 0.0) + v
    nv = folder / "gpu_nvidia.txt"
    if nv.exists():
        names = None
        for line in nv.open(encoding="utf-8", errors="replace"):
            parts = line.lstrip("#").split()
            if line.startswith("#"):
                if names is None and len(parts) > 2:
                    names = parts  # Date Time gpu pwr … ; la ligne suivante donne les unités
                continue
            if not names or len(parts) != len(names):
                continue
            row = dict(zip(names, parts))
            try:
                t = dt.datetime.strptime(row["Date"] + row["Time"], "%Y%m%d%H:%M:%S")
            except (KeyError, ValueError):
                continue
            for k, label in (("pwr", "puissance (W)"), ("sm", "utilisation carte (%)"),
                             ("pclk", "fréquence GPU (MHz)"), ("mclk", "fréquence mémoire (MHz)"),
                             ("gtemp", "température (°C)")):
                if row.get(k, "-") not in ("-", "N/A"):
                    out[t]["nvidia " + label] = float(row[k])
    return sorted(out.items())


def gpu_report(folder, pid, window, passages_wall, lines):
    samples = [(t, v) for t, v in read_gpu(folder, pid) if not window or window[0] <= t < window[1]]
    if not samples:
        return
    seg = collections.defaultdict(lambda: collections.defaultdict(list))
    count = collections.Counter()
    for t, v in samples:
        name = phase(t, passages_wall)
        count[name] += 1
        for k, x in v.items():
            seg[name][k].append(x)
    keys = sorted({k for _, v in samples for k in v}, key=lambda k: (not k.startswith("jeu"), k))
    names = [n for n in ("hors passage", "passage de tour") if n in seg]
    lines.append("### GPU (relevés à la seconde)\n")
    lines.append("Moyenne / p95 sur la période. Les compteurs « jeu » ne portent que sur le processus du "
                 "jeu, « autres » sur tous les autres (bureau, navigateur…).\n")
    lines.append("| mesure | " + " | ".join(f"{n} ({count[n]} s)" for n in names) + " |")
    lines.append("|---|" + "---:|" * len(names))
    for k in keys:
        cells = []
        for n in names:
            v = sorted(seg[n].get(k, []))
            cells.append(f"{sum(v) / len(v):.1f} / {pct(v, 95):.1f}" if v else "")
        lines.append(f"| {k} | " + " | ".join(cells) + " |")
    lines.append("")


# Raisons de sortie du processeur (KWAIT_REASON) des événements CSwitch.
WAIT_REASONS = {0: "Executive", 6: "attente (UserRequest)", 13: "attente (WrUserRequest)", 30: "fin de quantum",
                31: "interruption", 32: "préemption", 33: "cession (yield)", 34: "WrFastMutex",
                35: "WrGuardedMutex", 37: "verrou ou variable de condition (WrAlertByThreadId)"}


def net_report(tr, pid, lo, hi, lines, known=None):
    """Trafic réseau entre les QPC lo et hi, par processus : le jeu, steam.exe (qui porte les paquets
    de l'ancienne API P2P de Steam) et les autres processus bavards. L'écart entre deux paquets reçus
    dit à quelle cadence la boucle P2P du jeu aurait réellement quelque chose à lire."""
    packets = [p for p in tr.packets if lo <= p[0] <= hi]
    if not packets:
        return
    span = (hi - lo) / tr.perf_freq
    names = dict(known or {})
    for p, _, _, n in tr.images:
        if n.endswith(".exe"):
            names.setdefault(p, n)
    by_pid = collections.defaultdict(list)
    for p in packets:
        by_pid[p[1]].append(p)
    shown = [p for p in by_pid if p == pid or names.get(p) == "steam.exe"]
    shown += [p for p, v in sorted(by_pid.items(), key=lambda kv: -len(kv[1]))
              if p not in shown and len(v) / span >= 10][:4]
    lines.append("## Réseau\n")
    lines.append(f"{len(packets)} paquets en {span:.0f} s, tous processus.\n")
    lines.append("| processus | proto | reçus /s | envoyés /s | ko/s reçus | ko/s envoyés | écart entre reçus p50 / p95 / max (ms) |")
    lines.append("|---|---|---:|---:|---:|---:|---|")
    for p in shown:
        for proto in ("udp", "tcp"):
            v = [x for x in by_pid.get(p, []) if x[2] == proto]
            if not v:
                continue
            rx = [x for x in v if x[3]]
            tx = [x for x in v if not x[3]]
            gaps = sorted((b[0] - a[0]) * 1000 / tr.perf_freq for a, b in zip(rx, rx[1:]))
            gap = (f"{pct(gaps, 50):.1f} / {pct(gaps, 95):.1f} / {gaps[-1]:.0f}" if gaps else "")
            lines.append(f"| {names.get(p, '?')} ({p}) | {proto} | {len(rx) / span:.1f} | {len(tx) / span:.1f} | "
                         f"{sum(x[4] for x in rx) / span / 1024:.1f} | {sum(x[4] for x in tx) / span / 1024:.1f} | {gap} |")
    lines.append("")


def threads_report(tr, ours, mods, module, span, ms, main_gc, lines):
    """Une ligne par thread : CPU, changements de contexte, et de quoi dire s'il calcule ou s'il
    tourne à vide. Une attente active a peu de changements de contexte et des échantillons concentrés
    sur quelques adresses ; une attente par cession (Sleep(0), SwitchToThread) beaucoup de changements,
    surtout des sorties par cession, et une grosse part de noyau. Le point d'entrée regroupe les
    threads d'un même pool."""
    starts = [m[0] for m in mods]

    def where(ip):
        if ip >= 0xFFFF800000000000:
            return "noyau"
        i = bisect.bisect_right(starts, ip) - 1
        if i >= 0 and ip < mods[i][0] + mods[i][1]:
            return f"{mods[i][2]}+{ip - mods[i][0]:#x}"
        return f"{ip:#x}"

    lo, hi = min(s[0] for s in ours), max(s[0] for s in ours)
    switches = collections.Counter(c[1] for c in tr.cswitches if lo <= c[0] <= hi)
    outs = collections.defaultdict(collections.Counter)  # thread sortant -> raison
    for c in tr.cswitches:
        if lo <= c[0] <= hi:
            outs[c[2]][c[3]] += 1
    by_thread = collections.defaultdict(list)
    for s in ours:
        by_thread[s[1]].append(s)
    plumbing = {"noyau", "ntdll.dll", "kernel32.dll", "kernelbase.dll", "ucrtbase.dll", "msvcrt.dll"}
    lines.append("### Threads\n")
    lines.append("Concentration : part des échantillons sur les 4 adresses les plus fréquentes du thread. "
                 "Tranche : CPU moyen par mise sur le processeur. Sortie : raison la plus fréquente de "
                 "sortie du processeur.\n")
    lines.append("| thread | nom | CPU (ms) | occupation | dont GameCore (ms) | noyau | mises sur le CPU "
                 "| tranche (ms) | sortie | concentration | adresse la plus fréquente | point d'entrée |")
    lines.append("|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---|---|")
    for tid, ss in sorted(by_thread.items(), key=lambda kv: -len(kv[1]))[:16]:
        n = len(ss)
        ips = collections.Counter(s[2] for s in ss)
        conc = sum(c for _, c in ips.most_common(4)) / n
        kernel = sum(1 for s in ss if s[2] >= 0xFFFF800000000000) / n
        gc = sum(1 for s in ss if module(s[2]) == GAMECORE)
        entries = collections.Counter()
        for s in ss:
            frames = tr.stacks.get((s[0], s[1]))
            if frames:
                e = next((w for w in map(where, reversed(frames)) if w.split("+")[0] not in plumbing), None)
                if e:
                    entries[e] += 1
        sw = switches.get(tid, 0)
        entry = f"`{entries.most_common(1)[0][0]}`" if entries else ""
        out = ""
        if outs[tid]:
            reason, k = outs[tid].most_common(1)[0]
            out = f"{WAIT_REASONS.get(reason, reason)} {100 * k / sum(outs[tid].values()):.0f} %"
        lines.append(f"| {tid}{' (GameCore)' if tid == main_gc else ''} | {tr.thread_name.get(tid, '')} | "
                     f"{n * ms:.0f} | {100 * n * ms / 1000 / span:.0f} % | {gc * ms:.0f} | {100 * kernel:.0f} % | "
                     f"{sw} | {f'{n * ms / sw:.2f}' if sw else ''} | {out} | {100 * conc:.0f} % | "
                     f"`{where(ips.most_common(1)[0][0])}` | {entry} |")


def zones_report(folder, pid, ours, ms, main_gc, lines):
    """Zones du profileur de l'exe (proxy de démarrage, `profil_zones=on`), copiées par capture.ps1,
    sur la fenêtre et avec les échantillons de la trace."""
    files = sorted(folder.glob("zones-*.bin"))
    if not files:
        return
    z = zones_reader.read(files[-1])
    if z.pid != pid:
        lines.append(f"### Zones du profileur de l'exe\n\n{files[-1].name} : pid {z.pid}, la trace est celle "
                     f"du pid {pid}. Ignoré.\n")
        return
    lo, hi = min(s[0] for s in ours), max(s[0] for s in ours)
    zones_reader.report(z, lines, lo, hi, [(s[0], s[1]) for s in ours], ms,
                        {main_gc: f"{main_gc} (GameCore)"} if main_gc else None)


def capture_meta(folder):
    meta = folder / "capture.json"
    return json.loads(meta.read_text(encoding="utf-8-sig")) if meta.exists() else {}


def light_report(tr, folder, window, trans, wall, lines):
    """Capture légère (capture.ps1 -Light) : ni échantillons CPU ni noyau, seulement les images, le
    trafic réseau, le GPU et les zones, sur toute la capture. Le pid et celui de steam.exe viennent
    de capture.json ; les passages de tour, des journaux du jeu."""
    m = capture_meta(folder)
    pid = m.get("pid")
    if not pid or not window:
        lines.append("Capture légère sans capture.json complet : rien à analyser.\n")
        return
    start_utc = wall(tr.first_qpc)

    def qpc(t):
        return tr.first_qpc + (t - start_utc).total_seconds() * tr.perf_freq

    lo, hi = qpc(window[0]), qpc(window[1])
    span = (window[1] - window[0]).total_seconds()
    passages = [(qpc(t["clic"] or t["fin"]), qpc(t["main"] + dt.timedelta(seconds=1)))
                for t in trans if (t["clic"] or t["fin"]) and window[0] <= t["main"] <= window[1]]
    lines.append("## Trace WPR légère\n")
    lines.append(f"- Fenêtre : {window[0]:%H:%M:%S} → {window[1]:%H:%M:%S} ({span / 60:.0f} min), "
                 f"{m.get('exe', '?')} (pid {pid}), {len(passages)} passages de tour.\n")
    frames_report(tr, pid, lo, hi, passages, wall, lines)
    gpu_report(folder, pid, window, [(wall(a), wall(b)) for a, b in passages], lines)
    known = {pid: f"{m.get('exe', 'jeu')}.exe"}
    if m.get("steam_pid"):
        known[m["steam_pid"]] = "steam.exe"
    net_report(tr, pid, lo, hi, lines, known)
    files = sorted(folder.glob("zones-*.bin"))
    long_pkl = folder / "zones_long.pkl"  # zones_long.py : fichier de toute une partie, trop gros pour zones.read
    if long_pkl.exists():
        with long_pkl.open("rb") as f:
            zl = pickle.load(f)
        if zl.pid == pid:
            zones_long.report(zl, lines, passages, wall)
    elif files:
        z = zones_reader.read(files[-1])
        if z.pid == pid:
            zones_reader.report(z, lines, lo, hi)
            if passages:
                lines.append("#### Les trois derniers passages de tour\n")
                for a, b in passages[-3:]:
                    lines.append(f"Passage {wall(a):%H:%M:%S} → {wall(b):%H:%M:%S} :\n")
                    zones_reader.report(z, lines, a, b, top=12)


def capture_window(folder):
    """Début et fin de la capture (capture.json, heure locale), ou None."""
    meta = folder / "capture.json"
    if not meta.exists():
        return None
    m = json.loads(meta.read_text(encoding="utf-8-sig"))
    return tuple(dt.datetime.fromisoformat(m[k]).astimezone().replace(tzinfo=None) for k in ("start", "stop"))


def trace_report(etl, lines, window, trans, folder=None):
    tr = load_trace(etl)
    ms = (tr.interval_100ns or 10000) / 10000
    start_utc = dt.datetime(1601, 1, 1, tzinfo=dt.timezone.utc) + dt.timedelta(microseconds=tr.start_filetime / 10)

    def wall(qpc):
        t = start_utc + dt.timedelta(seconds=(qpc - tr.first_qpc) / tr.perf_freq)
        return t.astimezone().replace(tzinfo=None)

    if folder and capture_meta(folder).get("light"):
        light_report(tr, folder, window or capture_window(folder), trans, wall, lines)
        return
    civ = [i for i in tr.images if i[3].startswith("civilizationvi")]
    if not civ:
        lines.append("Aucun processus CivilizationVI dans la trace.\n")
        return
    pid = civ[0][0]
    mods = sorted({(b, s, n) for p, b, s, n in tr.images if p == pid})
    # Avec le proxy installé, l'originale est chargée sous le nom _orig et le proxy prend le sien.
    if any(n == GAMECORE_ORIG for _, _, n in mods):
        rename = {GAMECORE: PROXY, GAMECORE_ORIG: GAMECORE}
        mods = [(b, s, rename.get(n, n)) for b, s, n in mods]
    mod_starts = [m[0] for m in mods]

    def module(ip):
        if ip >= 0xFFFF800000000000:
            return "noyau"
        i = bisect.bisect_right(mod_starts, ip) - 1
        if i >= 0 and ip < mods[i][0] + mods[i][1]:
            return mods[i][2]
        return "?"

    funcs = Functions(FUNCTIONS_TSV)
    gc = next((m for m in mods if m[2] == GAMECORE), None)

    def gc_func(ip):
        if gc and gc[0] <= ip < gc[0] + gc[1]:
            r = funcs.find(ip - gc[0] + GAMECORE_BASE)
            return r["adresse"] if r else None
        return None

    game = [(qpc, tid, ip, wall(qpc)) for qpc, tid, ip in tr.samples if tr.thread_pid.get(tid) == pid]
    ours = [s for s in game if not window or window[0] <= s[3] < window[1]]
    if not ours and window:
        # Le dernier passage des journaux précède la capture (scénario sans passage) : toute la capture,
        # bornée par capture.json. Sans ces bornes, la fenêtre compterait aussi les quelque 20 s que
        # met wpr -stop à écrire la trace, pendant lesquelles le jeu tourne encore.
        window = capture_window(folder) if folder else None
        ours = [s for s in game if window[0] <= s[3] < window[1]] if window else game
    if not ours:
        lines.append("Aucun échantillon du jeu dans la fenêtre.\n")
        return
    if window:
        span = (window[1] - window[0]).total_seconds()
    else:
        span = (ours[-1][0] - ours[0][0]) / tr.perf_freq

    lines.append("## Trace WPR\n")
    if window:
        lines.append(f"- Fenêtre : {window[0]:%H:%M:%S} → {window[1]:%H:%M:%S} ({span:.0f} s).")
    else:
        lines.append(f"- Toute la capture ({span:.0f} s).")
    n_stack = sum(1 for s in ours if (s[0], s[1]) in tr.stacks)
    lines.append(f"- {civ[0][3]} (pid {pid}) : {len(ours)} échantillons à {ms:g} ms, "
                 f"soit {len(ours) * ms / 1000:.1f} s de CPU ; {n_stack} avec pile.\n")
    passages = passages_report(game, trans, module, ms, tr.perf_freq, lines)
    frames_report(tr, pid, min(s[0] for s in ours), max(s[0] for s in ours), passages, wall, lines)
    if folder:
        gpu_report(folder, pid, window, [(wall(a), wall(b)) for a, b in passages], lines)
    net_report(tr, pid, min(s[0] for s in ours), max(s[0] for s in ours), lines)

    gc_thread = collections.Counter(s[1] for s in ours if module(s[2]) == GAMECORE)
    main_gc = gc_thread.most_common(1)[0][0] if gc_thread else None
    # Pendant le tour du joueur, le thread GameCore dort : celui qui passe le plus de temps dans
    # GameCore est alors le thread principal, par ses appels à l'API Lua. On ne le prend pour le
    # thread GameCore que s'il y passe au moins un cinquième de son temps.
    by_thread = collections.Counter(s[1] for s in ours)
    if main_gc and gc_thread[main_gc] < by_thread[main_gc] / 5:
        main_gc = None
    threads_report(tr, ours, mods, module, span, ms, main_gc, lines)
    if folder:
        zones_report(folder, pid, ours, ms, main_gc, lines)

    def module_table(title, samples):
        c = collections.Counter(module(s[2]) for s in samples)
        lines.append(f"\n### {title}\n")
        lines.append("| module | CPU (ms) | % |")
        lines.append("|---|---:|---:|")
        for name, n in c.most_common(12):
            lines.append(f"| {name} | {n * ms:.0f} | {100 * n / len(samples):.1f} |")

    module_table("Modules, tout le processus (temps propre)", ours)
    gct = [s for s in ours if s[1] == main_gc]
    if not gct:
        lines.append("")
        return
    module_table(f"Modules, thread GameCore {main_gc} (temps propre)", gct)

    # Fonctions GameCore sur le thread GameCore : propre (IP) et inclusif (présente dans la pile).
    self_f, incl_f, self_cpp, incl_cpp = (collections.Counter() for _ in range(4))
    leaf_mod = collections.defaultdict(collections.Counter)  # dernière fonction GameCore -> module de l'IP
    tree = {}
    with_stack = 0
    for qpc, tid, ip, _ in gct:
        a = gc_func(ip)
        if a:
            self_f[a] += 1
            self_cpp[funcs.by_addr[a]["fichier"] or "?"] += 1
        frames = tr.stacks.get((qpc, tid))
        if not frames:
            continue
        with_stack += 1
        chain = []
        for f in reversed(frames):  # de la racine vers la feuille
            fa = gc_func(f)
            if fa and (not chain or chain[-1] != fa):
                chain.append(fa)
        for x in set(chain):
            incl_f[x] += 1
        for c in {funcs.by_addr[x]["fichier"] or "?" for x in chain}:
            incl_cpp[c] += 1
        if chain:
            leaf_mod[chain[-1]][module(ip)] += 1
        node = tree
        for x in chain[:16]:
            node = node.setdefault(x, [0, {}])
            node[0] += 1
            node = node[1]

    lines.append(f"\n### Arbre d'appels GameCore (thread {main_gc}, {with_stack} piles)\n")
    lines.append("Seules les fonctions GameCore sont gardées dans les piles ; nœuds ≥ 1 % du thread.\n")
    lines.append("```")
    floor = max(1, with_stack // 100)

    def walk(node, depth):
        for a, (n, kids) in sorted(node.items(), key=lambda kv: -kv[1][0]):
            if n >= floor:
                lines.append(f"{'  ' * depth}{n * ms:7.0f} ms  {funcs.label(a)}")
                walk(kids, depth + 1)

    walk(tree, 0)
    lines.append("```")

    lines.append("\n### Fonctions GameCore, temps inclusif\n")
    lines.append("| fonction | inclusif (ms) | propre (ms) |")
    lines.append("|---|---:|---:|")
    for a, n in incl_f.most_common(40):
        lines.append(f"| {funcs.label(a)} | {n * ms:.0f} | {self_f[a] * ms:.0f} |")

    outside = collections.Counter({a: sum(n for m, n in c.items() if m != GAMECORE) for a, c in leaf_mod.items()})
    lines.append("\n### Fonctions GameCore, temps propre et appels hors GameCore en feuille\n")
    lines.append("| fonction | propre (ms) | feuille hors GameCore (ms) | modules |")
    lines.append("|---|---:|---:|---|")
    for a, _ in (self_f + outside).most_common(40):
        other = ", ".join(f"{m} {v * ms:.0f}" for m, v in leaf_mod[a].most_common(4) if m != GAMECORE)
        lines.append(f"| {funcs.label(a)} | {self_f[a] * ms:.0f} | {outside[a] * ms:.0f} | {other} |")

    lines.append("\n### Fichiers source GameCore\n")
    lines.append("| fichier | inclusif (ms) | propre (ms) |")
    lines.append("|---|---:|---:|")
    for c, n in incl_cpp.most_common(30):
        lines.append(f"| {c} | {n * ms:.0f} | {self_cpp[c] * ms:.0f} |")
    lines.append("")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dossier", type=pathlib.Path)
    ap.add_argument("--out", type=pathlib.Path)
    ap.add_argument("--tout", action="store_true", help="toute la capture, pas seulement le dernier passage de tour")
    args = ap.parse_args()

    lines = [f"# Passage de tour : {args.dossier.name}\n"]
    trans = log_report(args.dossier, lines)
    etl = args.dossier / "trace.etl"
    if etl.exists():
        window = None
        if trans and trans[-1]["fin"] and not args.tout:
            # Les journaux sont à la seconde : on borne large d'une seconde de chaque côté.
            window = (trans[-1]["fin"], trans[-1]["main"] + dt.timedelta(seconds=1))
        if not args.tout and capture_meta(args.dossier).get("light"):
            window = capture_window(args.dossier)  # toute la partie, pas le dernier passage
        elif not args.tout and window is None:
            window = capture_window(args.dossier)  # aucun passage dans les journaux de la session
        trace_report(etl, lines, window, trans, args.dossier)
    out = args.out or args.dossier / "rapport.md"
    out.write_text("\n".join(lines), encoding="utf-8")
    print(out)


if __name__ == "__main__":
    sys.exit(main())
