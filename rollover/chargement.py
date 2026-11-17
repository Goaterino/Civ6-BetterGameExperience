"""Ventilation du temps d'un chargement de partie de Civ6 : entrée depuis le salon, reprise d'une
sauvegarde, remap de MPH (`Network.RestartGame`).

Trois journaux du jeu :
- LoadGameViewState.log : les phases du chargement (`X Begin`, `X took N seconds`), à la seconde ;
- Modding.log : la reconstruction des bases de données (CONFIGURE_CONTENT) et les étapes
  (`Stage: …`), à la milliseconde sur une horloge propre au journal, recalée sur l'heure par
  CONFIGURE_CONTENT ;
- UserInterface.log : le chargement des contextes Lua de l'interface (temps de chaque fichier),
  sans horodatage : le n-ième écran de chargement du journal est apparié au n-ième chargement.
Et net_message_debug.log pour situer l'instantané réseau (envoi chez l'hôte, réception chez le
client) et la fin de la mise en file des messages, quand le joueur a vraiment la main.

Une capture peut contenir plusieurs chargements (reprise, puis remap, puis nouvelle partie) : avec
une fenêtre, chacun est détaillé ; sans fenêtre, le dernier seulement. Les journaux du jeu sont
réécrits à chaque lancement de l'exe : il faut revenir au menu principal entre deux parties, pas
quitter le jeu.

Usage seul (sans trace WPR) :
    python -I rollover/chargement.py <dossier de capture ou dossier Logs du jeu> [--tous]
Avec une trace : analyze.py --chargement, qui borne aussi l'analyse CPU à chaque chargement.
"""
import collections
import datetime as dt
import pathlib
import re
import sys

LIGNE = re.compile(r"\[(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)\]\t(.*)")
MODDING = re.compile(r"\[(\d+\.\d+)\]\s*(.*)")


def _lire(path):
    if not path.exists():
        return
    for line in path.open(encoding="utf-8", errors="replace"):
        m = LIGNE.match(line.rstrip("\n"))
        if m:
            yield dt.datetime.strptime(m[1], "%Y-%m-%d %H:%M:%S"), m[2]


def blocs(logs):
    """Un bloc par passage dans l'écran de chargement (LoadGameViewState::Begin → End) : début, fin,
    et les phases dans l'ordre du journal. Chaque phase : nom, durée en s, instant de sa dernière
    ligne, et ses sous-phases (nom, durée, instant). Une phase sans ligne « took » (WAIT_FOR_PLAYERS,
    bornée par Begin et End) prend la somme de ses sous-phases."""
    out, cur, ouverte = [], None, None
    for t, msg in _lire(logs / "LoadGameViewState.log"):
        txt = msg.strip()
        if txt.startswith("LoadGameViewState::Begin"):
            cur, ouverte = {"debut": t, "fin": None, "phases": [], "rang": len(out)}, None
            out.append(cur)
        elif cur is None:
            continue
        elif txt.startswith("LoadGameViewState::End"):
            cur["fin"] = t
        elif (m := re.match(r"(\S+) Begin$", txt)):
            ouverte = {"nom": m[1], "d": None, "t": t, "subs": []}
            cur["phases"].append(ouverte)
        elif (m := re.match(r",(.+?) took ([\d.]+) seconds", txt)):
            if ouverte is not None:
                ouverte["subs"].append((m[1].strip(), float(m[2]), t))
        elif (m := re.match(r"(\S+) took ([\d.]+) seconds", txt)):
            if ouverte is not None and ouverte["nom"] == m[1]:
                ouverte["d"], ouverte["t"] = float(m[2]), t
            else:
                cur["phases"].append({"nom": m[1], "d": float(m[2]), "t": t, "subs": []})
            ouverte = None
        elif (m := re.match(r"(\S+) End$", txt)):
            if ouverte is not None and ouverte["nom"] == m[1]:
                ouverte["d"], ouverte["t"] = sum(d for _, d, _ in ouverte["subs"]), t
            ouverte = None
        elif (m := re.match(r"PostLoadTime: ([\d.]+) seconds", txt)):
            cur["phases"].append({"nom": "PostLoadTime", "d": float(m[1]), "t": t, "subs": []})
    for b in out:
        b["phases"] = [p for p in b["phases"] if p["d"] is not None]
    return out


