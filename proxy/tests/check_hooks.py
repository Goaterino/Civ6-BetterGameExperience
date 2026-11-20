"""Test hors jeu du proxy de GameCore : exports, chargement de l'originale, détours.

On reproduit l'installation dans un dossier temporaire : le proxy sous le nom BGD_GameCore_XP2.dll,
à côté de l'originale sous son vrai nom, GameCore_XP2_FinalRelease.dll.

L'originale refuse de s'initialiser hors du jeu (erreur 1114). On la projette donc d'abord avec
DONT_RESOLVE_DLL_REFERENCES : le LoadLibraryW du proxy récupère alors ce module sans exécuter son
DllMain. Aucune fonction du jeu n'est appelée ; on relit les octets écrits à chaque site.

Usage (LOCALAPPDATA redirigé pour ne pas écrire dans le vrai journal) :
    python -I check_hooks.py <proxy compilé> <GameCore_XP2_FinalRelease.dll d'origine> <dossier temporaire>
"""
import ctypes
import os
import re
import shutil
import struct
import sys
from ctypes import wintypes

proxy_src, original_src, tmp = sys.argv[1:4]
game = os.path.join(tmp, "Win64")
os.makedirs(game, exist_ok=True)
proxy = os.path.join(game, "BGD_GameCore_XP2.dll")
original = os.path.join(game, "GameCore_XP2_FinalRelease.dll")
shutil.copyfile(proxy_src, proxy)
shutil.copyfile(original_src, original)
with open(os.path.join(game, "rollover_proxy.cfg"), "w") as f:
    f.write("mode=count\nportee=message\nameliorations=verify\nrendements=verify\n")
os.environ["LOCALAPPDATA"] = tmp
log_file = os.path.join(tmp, "Civ6RolloverProxy", "RolloverProxy.log")
if os.path.exists(log_file):
    os.remove(log_file)

# Game 1.0.12.74: same code at other addresses. The addresses below are those of the 2026-11-02
# build; R() gives the RVA in the build under test and K() the bytes read there as that build has
# them, from the tables of src/patch.rs.
_src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "patch.rs"), encoding="utf-8").read()
_num = lambda t: int(t.replace("_", ""), 16)
_block = lambda name: _src[_src.index("const " + name):_src.index("];", _src.index("const " + name))]
_moved = sorted((_num(a) - 0x180000000, _num(b) - 0x180000000) for a, b in re.findall(r"\((0x1_80[0-9a-f_]+), (0x1_80[0-9a-f_]+)\)", _block("MOVED")))
_fixups = {_num(a) - 0x180000000: (int(b, 16), int(c, 16)) for a, b, c in re.findall(r"\((0x1_80[0-9a-f_]+), (0x[0-9a-f]{2}), (0x[0-9a-f]{2})\)", _block("FIXUPS"))}
with open(original_src, "rb") as _f:
    _head = _f.read(0x400)
MOVED_BUILD = struct.unpack_from("<I", _head, struct.unpack_from("<I", _head, 0x3C)[0] + 8)[0] == 0x6AF3BCB5


def R(rva):
    if not MOVED_BUILD:
        return rva
    below = [m for m in _moved if m[0] <= rva]
    return rva + (below[-1][1] - below[-1][0]) if below else rva


def K(rva, data):
    if not MOVED_BUILD:
        return data
    out = bytearray(data)
    for i in range(len(out)):
        before, now = _fixups.get(rva + i, (None, None))
        if out[i] == now:
            out[i] = before
    return bytes(out)


k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.LoadLibraryW.restype = wintypes.HMODULE
k32.LoadLibraryExW.restype = wintypes.HMODULE
k32.LoadLibraryExW.argtypes = [wintypes.LPCWSTR, wintypes.HANDLE, wintypes.DWORD]

DONT_RESOLVE_DLL_REFERENCES = 0x1
base = k32.LoadLibraryExW(original, None, DONT_RESOLVE_DLL_REFERENCES)
assert base, f"projection de l'originale impossible : {ctypes.get_last_error()}"

h = k32.LoadLibraryW(proxy)
assert h, f"chargement du proxy impossible : {ctypes.get_last_error()}"
lib = ctypes.WinDLL(proxy)
status = lib.RolloverProxyStatus
status.restype = wintypes.DWORD
print(f"proxy chargé, état {status():#x} (1 = DllMain)")

