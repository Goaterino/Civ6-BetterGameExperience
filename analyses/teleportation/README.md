# Téléportation vers une ville : départ depuis un port

Analyse statique (2026-11-04) de `GameCore_XP2_FinalRelease.dll` (projet Ghidra `civ6`), en vue d'un
**correctif POL** : laisser une unité navale (l'amiral) repartir d'un port avec
`UNITOPERATION_TELEPORT_TO_CITY`, et pas seulement y arriver. Le correctif est dans le proxy
([proxy/src/ports.rs](../../proxy/src/ports.rs)).

## L'opération

`Unit_Operations_TeleportToCity.cpp`, vtable en `180a2d050` :

| entrée | adresse | rôle |
|---|---|---|
| `+0x10` | `18067d290` | `CanStart(op, unité, paramètres, contexte, bTestVisible, résultats)` |
| `+0x40` | `18067d900` | exécution : place l'unité (`1803b11b0`) puis `1803a3120` |
| `+0xc0` | `18067d5c0` | l'unité a-t-elle droit à l'opération ? |
| `+0xc8` | `18067d650` | la destination est-elle valide ? |

### Qui y a droit (`18067d5c0`)

- `UnitInfo+0x1b9 & 0x40` : les marchands (`MakeTradeRoute`) ;
- ou `unité+0x9d8`/`+0x9dc` différents de -1 ;
- ou la propriété de type `CAN_TELEPORT_TO_CITY` (`3f9140c1`, `~CRC32` comme `DB.MakeHash`), lue par
  `180625330(unité+0x98, hachage)`. Dans le jeu de base, seuls les héros du mode Héros l'ont
  (`TypeProperties`, `Babylon_Heroes_MODE.xml`), Sinbad compris. BBG ne la donne pas à l'amiral.

### Le départ (`18067d290`)

```
si bTestVisible : vrai                       // le bouton reste visible, grisé
unité+0x4f0 > 0                              // mouvements restants (1803a25f0)
objet = contexte->vtable[0x30]()             // ce qui est sous l'unité
18005e770(objet)                             // le district est-il DISTRICT_CITY_CENTER ?
si UnitInfo+0x1ba & 2 ou +0x1b9 & 0x40 : pas de contrôle de propriétaire
sinon objet+0x1c == unité+0x128              // district à soi
puis la destination (vtable +0xc8), ou la liste des destinations valides (hachage 7341ab55)
```

`18005e770` retrouve le district par `objet->vtable[8]({propriétaire objet+0x1c, genre 3, id objet+0x10})`
et compare le hachage de son type (`district+0xf0`, index dans les `DistrictInfo`) à `8a32007d`,
`DISTRICT_CITY_CENTER` (contrôlé dans `DebugGameplay.sqlite`).

### La destination (`18067d650`)

Case valide, différente de celle de l'unité, et :
- une ville du joueur sur la case ;
- **ou** un district du joueur sur la case, si `unité+0x2a0 == 0` (`DOMAIN_SEA`, même numérotation que
  `180191a50`) ;

puis un chemin possible (`1803b08e0`) et une limite de `GlobalParameters+0x58c`.

Une unité navale peut donc arriver dans un port, mais elle ne peut plus en repartir : sur le port,
`18005e770` répond non.

## Le correctif

Le seul `call 18005e770` de `18067d290` (en `18067d315`, entre `mov rcx,rax` et `test al,al`) est
redirigé vers un relais posé à moins de 2 Go de GameCore : `mov rdx,r14` (l'unité), puis
`jmp [rip+0]` vers `depart_valide(objet, unité)`. Cette fonction répond :
- comme `18005e770` si c'est un centre-ville ;
- sinon, si les correctifs POL sont actifs, oui pour une unité navale (`unité+0x2a0 == 0`) posée sur
  un district, quel qu'il soit : port, parc aquatique, canal… C'est la règle de la destination
  (`18067d650` : `FUN_180191b50(district) == unité+0x128`, sans condition de domaine).

La première version exigeait en plus un district dont le `MilitaryDomain` (`180191a50`) est
`DOMAIN_SEA`. En jeu (2026-11-05), l'amiral arrivait bien dans un parc aquatique mais ne pouvait pas
en repartir : le parc aquatique n'est pas `DOMAIN_SEA`. La condition a été retirée.

Le contrôle de propriétaire qui suit l'appel reste en place : on ne part que de ses propres districts.
Les règles de destination et l'exécution ne changent pas, et l'interface non plus : le bouton est déjà
visible, et `CanStartOperation` l'active.

`18005e770` sert à une centaine d'appelants : on ne le détourne pas, on ne redirige que cet appel.

## À vérifier en jeu

- Que l'objet de `contexte->vtable[0x30]` désigne bien le district sous l'unité quand elle est sur un
  port (l'analyse statique le laisse supposer, et le journal du proxy le dira :
  `ports : départ accepté depuis un district hors centre-ville`).
- L'amiral n'a pas l'opération sans `CAN_TELEPORT_TO_CITY`. Si on veut la lui donner :
  `INSERT INTO TypeProperties (Type, Name, Value) VALUES ('UNIT_GREAT_ADMIRAL', 'CAN_TELEPORT_TO_CITY', 'true');`
  (données de mod, chargées par tous les joueurs).
- Le rôle de `unité+0x9d8`/`+0x9dc` dans le droit à l'opération.
