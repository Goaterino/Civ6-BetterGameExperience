"""Lecture directe d'une trace WPR (.etl) par l'API de consommation ETW de Windows.

Plus rapide et plus léger que tracerpt -> XML (une capture CPU + E/S de 1 minute fait plusieurs Go).
On ne décode que les événements noyau utiles, par leur disposition binaire x64 connue :
images chargées, threads (et leurs noms), échantillons CPU et piles, et, pour les seuls threads de
Civilization VI, les changements de contexte (CSwitch) et réveils (ReadyThread) avec leurs piles.
S'y ajoutent les présentations DXGI (profil Frames de civ6.wprp), une par image, et le trafic réseau
de tous les processus (un événement par paquet UDP ou segment TCP : pid, taille).

Les horodatages sont bruts (compteur QPC) pour pouvoir apparier une pile à son échantillon.
"""
import ctypes
import struct
import sys
import uuid
from ctypes import wintypes

PROCESS_TRACE_MODE_RAW_TIMESTAMP = 0x00001000
PROCESS_TRACE_MODE_EVENT_RECORD = 0x10000000
INVALID_HANDLE = 0xFFFFFFFFFFFFFFFF


def guid(s):
    return uuid.UUID(s).bytes_le


PERFINFO = guid("ce1dbfb4-137e-4da6-87b0-3f59aa102cbc")
STACKWALK = guid("def2fe46-7bd6-4b80-bd94-f57fe20d0ce3")
IMAGE = guid("2cb15d1d-5fc1-11d2-abe1-00a0c911f518")
THREAD = guid("3d6fa8d1-fe05-11d0-9dda-00c04fd7ba7c")
DXGI = guid("ca11c036-0102-4a2d-a6ad-f03cfed5d3c9")  # Microsoft-Windows-DXGI
NET = guid("7dd42a49-5329-4832-8dfd-43d979153a88")  # Microsoft-Windows-Kernel-Network
WANTED = {PERFINFO, STACKWALK, IMAGE, THREAD, DXGI, NET}
# Present_Start / Present_Stop, et leurs variantes PresentMultiplaneOverlay.
DXGI_START, DXGI_STOP = {42, 55}, {43, 56}
# Kernel-Network, données envoyées / reçues : UDP IPv4 (42, 43) et IPv6 (58, 59), TCP IPv4 (10, 11)
# et IPv6 (26, 27). Le pid et la taille ouvrent la charge utile, quelle que soit la famille.
NET_EVENTS = {42: ("udp", False), 43: ("udp", True), 58: ("udp", False), 59: ("udp", True),
              10: ("tcp", False), 11: ("tcp", True), 26: ("tcp", False), 27: ("tcp", True)}


class GUID(ctypes.Structure):
    _fields_ = [("b", ctypes.c_ubyte * 16)]


class EVENT_TRACE_HEADER(ctypes.Structure):
    _fields_ = [("Size", ctypes.c_ushort), ("FieldTypeFlags", ctypes.c_ushort), ("Version", ctypes.c_ulong),
                ("ThreadId", ctypes.c_ulong), ("ProcessId", ctypes.c_ulong), ("TimeStamp", ctypes.c_longlong),
                ("Guid", GUID), ("ClientContext", ctypes.c_ulong), ("Flags", ctypes.c_ulong)]


class EVENT_TRACE(ctypes.Structure):
    _fields_ = [("Header", EVENT_TRACE_HEADER), ("InstanceId", ctypes.c_ulong),
                ("ParentInstanceId", ctypes.c_ulong), ("ParentGuid", GUID), ("MofData", ctypes.c_void_p),
                ("MofLength", ctypes.c_ulong), ("BufferContext", ctypes.c_ulong)]


class SYSTEMTIME(ctypes.Structure):
    _fields_ = [(n, ctypes.c_ushort) for n in ("Y", "M", "DoW", "D", "h", "m", "s", "ms")]


class TIME_ZONE_INFORMATION(ctypes.Structure):
    _fields_ = [("Bias", ctypes.c_long), ("StandardName", ctypes.c_wchar * 32), ("StandardDate", SYSTEMTIME),
                ("StandardBias", ctypes.c_long), ("DaylightName", ctypes.c_wchar * 32),
                ("DaylightDate", SYSTEMTIME), ("DaylightBias", ctypes.c_long)]


class TRACE_LOGFILE_HEADER(ctypes.Structure):
    _fields_ = [("BufferSize", ctypes.c_ulong), ("Version", ctypes.c_ulong), ("ProviderVersion", ctypes.c_ulong),
                ("NumberOfProcessors", ctypes.c_ulong), ("EndTime", ctypes.c_longlong),
                ("TimerResolution", ctypes.c_ulong), ("MaximumFileSize", ctypes.c_ulong),
                ("LogFileMode", ctypes.c_ulong), ("BuffersWritten", ctypes.c_ulong), ("LogInstanceGuid", GUID),
                ("LoggerName", ctypes.c_void_p), ("LogFileName", ctypes.c_void_p),
                ("TimeZone", TIME_ZONE_INFORMATION), ("BootTime", ctypes.c_longlong),
                ("PerfFreq", ctypes.c_longlong), ("StartTime", ctypes.c_longlong),
                ("ReservedFlags", ctypes.c_ulong), ("BuffersLost", ctypes.c_ulong)]