load = lib.RolloverProxyLoadOriginal
load.restype = ctypes.c_size_t
got = load()
print(f"originale : {got:#x} (projection préalable en {base:#x}), état {status():#x} (2 = originale, 8 = détours)")
print("journal :\n  " + open(log_file, encoding="utf-8").read().strip().replace("\n", "\n  "))
assert got == base, "le proxy n'a pas récupéré l'originale"
assert status() & 0xB == 0xB, "les détours n'ont pas été posés"
assert status() & 0x10, "les correctifs POL n'ont pas été posés"

SITES = (  # nom, RVA, prologue d'origine
    ("instantané", 0x72F80, "48895c2410574883ec304863da8bf9"),
    ("diplomatie", 0xB7630, "48895c2410564883ec204863da8bf1"),
    ("DeclareWar", 0x2A1F90, "83faff0f840a020000488bc448894808"),
    ("passe", 0x4AFC0, "48895c24185556574154415541564157"),
    ("amélioration", 0x260720, "41564883ec304d8bf04d85c07509"),
    ("rendement", 0x37EC60, "44894424185553574154415541564157"),
    ("attente IA", 0x4A16E0, "40534883ec30488d05d34852"),
    ("citoyens", 0x10F050, "4c8bdc45894b204989531049894b08"),
    ("player connection change", 0x383E0, "4c8bdc5557498dab28ffffff4881ecc8010000"),
    ("slot status", 0x16B840, "48895c2408574883ec20488bd98bfa"),
    ("modificateur de production", 0x9497A0, "48895c2408574883ec404863da488bf9"),
    ("modificateur de production (ville, ajout)", 0x9444E0, "48895c2420574883ec50837930"),
    ("modificateur de production (ville, retrait)", 0x973C20, "48895c2420574883ec50837930"),
    ("promotion de gouverneur", 0x2DF340, "48895c240848896c2410488974241857"),
    ("retrait de gouverneur", 0x2DF800, "48895c2408488974241048897c2418"),
    ("établissement de gouverneur", 0x259B10, "4c8bdc4989732055498dabb8feffff"),
    ("personnage illustre", 0x655560, "4c8944241848894c2408555356574154"),
    ("touristes", 0x27CE10, "4053574883ec28488b0552dc9000"),
    ("demande de touristes", 0x27E960, "4057415641574883ec20488bfa4533ff"),
    ("vote sur une résolution", 0x3C0BA0, "48895c241048896c24184889742420"),
    ("vote sur une proposition", 0x3C0D40, "40555641574883ec304863ea458bf9"),
    ("dépouillement", 0x3C5F50, "48894c240855535657415441554156"),
    ("annulation pendant le congrès", 0x5B640, "8b057a61b00083f8ff74258bc8488b05"),
    ("envoi des messages (départ simultané)", 0x366B0, "4055535741564157488dac2450faffff"),
)
for name, rva, _ in SITES:
    patch = ctypes.string_at(base + R(rva), 14)
    assert patch[:6] == b"\xff\x25\x00\x00\x00\x00", f"{name} : pas de saut au site ({patch.hex()})"
    hook = struct.unpack("<Q", patch[6:])[0]
    assert h <= hook < h + 0x400000, f"{name} : le saut ne mène pas au proxy ({hook:#x})"
    print(f"{name} : site {base + R(rva):#x} -> détour {hook:#x} dans le proxy")

# Compteur de tours (tours.rs) : appels `call rel32` du cache de l'interface, redirigés vers un relais
# `jmp [rip+0]` posé près de GameCore, qui mène au proxy.
CALLS = (  # site, fonction appelée d'origine (RVA)
    (0x6B8DC, 0xFE9B0),
    (0x6C8D3, 0xFE9B0),
    (0x6C6AD, 0xFE300),
    (0x6C7B5, 0xFE300),
)
for rva, callee in CALLS:
    code = ctypes.string_at(base + R(rva), 5)
    assert code[0] == 0xE8, f"tours {rva:#x} : pas de call ({code.hex()})"
    stub = base + R(rva) + 5 + struct.unpack("<i", code[1:])[0]
    assert stub != base + R(callee), f"tours {rva:#x} : appel non redirigé"
    jump = ctypes.string_at(stub, 14)
    assert jump[:6] == b"\xff\x25\x00\x00\x00\x00", f"tours {rva:#x} : relais inattendu ({jump.hex()})"
    hook = struct.unpack("<Q", jump[6:])[0]
    assert h <= hook < h + 0x400000, f"tours {rva:#x} : le relais ne mène pas au proxy ({hook:#x})"
    print(f"tours : call {base + R(rva):#x} -> relais {stub:#x} -> {hook:#x} dans le proxy")

