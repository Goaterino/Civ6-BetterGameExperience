"""Attache hook_lobby.js au jeu et écrit ses événements dans un journal horodaté.

    .venv/Scripts/python.exe -I analyses/lobby/hook_lobby.py [dossier]

À lancer sur chaque machine du test (hôte et clients), jeu démarré en DX12, **avant** de créer ou de
rejoindre le salon. Sortie dans <dossier>, par défaut analyses/lobby/runs/<date-heure>/ :
- frida.log : un événement par ligne (heure à la ms, thread, type, JSON) ;
- configs/<n>_<type>.txt : texte complet de chaque NetGameConfig / NetPlayerInfo reçu, que le jeu
  coupe à 32 Ko dans net_message_debug.log. À décoder avec parse_netconfig.py.

Entrée : photo des 16 premiers emplacements de la config locale. « fin » + Entrée, ou Ctrl+C,
pour arrêter.
"""

import datetime
import json
import pathlib
import sys
import threading
import time

import frida

HERE = pathlib.Path(__file__).resolve().parent
PROCESS_NAMES = ("CivilizationVI_DX12.exe", "CivilizationVI.exe")


def find_game(device):
    for p in device.enumerate_processes():
        if p.name in PROCESS_NAMES:
            return p
    return None


def main():
    out_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else (
        HERE / "runs" / datetime.datetime.now().strftime("%Y%m%d-%H%M%S"))
    (out_dir / "configs").mkdir(parents=True, exist_ok=True)
    log_path = out_dir / "frida.log"
    device = frida.get_local_device()

    proc = find_game(device)
    while proc is None:
        print("En attente de Civilization VI…", flush=True)
        time.sleep(2)
        proc = find_game(device)

    log = open(log_path, "a", encoding="utf-8")
    done = []
    count = [0]

    def on_message(message, data):
        if message["type"] == "send":
            p = dict(message["payload"])
            t = datetime.datetime.fromtimestamp(p.pop("t") / 1000)
            kind = p.pop("kind")
            if data:
                count[0] += 1
                f = out_dir / "configs" / f"{count[0]:04d}_{t:%H%M%S}_{kind}.txt"
                f.write_bytes(data)
                p["fichier"] = f.name
            line = f"{t:%Y-%m-%d %H:%M:%S.%f}"[:-3] + f"\t{p.pop('tid')}\t{kind}\t" + json.dumps(
                p, ensure_ascii=False)
        else:
            line = f"{datetime.datetime.now():%Y-%m-%d %H:%M:%S}\t-\terreur_script\t{message.get('description')}"
        log.write(line + "\n")
        log.flush()
        print(line[:400], flush=True)

    session = device.attach(proc.pid)
    session.on("detached", lambda reason, crash: done.append(reason))
    script = session.create_script((HERE / "hook_lobby.js").read_text(encoding="utf-8"))
    script.on("message", on_message)
    script.load()
    print(f"Attaché à {proc.name} (pid {proc.pid}). Journal : {log_path}", flush=True)

    def clavier():
        for ligne in sys.stdin:
            if ligne.strip().lower() == "fin":
                done.append("fin demandée")
                return
            try:
                script.exports_sync.emplacements(16)
            except Exception as e:  # session détachée
                print(f"photo impossible : {e}", flush=True)

    threading.Thread(target=clavier, daemon=True).start()
    try:
        while not done:
            time.sleep(0.5)
        print(f"Arrêt : {done[0]}", flush=True)
        try:
            session.detach()
        except frida.InvalidOperationError:
            pass
    except KeyboardInterrupt:
        session.detach()
    log.close()


if __name__ == "__main__":
    main()