EVENT_RECORD_CALLBACK = ctypes.WINFUNCTYPE(None, ctypes.c_void_p)
BUFFER_CALLBACK = ctypes.WINFUNCTYPE(wintypes.ULONG, ctypes.c_void_p)


class EVENT_TRACE_LOGFILEW(ctypes.Structure):
    _fields_ = [("LogFileName", ctypes.c_wchar_p), ("LoggerName", ctypes.c_wchar_p),
                ("CurrentTime", ctypes.c_longlong), ("BuffersRead", ctypes.c_ulong),
                ("ProcessTraceMode", ctypes.c_ulong), ("CurrentEvent", EVENT_TRACE),
                ("LogfileHeader", TRACE_LOGFILE_HEADER), ("BufferCallback", BUFFER_CALLBACK),
                ("BufferSize", ctypes.c_ulong), ("Filled", ctypes.c_ulong), ("EventsLost", ctypes.c_ulong),
                ("EventRecordCallback", EVENT_RECORD_CALLBACK), ("IsKernelTrace", ctypes.c_ulong),
                ("Context", ctypes.c_void_p)]


advapi = ctypes.WinDLL("advapi32")
advapi.OpenTraceW.restype = ctypes.c_uint64
advapi.OpenTraceW.argtypes = [ctypes.POINTER(EVENT_TRACE_LOGFILEW)]
advapi.ProcessTrace.argtypes = [ctypes.POINTER(ctypes.c_uint64), ctypes.c_ulong, ctypes.c_void_p, ctypes.c_void_p]
advapi.CloseTrace.argtypes = [ctypes.c_uint64]

HEADER = struct.Struct("<HHHHIIq16sHBBBBHQ")  # EVENT_HEADER jusqu'au descripteur inclus
string_at = ctypes.string_at


class Trace:
    def __init__(self):
        self.images = []        # (pid, base, taille, nom de fichier)
        self.thread_pid = {}    # tid -> pid
        self.thread_name = {}   # tid -> nom
        self.samples = []       # (qpc, tid, ip)
        self.stacks = {}        # (qpc, tid) -> [adresses], piles des échantillons et des événements ci-dessous
        # Threads de Civilization VI seulement :
        # (qpc, tid entrant, tid sortant, raison d'attente du sortant, état du sortant, attente de l'entrant)
        self.cswitches = []
        self.readies = []       # (qpc, tid qui réveille ou None sans pile, tid réveillé)
        # Présentations DXGI, tous processus : (qpc, pid, tid, début ?, drapeaux, intervalle de synchro)
        # au début ; (qpc, pid, tid, False, résultat, None) à la fin.
        self.presents = []
        self.packets = []       # Trafic réseau, tous processus : (qpc, pid, "udp"/"tcp", reçu ?, octets)
        self.interval_100ns = None
        self.perf_freq = self.start_filetime = None
        self.first_qpc = None
        self.events = 0


