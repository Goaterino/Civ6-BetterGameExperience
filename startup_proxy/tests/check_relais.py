"""Test hors jeu du proxy de démarrage : relais des exports de winmm et ligne de journal.

Le proxy est chargé par son chemin dans un dossier temporaire, sous le nom winmm.dll. Chaque export
de la winmm de System32 doit exister dans le proxy, et son relais (`jmp [rip+case]`) doit viser la
même adresse que la vraie : l'ordinal 2, sans nom, compris. On appelle aussi timeGetTime par le
proxy. Hors de l'exe du jeu, le proxy ne pose aucun détour.

Usage (LOCALAPPDATA redirigé pour ne pas écrire dans le vrai journal) :
    python -I check_relais.py <winmm.dll compilée> <dossier temporaire>
"""
import ctypes
import os
import shutil
import struct
import sys
from ctypes import wintypes

proxy_src, tmp = sys.argv[1:3]
game = os.path.join(tmp, "Win64Steam")
os.makedirs(game, exist_ok=True)
proxy = os.path.join(game, "winmm.dll")
shutil.copyfile(proxy_src, proxy)
os.environ["LOCALAPPDATA"] = tmp
log_file = os.path.join(tmp, "Civ6RolloverProxy", "RolloverProxy.log")
if os.path.exists(log_file):
    os.remove(log_file)

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.LoadLibraryW.restype = wintypes.HMODULE
k32.LoadLibraryW.argtypes = [wintypes.LPCWSTR]
k32.GetModuleHandleW.restype = wintypes.HMODULE
k32.GetModuleHandleW.argtypes = [wintypes.LPCWSTR]
k32.GetProcAddress.restype = ctypes.c_void_p
k32.GetProcAddress.argtypes = [wintypes.HMODULE, ctypes.c_void_p]
k32.GetSystemDirectoryW.argtypes = [wintypes.LPWSTR, wintypes.UINT]

assert not k32.GetModuleHandleW("winmm.dll"), "winmm déjà chargée dans ce processus : le test ne verrait pas le proxy"
h = k32.LoadLibraryW(proxy)
assert h, f"chargement du proxy impossible : {ctypes.get_last_error()}"
status = ctypes.WinDLL(proxy).StartupProxyStatus
status.restype = wintypes.DWORD
assert status() & 1, "DllMain du proxy non appelée"

buf = ctypes.create_unicode_buffer(260)
k32.GetSystemDirectoryW(buf, 260)
real = k32.GetModuleHandleW(os.path.join(buf.value, "winmm.dll"))
assert real and real != h, "la winmm de System32 n'est pas chargée à part"


def target(stub):
    """Adresse visée par un relais `jmp qword ptr [rip+disp]` (ff 25 disp32)."""
    code = ctypes.string_at(stub, 6)
    assert code[:2] == b"\xff\x25", f"relais inattendu : {code.hex()}"
    case = stub + 6 + struct.unpack("<i", code[2:])[0]
    return ctypes.c_uint64.from_address(case).value


names = [n.strip() for n in open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "winmm_exports.txt")) if n.strip()]
absents = []
for name in names:
    ours = k32.GetProcAddress(h, name.encode())
    assert ours, f"export {name} absent du proxy"
    theirs = k32.GetProcAddress(real, name.encode())
    if not theirs:
        absents.append(name)
        continue
    assert target(ours) == theirs, f"{name} : relais vers {target(ours):#x}, attendu {theirs:#x}"
ordinal = k32.GetProcAddress(h, 2)
assert ordinal and target(ordinal) == k32.GetProcAddress(real, 2), "ordinal 2"

ours_tgt = ctypes.WINFUNCTYPE(wintypes.DWORD)(k32.GetProcAddress(h, b"timeGetTime"))
theirs_tgt = ctypes.WINFUNCTYPE(wintypes.DWORD)(k32.GetProcAddress(real, b"timeGetTime"))
assert abs(ours_tgt() - theirs_tgt()) < 1000, "timeGetTime par le relais"

log = open(log_file, encoding="utf-8").read()
assert "démarrage : proxy winmm chargé dans" in log, log
assert "démarrage : exports relayés vers" in log, log
assert "chargeur :" not in log, "détour posé hors du jeu"
print(f"ok : {len(names) - len(absents)} exports relayés vers {buf.value}\\winmm.dll, ordinal 2 compris" + (f", absents de cette winmm : {absents}" if absents else ""))
