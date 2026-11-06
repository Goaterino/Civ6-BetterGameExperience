"""Retrouve dans un exe de Civ6 les adresses dont a besoin l'accélération des WMD (proxy/src/wmd.rs).

Après une mise à jour du jeu qui recompile l'exe, le test `wmd::tests::agencement_des_exe_installes`
échoue : ce script donne le nouvel agencement à ajouter à `EXES`, pour DX12 et DX11.

Chemin suivi, sans Ghidra :
- `SerialEvent::CombatVisBegin` / `End` : la chaîne est passée à une fonction de création, qui
  range l'événement dans un global ; la fonction de diffusion est celle dont le prologue
  (mov [rsp+8],rbx / push rdi / sub rsp,40h / mov rax,[rip+disp]) lit ce global ;
- `IsQuickCombat` : la closure Lua enregistrée sous ce nom appelle la configuration utilisateur,
  puis IsQuickCombat sur son résultat ;
- `timescale` : la commande de console écrit un flottant à `App+décalage` ; le constructeur de
  l'application pose 1.0 au même décalage, et son appelant range l'objet dans un global.

Utilisation (capstone et numpy dans le venv) :
    .venv/Scripts/python.exe -I proxy/tests/wmd_sites.py [<dossier Win64Steam>]
"""

import re
import struct
import sys

import capstone
import numpy as np

BASE = 0x140000000
GAME = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam"
DISPATCH = bytes([0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0x05])
USER_CONFIG = bytes([0x48, 0x83, 0xEC, 0x38, 0x48, 0x8B, 0x05])
IS_QUICK = bytes([0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20])


class Exe:
    def __init__(self, path):
        d = open(path, "rb").read()
        self.d = d
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        count = struct.unpack_from("<H", d, pe + 6)[0]
        opt = struct.unpack_from("<H", d, pe + 20)[0]
        self.secs = {}
        for i in range(count):
            n, vs, va, rs, ro = struct.unpack_from("<8sIIII", d, pe + 24 + opt + 40 * i)
            self.secs[n.rstrip(b"\0").decode()] = (va, vs, ro, rs)
        va, _, ro, rs = self.secs[".text"]
        self.tva, self.text = va, d[ro : ro + rs]
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
        self.md.detail = True
        t = np.frombuffer(self.text, dtype=np.uint8).astype(np.int64)
        n = len(t) - 4
        v = t[:n] | (t[1 : n + 1] << 8) | (t[2 : n + 2] << 16) | (t[3 : n + 3] << 24)
        v = np.where(v >= 2**31, v - 2**32, v)
        # Cible d'un disp32 relatif à RIP qui finirait l'instruction.
        self.targets = np.arange(n, dtype=np.int64) + self.tva + 4 + v
        p = self.secs[".pdata"]
        self.pdata = [struct.unpack_from("<II", d, p[2] + k) for k in range(0, p[1] - 11, 12)]

    def strings(self, s):
        out, i = [], self.d.find(s)
        while i != -1:
            for va, _, ro, rs in self.secs.values():
                if ro <= i < ro + rs:
                    out.append(i - ro + va)
            i = self.d.find(s, i + 1)
        return out

    def read(self, rva, n):
        for va, _, ro, rs in self.secs.values():
            if va <= rva < va + rs:
                return self.d[rva - va + ro : rva - va + ro + n]

    def function(self, rva):
        for b, e in self.pdata:
            if b <= rva < e:
                return list(self.md.disasm(self.text[b - self.tva : e - self.tva], b))
        raise LookupError(hex(BASE + rva))

    @staticmethod
    def rip(ins):
        for op in ins.operands:
            if op.type == capstone.x86.X86_OP_MEM and op.mem.base == capstone.x86.X86_REG_RIP:
                return ins.address + ins.size + op.mem.disp

    def xrefs(self, target):
        """Instructions relatives à RIP qui visent `target`, décodées dans leur fonction."""
        out = set()
        for tail in range(5):
            for i in np.nonzero(self.targets == target - tail)[0]:
                at = self.tva + int(i)
                try:
                    body = self.function(at)
                except LookupError:
                    continue
                out.update(ins.address for ins in body if ins.address <= at < ins.address + ins.size and self.rip(ins) == target)
        return sorted(out)

    def calls_to(self, target):
        hits = np.nonzero(self.targets == target)[0]
        return [self.tva + int(i) - 1 for i in hits if i > 0 and self.text[i - 1] == 0xE8]

    def at(self, body, rva):
        return next(k for k, ins in enumerate(body) if ins.address == rva)


