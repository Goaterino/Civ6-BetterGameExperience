# Revue de la garde des messages réseau (branche `yodha-desync-issue`)

Relecture du 2026-11-10 des deux commits de yodha sur `origin/yodha-desync-issue` (`e475b2a`, `46da9f6`,
partis de `f2ffb40`), avant leur merge dans main. La branche n'est pas encore mergée. Les points
Ghidra ont été vérifiés le même jour sur l'exe `CivilizationVI_DX12.exe` du 5 novembre 2026 (projet
local `~/Tools/ghidra-projects/civ6exe1105` ; le projet `civ6exe` est d'un exe antérieur, ses
adresses sont décalées) : voir [Vérifié dans Ghidra](#vérifié-dans-ghidra).

## Ce que fait la branche

**Garde contre le gel** (`startup_proxy/src/messages.rs`, posée par `DllMain` du proxy de démarrage).
Le 9 novembre 2026, à la déconnexion d'un joueur, un message compressé abîmé a été désérialisé quand
même : `TypedVariantMap::Deserialize` a lu un compte d'entrées de `0x61746143` (« Cata ») et a bouclé
dessus sans borne, jusqu'au manque de mémoire. Le proxy détourne `ReadSerializerField` (`14099f9d0`) ;
un bout d'assembleur (`bgd_messages_lecture_stub`) lui passe l'adresse de retour de l'appelant, et à
six sites du désérialiseur une valeur impossible est remplacée par 0 :

| Site | Adresse de retour | Règle |
|---|---|---|
| `DeserializeTypedVariantMap` (`1407a1e20`) : paires | `1407a1f44` | compte > 100 000 |
| `DeserializeTypedVariantMap` : entrées (la boucle du gel) | `1407a209f` | compte > 100 000 |
| `DeserializeNamedVariantMap` (`1407adf50`) : compte | `1407adfe3` | compte, sauf mot de drapeaux (bit 31) |
| `DeserializeNamedVariantMap` : compte après drapeaux | `1407ae03a` | compte > 100 000 |
| `DeserializeVariantList` (`1407ad890`) : compte | `1407ad901` | compte > 100 000 |
| `DeserializeDataVariant` (`1407a3450`) : longueur | `1407a34f4` | élément × longueur > 16 Mo (tampon brut, bit 24) ou > 4 096 (scalaire) |

Empreinte vérifiée octet par octet au démarrage : sur un autre exe (DX11, autre version), rien n'est
posé et le journal le dit. Rien n'est posé avec `actif=off` ; `garde_messages=off` coupe la garde,
`garde_messages=test` injecte une fois le compte du gel.

**Annonce dans le chat** (`partie.lua`, `Civ6Proxy_GardeMessages()`). Le joueur dont la garde a agi
envoie à tous `[COLOR_RED]BGE - Blocked a damaged network message that would have frozen your
game.[ENDCOLOR]`, au plus une ligne par minute ; chaque machine équipée le traduit à l'arrivée
(`BGD_TxtChat`, qui retire bien le préfixe coloré). Sans chat (solo, hotseat, PlayByCloud), le journal
seulement.

## Ce qui est vérifié

- **Merge à blanc avec main** (`c162017`, 30 commits d'écart) : seul conflit, `textes/bgd_textes.lua`,
  fichier généré, résolu en le régénérant. `gen_textes.py --check --strict` et tous les tests Lua
  passent sur le résultat.
- **Le bout d'assembleur** : alignement (`rsp ≡ 8` à l'entrée, `sub rsp,0x28`), cinquième argument à
  `[rsp+0x20]`, `rcx`/`rdx`/`r8`/`r9` intacts, directives SEH : correct.
- **Pas de verrou POL nécessaire** : la garde ne change pas la simulation, elle ne touche qu'un message
  qui aurait gelé le jeu.
- **Les adresses** : l'empreinte (prologue de la lecture, débuts des quatre fonctions, `ff 50 30`
  avant chacun des six retours) correspond octet par octet à l'exe DX12 installé (5 novembre 2026).