def read(path, max_buffers=None):
    tr = Trace()
    sample_keys = set()
    pending_stacks = {}  # piles arrivées avant leur échantillon (rare, selon l'ordre des tampons)
    civ_pids = set()
    ready_at = {}  # horodatage d'un ReadyThread -> son indice, en attendant sa pile

    def civ(t):
        return tr.thread_pid.get(t) in civ_pids

    def want_stack(key):
        if key in pending_stacks:
            tr.stacks[key] = pending_stacks.pop(key)
        else:
            sample_keys.add(key)

    def on_event(rec):
        tr.events += 1
        (size, htype, flags, prop, tid, pid, ts, provider, eid, ver, chan, level, opcode, task, kw) = \
            HEADER.unpack(string_at(rec, 56))
        if provider not in WANTED:
            return
        if tr.first_qpc is None or ts < tr.first_qpc:
            tr.first_qpc = ts
        length, = struct.unpack("<H", string_at(rec + 86, 2))  # EVENT_RECORD.UserDataLength
        data = string_at(ctypes.c_void_p.from_address(rec + 96).value, length) if length else b""
        if provider == DXGI:
            if eid in DXGI_START and len(data) >= 16:
                _, flags, sync = struct.unpack_from("<QII", data)
                tr.presents.append((ts, pid, tid, True, flags, sync))
            elif eid in DXGI_STOP and len(data) >= 4:
                tr.presents.append((ts, pid, tid, False, struct.unpack_from("<I", data)[0], None))
            return
        if provider == NET:
            if eid in NET_EVENTS and len(data) >= 8:
                proto, recv = NET_EVENTS[eid]
                net_pid, size = struct.unpack_from("<II", data)
                tr.packets.append((ts, net_pid, proto, recv, size))
            return
        if provider == PERFINFO:
            if opcode == 46 and len(data) >= 12:  # SampledProfile
                ip, stid = struct.unpack_from("<QI", data)
                tr.samples.append((ts, stid, ip))
                want_stack((ts, stid))
            elif opcode in (73, 74) and len(data) >= 8:  # intervalle d'échantillonnage
                source, interval = struct.unpack_from("<II", data)
                if source == 0:
                    tr.interval_100ns = interval
        elif provider == STACKWALK and opcode == 32 and len(data) >= 16:
            qpc, spid, stid = struct.unpack_from("<QII", data)
            frames = list(struct.unpack_from(f"<{(len(data) - 16) // 8}Q", data, 16))
            key = (qpc, stid)
            # Une pile peut arriver en deux événements de même clé : partie noyau puis partie utilisateur.
            if key in tr.stacks:
                tr.stacks[key] = tr.stacks[key] + frames
                return
            i = ready_at.pop(qpc, None)
            if i is not None:
                tr.readies[i] = (qpc, stid, tr.readies[i][2])
                tr.stacks[key] = frames
            if key in sample_keys:
                sample_keys.discard(key)
                tr.stacks[key] = frames
            elif len(pending_stacks) < 200000:
                pending_stacks[key] = frames
        elif provider == IMAGE and opcode in (10, 3, 4) and len(data) >= 56:
            base, size_, ipid = struct.unpack_from("<QQI", data)
            name = data[56:].decode("utf-16-le", "replace").split("\0", 1)[0]
            tr.images.append((ipid, base, size_, name.rsplit("\\", 1)[-1].lower()))
            if tr.images[-1][3].startswith("civilizationvi"):
                civ_pids.add(ipid)
        elif provider == THREAD:
            if opcode == 36 and len(data) >= 24:  # CSwitch ; la pile est celle du thread entrant
                new, old, _, _, _, _, reason, _, state, _, wait = struct.unpack_from("<IIbbBbbbbbI", data)
                if civ(new) or civ(old):
                    tr.cswitches.append((ts, new, old, reason, state, wait))
                    if civ(new):
                        want_stack((ts, new))
            elif opcode == 50 and len(data) >= 4:  # ReadyThread
                # L'en-tête ne dit pas qui réveille : on le reprend de la pile, celle du thread qui
                # réveille, appariée par horodatage.
                target, = struct.unpack_from("<I", data)
                if civ(target):
                    ready_at[ts] = len(tr.readies)
                    tr.readies.append((ts, None, target))
            elif opcode in (1, 2, 3, 4) and len(data) >= 8:
                tpid, ttid = struct.unpack_from("<II", data)
                tr.thread_pid[ttid] = tpid
            elif opcode == 72 and len(data) >= 8:  # SetName
                tpid, ttid = struct.unpack_from("<II", data)
                tr.thread_pid.setdefault(ttid, tpid)
                tr.thread_name[ttid] = data[8:].decode("utf-16-le", "replace").split("\0", 1)[0]

    buffers = [0]

    def on_buffer(_):
        buffers[0] += 1
        return 0 if max_buffers and buffers[0] >= max_buffers else 1

    cb, bcb = EVENT_RECORD_CALLBACK(on_event), BUFFER_CALLBACK(on_buffer)
    lf = EVENT_TRACE_LOGFILEW()
    lf.LogFileName = str(path)
    lf.ProcessTraceMode = PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_RAW_TIMESTAMP
    lf.EventRecordCallback = cb
    lf.BufferCallback = bcb
    handle = advapi.OpenTraceW(ctypes.byref(lf))
    if handle == INVALID_HANDLE:
        raise OSError(ctypes.get_last_error(), f"OpenTraceW a échoué sur {path}")
    tr.perf_freq = lf.LogfileHeader.PerfFreq
    tr.start_filetime = lf.LogfileHeader.StartTime
    h = ctypes.c_uint64(handle)
    try:
        err = advapi.ProcessTrace(ctypes.byref(h), 1, None, None)
        if err not in (0, 1223):  # 1223 : arrêt demandé par on_buffer
            raise OSError(err, "ProcessTrace a échoué")
    finally:
        advapi.CloseTrace(handle)
    return tr


if __name__ == "__main__":
    import time
    t0 = time.time()
    tr = read(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else None)
    print(f"{tr.events} événements en {time.time() - t0:.1f} s ; {len(tr.samples)} échantillons, "
          f"{len(tr.stacks)} piles, {len(tr.images)} images, {len(tr.thread_name)} noms de thread, "
          f"{len(tr.presents)} événements Present, "
          f"intervalle {tr.interval_100ns}")
