"""Issues des missions d'espionnage et de l'évasion, recalculées depuis la table 3d6 de GameCore (voir README)."""
T=[0,0,0,1,3,6,10,15,21,25,27,27,25,21,15,10,6,3,1,0]
def f(lo,hi): return sum(T[i] for i in range(max(lo,0),min(hi,19)+1)) if lo<=hi else 0
cl=lambda v,a,b: max(a,min(b,v))
def outcomes(N):
    s5=cl(N,8,18); s4=cl(N-2,7,17); s3=cl(N-3,6,16); s2=cl(N-5,5,15); s1=cl(N-7,4,14)
    return [f(3,s1-1),f(s1,s2-1),f(s2,s3-1),f(s3,s4-1),f(s4,s5-1),f(s5,18)]
def escape(E): return [f(3,E-3),f(E-2,E-1),f(E,18)]
pct=lambda x: f"{100*x/216:5.1f}"
print("N  : tué capt échec+fuite échec succès+fuite succès | affiché")
for N in range(5,23):
    o=outcomes(N); print(f"{N:2d} :",*map(pct,o),"|",pct(o[4]+o[5]))
print("évasion E: tué capturé évadé")
for E in range(6,18): print(E,*map(pct,escape(E)))
print("\nB=14 : L | affiché | réel CS0 CS1 CS2 CS4 | butin sans police / police devinée (sans CS)")
for B in (13,14,15,16):
  for L in (1,2,3,4):
    N0=B-(L-1); o0=outcomes(N0)
    real=[o0]+[outcomes(N0+3+(c-1)) for c in (1,2,4)]
    E=10-(L-1)
    loot=[(o0[5]+o0[4]*escape(e)[2]/216) for e in (E,E+4)]
    print(B,L,"|",pct(o0[4]+o0[5]),"|",*[pct(o[4]+o[5]) for o in real],"|",*map(pct,loot))