def combat_vis(x, name):
    (s,) = x.strings(b"SerialEvent::" + name + b"\0")
    (ref,) = x.xrefs(s)
    body = x.function(ref)
    k = x.at(body, ref)
    create = next(ins for ins in body[k:] if ins.mnemonic == "call").operands[0].imm
    glob = next(x.rip(ins) for ins in x.function(create) if ins.mnemonic == "mov" and x.rip(ins) and ins.op_str.startswith("qword ptr [rip"))
    found = [r - 10 for r in x.xrefs(glob) if x.read(r - 10, 13) == DISPATCH]
    (at,) = found
    return at, x.read(at + 13, 4)


def quick_combat(x):
    (s,) = x.strings(b"IsQuickCombat\0")
    (ref,) = x.xrefs(s)
    body = x.function(ref)
    k = x.at(body, ref)
    closure = [x.rip(ins) for ins in body[k - 6 : k] if ins.mnemonic == "lea" and x.rip(ins)][-1]
    calls = [ins.operands[0].imm for ins in x.function(closure) if ins.mnemonic == "call"][:2]
    assert x.read(calls[0], len(USER_CONFIG)) == USER_CONFIG, "configuration utilisateur : prologue inattendu"
    assert x.read(calls[1], len(IS_QUICK)) == IS_QUICK, "IsQuickCombat : prologue inattendu"
    return calls


def application(x):
    (s,) = [r for r in x.strings(b"timescale\0") if x.xrefs(r)]
    body = x.function(x.xrefs(s)[0])
    offset = next(int(m.group(1), 16) for ins in body if ins.mnemonic == "movss" for m in [re.search(r"\[\w+ \+ (0x[0-9a-f]+)\], xmm", ins.op_str)] if m)
    # Le constructeur : `mov qword ptr [rdi + décalage], 0x3f800000`.
    pattern = re.compile(rf"qword ptr \[\w+ \+ {offset:#x}\], 0x3f800000")
    ctors = set()
    raw = struct.pack("<I", offset)
    i = x.text.find(raw)
    while i != -1:
        try:
            body = x.function(x.tva + i)
            if any(pattern.search(ins.op_str) for ins in body if ins.mnemonic == "mov"):
                ctors.add(body[0].address)
        except LookupError:
            pass
        i = x.text.find(raw, i + 1)
    (ctor,) = ctors
    globs = set()
    for c in x.calls_to(ctor):
        nxt = list(x.md.disasm(x.text[c - x.tva : c - x.tva + 32], c))
        globs.update(x.rip(ins) for ins in nxt[1:4] if ins.mnemonic == "mov" and ins.op_str.startswith("qword ptr [rip") and ins.op_str.endswith("rax"))
    (glob,) = globs
    return glob, offset


def main():
    folder = sys.argv[1] if len(sys.argv) > 1 else GAME
    for exe, kind in [("CivilizationVI_DX12.exe", "DX12"), ("CivilizationVI.exe", "DX11")]:
        x = Exe(f"{folder}\\{exe}")
        (begin, bd), (end, ed) = combat_vis(x, b"CombatVisBegin"), combat_vis(x, b"CombatVisEnd")
        cfg, quick = quick_combat(x)
        app, offset = application(x)
        disp = lambda b: ", ".join(f"{v:#04x}" for v in b)
        h = lambda r: f"0x1_{(BASE + r) >> 16 & 0xffff:04x}_{(BASE + r) & 0xffff:04x}"
        print(f"""    Exe {{
        name: "{kind}, build ?",
        vis_begin: ExeSite {{ name: "CombatVisBegin", address: {h(begin)}, expected: dispatch_prologue([{disp(bd)}]) }},
        vis_end: ExeSite {{ name: "CombatVisEnd", address: {h(end)}, expected: dispatch_prologue([{disp(ed)}]) }},
        user_config: {h(cfg)},
        is_quick_combat: {h(quick)},
        app: {h(app)},
        timescale: {offset:#x},
    }},""")


if __name__ == "__main__":
    main()