def _bloc_report(b, lines):
    total = (b["fin"] - b["debut"]).total_seconds() if b["fin"] else None
    lines.append(f"Chargement de {b['debut']:%H:%M:%S}"
                 + (f" à {b['fin']:%H:%M:%S} ({total:.0f} s, à la seconde près)" if total is not None else
                    ", jamais terminé") + ". Les sous-phases à 0 s ne sont pas chronométrées par le jeu : "
                 "leur instant (à la seconde) montre où le temps passe.\n")
    lines.append("| phase | durée (s) | à (s) |")
    lines.append("|---|---:|---:|")
    for p in b["phases"]:
        for nom, d, t in p["subs"]:
            lines.append(f"| ↳ {nom} | {d:.2f} | {(t - b['debut']).total_seconds():.0f} |")
        lines.append(f"| **{p['nom']}** | {p['d']:.2f} | {(p['t'] - b['debut']).total_seconds():.0f} |")
    if total is not None:
        connu = sum(p["d"] for p in b["phases"])
        lines.append(f"| *non attribué (bornes à la seconde)* | {total - connu:.1f} | |")
    lines.append("")


def reseau(logs, b):
    """Instantané réseau autour du bloc : dernier NetSnapshot envoyé ou reçu dans la minute avant le
    début, et fin de la mise en file après la fin (le joueur a la main)."""
    snap = fin_file = None
    for t, msg in _lire(logs / "net_message_debug.log"):
        if (b["debut"] - dt.timedelta(seconds=60) <= t <= b["debut"] + dt.timedelta(seconds=1)
                and "NetSnapshot :" in msg and "m_snapShotDataSize" in msg):
            snap = (t, msg)
        if b["fin"] and t >= b["fin"] - dt.timedelta(seconds=1) and "Queuing Messages End" in msg and not fin_file:
            fin_file = t
    return snap, fin_file


def modding_events(logs):
    path = logs / "Modding.log"
    if not path.exists():
        return []
    return [(float(m[1]), m[2]) for line in path.open(encoding="utf-8", errors="replace")
            if (m := MODDING.match(line.rstrip("\n")))]


def configure_segments(ev):
    """Passages dans CONFIGURE_CONTENT (Modding.log) : de « Stage: Configure Content » à
    « Successfully reconfigured game. ». Les reconfigurations du salon (Configuring game content,
    Apply Settings) ne sont pas des chargements et sont ignorées."""
    out, cur, stage = [], None, None
    for t, msg in ev:
        if msg.startswith("Stage: ") and msg != "Stage: Configure Content":
            stage = msg[7:]
            if cur is not None:
                cur["suite"] = stage
                cur = None
        if msg == "Stage: Configure Content":
            cur = {"avant": stage, "suite": None, "ev": []}
            out.append(cur)
        if cur is not None:
            cur["ev"].append((t, msg))
    for seg in out:
        seg["fin"] = next((t for t, m in seg["ev"] if m.startswith("Successfully reconfigured")), seg["ev"][-1][0])
    return out