# Départ d'un port (ports.rs) : `call 18005e770` de CanStart (TeleportToCity) redirigé vers un relais
# `mov rdx,r14 ; jmp [rip+0]`.
PORT_RVA, PORT_CALLEE = 0x67D315, 0x5E770
code = ctypes.string_at(base + R(PORT_RVA), 5)
assert code[0] == 0xE8, f"ports : pas de call ({code.hex()})"
stub = base + R(PORT_RVA) + 5 + struct.unpack("<i", code[1:])[0]
assert stub != base + R(PORT_CALLEE), "ports : appel non redirigé"
jump = ctypes.string_at(stub, 17)
assert jump[:9] == bytes.fromhex("4c89f2ff2500000000"), f"ports : relais inattendu ({jump.hex()})"
hook = struct.unpack("<Q", jump[9:])[0]
assert h <= hook < h + 0x400000, f"ports : le relais ne mène pas au proxy ({hook:#x})"
print(f"ports : call {base + R(PORT_RVA):#x} -> relais {stub:#x} -> {hook:#x} dans le proxy")

# Unités empilées (empilement.rs) : `call 1802c4360` du contournement et du soutien redirigés vers un
# relais `mov r8,rsi ; mov r9,r14 ; jmp [rip+0]`.
PILE_RVAS, PILE_CALLEE = (0x36C3E1, 0x36D301), 0x2C4360
for rva in PILE_RVAS:
    assert ctypes.string_at(base + R(rva) - 10, 10).hex() == "8b5704488b88c8060000", f"empilement {rva:#x} : contexte inattendu"
    assert ctypes.string_at(base + R(rva) + 5, 6).hex() == "488bd84885c0", f"empilement {rva:#x} : suite inattendue"
    code = ctypes.string_at(base + R(rva), 5)
    assert code[0] == 0xE8, f"empilement {rva:#x} : pas de call ({code.hex()})"
    stub = base + R(rva) + 5 + struct.unpack("<i", code[1:])[0]
    assert stub != base + R(PILE_CALLEE), f"empilement {rva:#x} : appel non redirigé"
    jump = ctypes.string_at(stub, 20)
    assert jump[:12] == bytes.fromhex("4989f04d89f1ff2500000000"), f"empilement {rva:#x} : relais inattendu ({jump.hex()})"
    hook = struct.unpack("<Q", jump[12:])[0]
    assert h <= hook < h + 0x400000, f"empilement {rva:#x} : le relais ne mène pas au proxy ({hook:#x})"
    print(f"empilement : call {base + R(rva):#x} -> relais {stub:#x} -> {hook:#x} dans le proxy")

# Échange d'unités (echange.rs) : l'entrée de l'exécution de SWAP_UNITS dans sa vtable (`.rdata`)
# mène au proxy.
SWAP_SLOT, SWAP_EXEC = 0xA2CE28, 0x67C860
hook = struct.unpack("<Q", ctypes.string_at(base + R(SWAP_SLOT), 8))[0]
assert h <= hook < h + 0x400000, f"echange : entrée de vtable non détournée ({hook:#x})"
print(f"echange : vtable {base + R(SWAP_SLOT):#x} -> {hook:#x} dans le proxy")

