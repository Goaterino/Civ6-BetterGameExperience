"""Décode les messages NetGameConfig / NetPlayerInfo de net_message_debug.log.

Le jeu les journalise sous forme de TypedVariantMap imbriquées :
    CLE=Int(3), CLE=Char{texte}, CLE=TypedVariantMap(...), Unknown Variant Key(<hachage>)=...

Le jeu coupe chaque ligne de journal à 32 Ko : un NetGameConfig de partie à 4 joueurs et
cités-États l'est presque toujours, et perd ses clés de partie, écrites en dernier. Le décodeur
rend ce qui a pu être lu et marque la table coupée d'une clé '…tronqué'.

Les clés de mods sont nommées d'après noms_cles.tsv (préfixe ~), sinon gardées en #<hachage>.

Usage :
    python -I parse_netconfig.py LOG                # liste les messages (numéro de ligne)
    python -I parse_netconfig.py LOG 34             # arbre JSON du message de la ligne 34
    python -I parse_netconfig.py LOG 34 56          # différences entre deux messages
"""
import json
import re
import sys

TYPE = re.compile(r'[A-Za-z0-9_]+')


def charger_noms():
    """noms_cles.tsv (gen_noms_cles.py) : hachage -> nom des clés que le jeu ne nomme pas."""
    import pathlib
    f = pathlib.Path(__file__).with_name('noms_cles.tsv')
    if not f.exists():
        return {}
    return {l.split('	')[0]: l.split('	')[1].strip() for l in f.read_text(encoding='utf-8').splitlines()}


NOMS = charger_noms()


class Coupe(Exception):
    pass


def lire_table(s, i, out):
    """Remplit `out` avec les paires CLE=VALEUR jusqu'à ')'. Rend l'indice après ')'."""
    n = len(s)
    while True:
        while i < n and s[i] in ' ,':
            i += 1
        if i >= n:
            raise Coupe
        if s[i] == ')':
            return i + 1
        if s.startswith(('TypedVariantMap(', 'TypedVariantVector('), i):  # élément de liste, sans clé
            cle = f'[{len(out)}]'
            i = lire_valeur(s, i, out, cle)
            continue
        if s.startswith('Unknown Variant Key(', i):
            j = s.find(')', i)
            if j < 0:
                raise Coupe
            h = s[i + 20:j]
            cle = '~' + NOMS[h] if h in NOMS else '#' + h
            i = j + 1
        else:
            j = s.find('=', i)
            if j < 0:
                raise Coupe
            cle = s[i:j]
            i = j
        if i >= n or s[i] != '=':
            raise Coupe
        i += 1
        k, base = 2, cle
        while cle in out:
            cle = f'{base}~{k}'
            k += 1
        i = lire_valeur(s, i, out, cle)


def lire_valeur(s, i, out, cle):
    m = TYPE.match(s, i)
    if not m:
        raise Coupe
    typ, i = m.group(), m.end()
    if i >= len(s):
        raise Coupe
    if s[i] == '{':  # Char{...}, qui peut contenir des accolades (JSON des titres de mods)
        prof, j = 0, i
        while j < len(s):
            if s[j] == '{':
                prof += 1
            elif s[j] == '}':
                prof -= 1
                if prof == 0:
                    break
            j += 1
        else:
            raise Coupe
        out[cle] = s[i + 1:j]
        return j + 1
    if s[i] != '(':
        raise Coupe
    if typ in ('TypedVariantMap', 'TypedVariantVector'):
        out[cle] = {}
        try:
            return lire_table(s, i + 1, out[cle])
        except Coupe:
            out[cle]['…tronqué'] = True
            raise
    j = s.find(')', i)
    if j < 0:
        raise Coupe
    brut = s[i + 1:j]
    if typ in ('Int', 'UInt'):
        out[cle] = int(brut) if brut else None
    elif typ == 'Bool':
        out[cle] = brut == 'true'
    elif typ == 'Char':
        out[cle] = brut
    else:
        out[cle] = f'{typ}({brut})'
    return j + 1


def message(ligne):
    m = re.search(r'(NetGameConfig|NetPlayerInfo) : (m_playerID\((\d+)\), )?fromCloud\(\d\), ', ligne)
    if not m:
        return None, None
    d = {}
    try:
        lire_table(ligne + ')', m.end(), d)
    except Coupe:
        d['…tronqué'] = True
    return m.group(1), d


def aplatir(d, prefixe=''):
    for k, v in d.items():
        if isinstance(v, dict):
            yield from aplatir(v, prefixe + k + '.')
        else:
            yield prefixe + k, v


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    lignes = open(sys.argv[1], encoding='utf-8', errors='replace').read().split('\n')
    nums = [int(x) for x in sys.argv[2:]]
    if not nums:
        for n, l in enumerate(lignes, 1):
            if 'NetGameConfig :' in l or 'NetPlayerInfo :' in l:
                coupe = ' [tronqué]' if len(l) >= 32000 else ''
                print(n, l[:100] + coupe)
        return
    msgs = [message(lignes[n - 1])[1] for n in nums]
    if len(msgs) == 1:
        print(json.dumps(msgs[0], indent=1, ensure_ascii=False))
        return
    a, b = (dict(aplatir(m)) for m in msgs[:2])
    for k in sorted(set(a) | set(b)):
        if a.get(k, '<absent>') != b.get(k, '<absent>'):
            print(f'{k}: {a.get(k, "<absent>")!r} -> {b.get(k, "<absent>")!r}')


if __name__ == '__main__':
    main()