def decalage(bs, segs):
    """Heure (timestamp) moins horloge de Modding.log. La fin de CONFIGURE_CONTENT est notée dans les
    deux journaux : on garde le décalage sur lequel s'accordent le plus de couples (bloc, passage)."""
    couples = [p["t"].timestamp() - s["fin"] for b in bs for p in b["phases"] if p["nom"] == "CONFIGURE_CONTENT"
               for s in segs]
    if not couples:
        return None
    meilleur = max(couples, key=lambda c: sum(abs(c - d) < 1.5 for d in couples))
    proches = sorted(d for d in couples if abs(d - meilleur) < 1.5)
    return proches[len(proches) // 2]


REPERES = re.compile(r"pre configure|Reloading Gameplay DLL|Rebuilding|Reverted|Updat(ing|ed) gameplay database|"
                     r"Importing implicit|Applying Components|Finished Apply|Applied all|Caching data|"
                     r"Initializing Gameplay DLL|debug version|post configure|Reloading icons|Reloading colors|"
                     r"Successfully reconfigured")


def _configure_report(seg, lines):
    ev = seg["ev"]
    t0 = ev[0][0]
    fin = seg["fin"]
    lines.append(f"CONFIGURE_CONTENT (Modding.log, à la milliseconde), {fin - t0:.2f} s, après « {seg['avant']} »"
                 + (f", avant « {seg['suite']} »" if seg["suite"] else "") + " :\n")
    lines.append("| t (s) | étape | jusqu'à l'étape suivante (s) |")
    lines.append("|---:|---|---:|")
    reperes = [(t, m) for t, m in ev if REPERES.search(m)]
    for i, (t, m) in enumerate(reperes):
        nxt = reperes[i + 1][0] if i + 1 < len(reperes) else fin
        lines.append(f"| {t - t0:.3f} | {m[:90]} | {nxt - t:.3f} |")
    # Temps par composant : d'un « Applying Component » au suivant (ou à la fin des composants).
    comp, fichiers = collections.Counter(), collections.Counter()
    courant = None
    for i, (t, m) in enumerate(ev[:-1]):
        d = ev[i + 1][0] - t
        if (mc := re.match(r"Applying Component - (.+)", m)):
            courant = mc[1]
        elif m.startswith("Modding Framework - Finished Apply"):
            courant = None
        if courant:
            comp[courant] += d
        if (mf := re.match(r"(?:UpdateDatabase|LocalizedText|UpdateIcons|UpdateColors) - Loading (.+)", m)):
            fichiers[mf[1]] += d
    if comp:
        lines.append(f"\nComposants les plus lents ({len(comp)} composants, {sum(comp.values()):.2f} s au total) :\n")
        lines.append("| composant | s |")
        lines.append("|---|---:|")
        for c, d in comp.most_common(15):
            lines.append(f"| {c} | {d:.3f} |")
    if fichiers:
        textes = sum(d for f, d in fichiers.items() if re.search(r"text|lang/|translation", f, re.I))
        lines.append(f"\nFichiers les plus lents ({len(fichiers)} fichiers, {sum(fichiers.values()):.2f} s, "
                     f"dont {textes:.2f} s de textes) :\n")
        lines.append("| fichier | s |")
        lines.append("|---|---:|")
        for f, d in fichiers.most_common(15):
            lines.append(f"| {f[-80:]} | {d:.3f} |")
    lines.append("")


def _stages_report(ev, off, b, lines):
    """Étapes de Modding.log (`Stage: …`, reconfigurations) pendant le chargement et la minute qui le
    précède : une nouvelle partie (remap) n'a pas les mêmes étapes qu'une reprise."""
    lo = b["debut"].timestamp() - 60 - off
    hi = (b["fin"] or b["debut"]).timestamp() + 2 - off
    st = [(t, m) for t, m in ev if lo <= t <= hi
          and (m.startswith("Stage:") or m.startswith("Configuring game content")
               or m.startswith("Successfully reconfigured"))]
    if not st:
        return
    lines.append("Étapes de Modding.log autour du chargement (heure recalée) :\n")
    lines.append("| heure | t / début (s) | étape | jusqu'à la suivante (s) |")
    lines.append("|---|---:|---|---:|")
    for i, (t, m) in enumerate(st):
        h = dt.datetime.fromtimestamp(t + off)
        nxt = f"{st[i + 1][0] - t:.3f}" if i + 1 < len(st) else ""
        lines.append(f"| {h:%H:%M:%S}.{h.microsecond // 1000:03d} | {t + off - b['debut'].timestamp():.1f} | "
                     f"{m[:80]} | {nxt} |")
    lines.append("")


def ecrans_ui(logs):
    """Pour chaque écran de chargement de UserInterface.log : les fichiers Lua chargés et
    l'initialisation des contrôles, jusqu'à l'écran suivant."""
    path = logs / "UserInterface.log"
    if not path.exists():
        return []
    L = path.read_text(encoding="utf-8", errors="replace").splitlines()
    debut = [i for i, l in enumerate(L) if "LOADING" in l and re.search(r"[/\\]LoadScreen\.lua", l)]
    out = []
    for k, i in enumerate(debut):
        fichiers, init = [], []
        for l in L[i:debut[k + 1] if k + 1 < len(debut) else len(L)]:
            if (m := re.search(r"LOADING (.*?)\s+Load time: ([\d.]+)", l)):
                fichiers.append((float(m[2]), m[1]))
            elif (m := re.search(r"(\S+): ControlBase::Initialize time: ([\d.]+)", l)):
                init.append((float(m[2]), m[1]))
        out.append((fichiers, init))
    return out


def _ui_report(ecran, lines):
    fichiers, init = ecran
    lines.append(f"Interface (UserInterface.log) : {len(fichiers)} fichiers Lua, {sum(d for d, _ in fichiers):.2f} s "
                 f"d'exécution du corps des fichiers, {sum(d for d, _ in init):.2f} s d'initialisation des "
                 "contrôles. Les gestionnaires d'événements appelés ensuite (LoadGameViewStateDone…) ne sont "
                 "pas chronométrés par le jeu.\n")
    lines.append("| fichier ou contexte | s |")
    lines.append("|---|---:|")
    for d, n in sorted(fichiers + [(d, f"contrôles de {c}") for d, c in init], reverse=True)[:12]:
        lines.append(f"| {n[-80:]} | {d:.3f} |")
    lines.append("")


class Journaux:
    """Les journaux d'un dossier, lus une fois, pour détailler plusieurs chargements."""

    def __init__(self, logs):
        self.logs = logs
        self.blocs = blocs(logs)
        self.ev = modding_events(logs)
        self.segs = configure_segments(self.ev)
        self.off = decalage(self.blocs, self.segs)
        self.ecrans = ecrans_ui(logs)

    def resume(self, lines):
        bs = self.blocs
        if not bs:
            lines.append("## Chargements\n\nAucun écran de chargement dans LoadGameViewState.log.\n")
            return
        lines.append("## Chargements (LoadGameViewState.log)\n")
        noms = list(dict.fromkeys(p["nom"] for b in bs for p in b["phases"]))
        lines.append("| n° | début | durée (s) | " + " | ".join(noms) + " |")
        lines.append("|---:|---|---:|" + "---:|" * len(noms))
        for b in bs:
            dur = {p["nom"]: p["d"] for p in b["phases"]}
            total = f"{(b['fin'] - b['debut']).total_seconds():.0f}" if b["fin"] else "?"
            lines.append(f"| {b['rang'] + 1} | {b['debut']:%H:%M:%S} | {total} | "
                         + " | ".join(f"{dur[n]:.2f}" if n in dur else "" for n in noms) + " |")
        lines.append("")

    def dans(self, fenetre):
        """Les chargements commencés dans la fenêtre (début, fin), ou le dernier sans fenêtre."""
        if fenetre is None:
            return self.blocs[-1:]
        return [b for b in self.blocs if fenetre[0] <= b["debut"] <= fenetre[1]]

    def detail(self, b, lines):
        lines.append(f"## Chargement n° {b['rang'] + 1}, {b['debut']:%H:%M:%S}\n")
        snap, fin_file = reseau(self.logs, b)
        if snap:
            lines.append(f"- Instantané réseau : `{snap[1][:110]}` à {snap[0]:%H:%M:%S}, "
                         f"{(b['debut'] - snap[0]).total_seconds():.0f} s avant le début du chargement.")
        else:
            lines.append("- Aucun instantané réseau dans la minute avant le chargement.")
        if fin_file:
            lines.append(f"- Fin de la mise en file des messages (le joueur a la main) : {fin_file:%H:%M:%S}, "
                         f"{(fin_file - b['debut']).total_seconds():.0f} s après le début.")
        lines.append("")
        _bloc_report(b, lines)
        if self.off is not None:
            _stages_report(self.ev, self.off, b, lines)
            cc = next((p["t"].timestamp() for p in b["phases"] if p["nom"] == "CONFIGURE_CONTENT"), None)
            seg = next((s for s in self.segs if cc is not None and abs(s["fin"] + self.off - cc) < 2), None)
            if seg:
                _configure_report(seg, lines)
        # UserInterface.log n'a pas d'heure : appariement par rang, compté depuis la fin (le début du
        # journal peut porter des écrans d'avant le premier bloc).
        k = len(self.ecrans) - (len(self.blocs) - b["rang"])
        if 0 <= k < len(self.ecrans):
            _ui_report(self.ecrans[k], lines)


def rapport(logs, lines, fenetre=None):
    """Résumé de tous les chargements, puis détail de ceux de la fenêtre (le dernier sans fenêtre).
    Rend les blocs détaillés."""
    j = Journaux(logs)
    j.resume(lines)
    choisis = j.dans(fenetre)
    for b in choisis:
        j.detail(b, lines)
    return choisis


def main():
    args = [a for a in sys.argv[1:] if a != "--tous"]
    if len(args) != 1:
        print(__doc__)
        return 2
    folder = pathlib.Path(args[0])
    lines = [f"# Chargement : {folder.name}\n"]
    tous = (dt.datetime.min, dt.datetime.max) if "--tous" in sys.argv else None
    rapport(folder, lines, tous)
    sys.stdout.reconfigure(encoding="utf-8")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