# Passage de tour (fin_tour.rs) : l'étape de mise à jour `18063e2e0` en fin de DiplomacyAction,
# DiplomacyDeal (jmp terminal) et DiplomacySession (call) redirigée vers un relais
# `mov edx,quel ; jmp [rip+0]`.
FIN_APPELS, FIN_CALLEE = ((0x3514F, 0xE9), (0x353B8, 0xE9), (0x358F2, 0xE8)), 0x63E2E0
for i, (rva, opcode) in enumerate(FIN_APPELS):
    code = ctypes.string_at(base + R(rva), 5)
    assert code[0] == opcode, f"fin de tour {rva:#x} : pas de jmp/call ({code.hex()})"
    stub = base + R(rva) + 5 + struct.unpack("<i", code[1:])[0]
    assert stub != base + R(FIN_CALLEE), f"fin de tour {rva:#x} : appel non redirigé"
    jump = ctypes.string_at(stub, 19)
    assert jump[:11] == bytes([0xBA]) + struct.pack("<I", i) + bytes.fromhex("ff2500000000"), f"fin de tour {rva:#x} : relais inattendu ({jump.hex()})"
    hook = struct.unpack("<Q", jump[11:])[0]
    assert h <= hook < h + 0x400000, f"fin de tour {rva:#x} : le relais ne mène pas au proxy ({hook:#x})"
    print(f"fin de tour : {code[:1].hex()} {base + R(rva):#x} -> relais {stub:#x} -> {hook:#x} dans le proxy")

# Le proxy n'exporte que les noms de l'originale, plus DllMain, les deux exports de diagnostic et sa
# version (BgdVersion, lue par le proxy de démarrage).
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from gen_exports import exports  # noqa: E402

extra = set(exports(proxy)) - set(exports(original))
missing = set(exports(original)) - set(exports(proxy))
assert not missing, f"exports manquants : {sorted(missing)}"
assert extra == {"DllMain", "RolloverProxyStatus", "RolloverProxyLoadOriginal", "BgdVersion"}, f"exports en trop : {sorted(extra)}"
lib.BgdVersion.restype = ctypes.c_char_p
version = lib.BgdVersion().decode()
src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../src/version.rs"), encoding="utf-8").read()
assert f'VERSION: &str = "{version}";' in src, f"BgdVersion rend {version!r}, pas la VERSION de version.rs"
assert f"BgdVersion={version}\0".encode() in open(proxy, "rb").read(), "marque BgdVersion= absente du fichier"
print(f"version : {version} (export et marque du fichier)")
print(f"exports : les {len(exports(original))} de l'originale, plus {sorted(extra)}")

# Déchargement du proxy (l'originale reste) : les octets d'origine doivent revenir à chaque site.
k32.FreeLibrary.argtypes = [wintypes.HMODULE]
k32.GetModuleHandleW.restype = wintypes.HMODULE
while k32.GetModuleHandleW(proxy):
    k32.FreeLibrary(lib._handle)
for name, rva, prologue in SITES:
    now = K(rva, ctypes.string_at(base + R(rva), len(prologue) // 2)).hex()
    assert now == prologue, f"{name} : octets non restaurés après déchargement ({now})"
    print(f"{name} : octets d'origine restaurés après déchargement du proxy")
for rva, callee in CALLS:
    code = ctypes.string_at(base + R(rva), 5)
    assert base + R(rva) + 5 + struct.unpack("<i", code[1:])[0] == base + R(callee), f"tours {rva:#x} : appel non restauré"
print("tours : appels d'origine restaurés après déchargement du proxy")
code = ctypes.string_at(base + R(PORT_RVA), 5)
assert base + R(PORT_RVA) + 5 + struct.unpack("<i", code[1:])[0] == base + R(PORT_CALLEE), "ports : appel non restauré"
print("ports : appel d'origine restauré après déchargement du proxy")
for rva in PILE_RVAS:
    code = ctypes.string_at(base + R(rva), 5)
    assert base + R(rva) + 5 + struct.unpack("<i", code[1:])[0] == base + R(PILE_CALLEE), f"empilement {rva:#x} : appel non restauré"
print("empilement : appels d'origine restaurés après déchargement du proxy")
for rva, opcode in FIN_APPELS:
    code = ctypes.string_at(base + R(rva), 5)
    assert code[0] == opcode and base + R(rva) + 5 + struct.unpack("<i", code[1:])[0] == base + R(FIN_CALLEE), f"fin de tour {rva:#x} : appel non restauré"
print("fin de tour : appels d'origine restaurés après déchargement du proxy")
assert struct.unpack("<Q", ctypes.string_at(base + R(SWAP_SLOT), 8))[0] == base + R(SWAP_EXEC), "echange : entrée de vtable non restaurée"
print("echange : entrée de vtable d'origine restaurée après déchargement du proxy")
print("OK")