- **Pas vérifié** : la compilation Rust (aucune CI n'a tourné sur la branche).

## Vérifié dans Ghidra

**Les drapeaux sont bien à `dest + 4` : le code est juste.** `DeserializeDataVariant` (`1407a3450`)
lit d'abord les drapeaux dans `variant + 0xC` (appel de `1407a34ad`), puis la longueur dans
`variant + 8` (appel de `1407a34f1`, la destination vue par la garde). Les drapeaux sont donc à
`dest + 4`, et lus avant la longueur : `dest.add(4)` lit la bonne valeur. Seul le commentaire de
`Regle::Longueur` (« lue juste après ses drapeaux ») se lit comme une position en mémoire ; à
reformuler (« lue après ses drapeaux, qui la suivent en mémoire »). Le masque `0x5e000000` est exactement
celui des branches du jeu : bit 25 carte typée, 27 et 26 objets, 28 liste, 30 carte nommée ; bit 24
tampon brut ; sinon scalaire (8 octets si bit 31, 4 sinon).

Deux nuances, sans gravité :
- **Scalaire** : le jeu ne se sert pas de la longueur, il lit toujours 4 octets (`140099280`) ou 8
  (`1400992e0`). La règle « scalaire > 4 096 » ne protège donc d'aucune boucle ni allocation ; elle
  ne change que la longueur stockée d'un variant aberrant. Inoffensive, elle pourrait être retirée.
- **Tampon brut** : le jeu calcule `(drapeaux & 0xffffff) × longueur` sur 32 bits (débordement
  possible) ; la garde calcule sur 64 bits, elle est donc plus stricte. Si l'allocation échoue, le jeu
  saute les octets (`1407a3410`) ; une longueur remise à 0 ne saute rien.

**La signature de `ReadSerializerField` est confirmée** (`14099f9d0`) : quatre arguments (lecteur,
descripteur, destination, octets), rien sur la pile. Elle construit un petit tampon (`1409974d0`),
appelle la lecture virtuelle `+0x28` du lecteur, puis le détruit (`140997510`) ; sa valeur de retour
n'est pas utilisée par les appelants, le `usize` renvoyé par `lecture` est sans effet.

**Les exceptions : aucune levée dans le désérialiseur.** Des 308 appels à `_CxxThrowException`
(`140ea8bbc`), aucun n'est dans les fonctions de `Data_Variant.cpp` (`1407a…`) ni dans celles du
sérialiseur (`14099…`). Reste la lecture virtuelle `+0x28`, dont l'implémentation n'a pas été suivie.
`extern "C-unwind"` reste une assurance gratuite, plus un point bloquant.

**Ce que devient un message vidé : le contenu est remplacé, pas complété.** Avant de relire, la carte
typée libère toutes ses entrées et vide son arbre (`1407a1e20`), la carte nommée se vide (appel
virtuel `+0x160`, `1407adf50`), la liste aussi (`1407acea0`, `1407ad890`). Un compte remis à 0 donne
donc un objet vide. Et le compte remis à 0 ne fait pas sauter les octets qui suivaient : la suite du
message est lue décalée. Ces octets étaient déjà illisibles, et les comptes suivants restent bornés
par la garde ; ce n'est pas pire qu'avant, mais le message appliqué est faux, pas seulement vide.

**Reste ouvert** : si l'application d'un `NetPlayerInfo` ou d'un `NetGameConfig` vide (ou décalé)
remplace la configuration vivante du joueur. Il faut remonter les appelants de
`DeserializeNamedVariantMap` jusqu'au traitement du message. La machine touchée peut alors diverger
des autres, ce qui reste mieux qu'un gel.

## À corriger avant le merge

1. **Le mode `test` abîme un vrai message** : il injecte le compte du gel dans le premier vrai message
   « carte : entrées », que la garde vide ensuite (configuration de la partie au chargement, par
   exemple). En multijoueur, la machine qui l'a réglé diverge. À réserver aux builds `diag`, et à
   refuser hors partie solo.
2. **`garde_messages` absent de l'onglet d'options** : « Réinitialiser » ne le retire pas, et le test
   `cles_communes_avec_le_proxy` ne lit pas `messages.rs` (seulement `lib.rs`, `maj.rs`, `zones.rs`),
   donc ne le voit pas. À ajouter à l'onglet (`demarrage = true`), la valeur `test` en diagnostic, et
   `messages.rs` aux fichiers lus par le test.
3. **`extern "C-unwind"`** pour `lecture` et le type `Lecture` : aucune levée trouvée dans le
   désérialiseur, mais l'assurance ne coûte rien (comme les natives Lua du proxy de démarrage).
   Reformuler au passage le commentaire de `Regle::Longueur` (voir plus haut).
4. **Formulation** : « would have frozen **your** game » se lit, chez celui qui reçoit, comme sa
   propre partie. « my game » ou une tournure neutre ; la traduction française est déjà neutre.
5. **CI** : la faire tourner sur le résultat du merge (tests Rust de `startup_proxy` sous Windows)
   avant de pousser main.

## À essayer en partie

- Arrivée d'un joueur en cours de partie : la sauvegarde transite par le réseau, vérifier que les
  bornes (16 Mo, 100 000 entrées) ne touchent rien de légitime. Les essais de la mise au point (deux
  parties de dix joueurs, 162 tours, 108 000 messages sans fausse alerte) ne disent pas s'il y en a eu.
- Mode `test` en solo : le journal doit montrer l'injection, puis la correction.
