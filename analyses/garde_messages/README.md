# Revue de la garde des messages réseau (branche `yodha-desync-issue`)

Relecture du 2026-11-10 des deux commits de yodha sur `origin/yodha-desync-issue` (`e475b2a`, `46da9f6`,
partis de `f2ffb40`), avant leur merge dans main. La branche n'est pas encore mergée. Ce qui reste à
trancher demande le projet Ghidra `civ6-netcode` (signets « Gel 2026-11-09 ») et l'exe
`CivilizationVI_DX12.exe` du 5 novembre 2026 : voir [À vérifier dans Ghidra](#à-vérifier-dans-ghidra).

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
- **Pas vérifié** : la compilation Rust (aucune CI n'a tourné sur la branche, et le poste de la revue
  ne compile pas pour Windows) ; les adresses, faute d'exe sur ce poste.

## À vérifier dans Ghidra

**Où sont les drapeaux à la lecture de la longueur** (`DeserializeDataVariant`, retour `1407a34f4`).
Le commentaire de `Regle::Longueur` dit la longueur « lue juste après ses drapeaux (destination + 4) »,
ce qui met les drapeaux à `dest - 4` ; le code les lit à `dest + 4` (`dest.add(4)`). Il faut regarder
dans la fonction où l'appel de `1407a34f1` écrit la longueur, et où se trouve le mot de drapeaux lu
juste avant. Si le code se trompe, il lit autre chose que les drapeaux : un tampon brut légitime de
plus de 4 096 octets (chaîne, tableau) peut être pris pour un scalaire et sa longueur remise à 0, le
message étant alors vidé sans erreur. C'est le défaut le plus grave possible de la branche. Les tests
de `absurde` ne le voient pas : ils reçoivent les drapeaux en paramètre.

**Ce que devient un message vidé.** Le désérialiseur continue après un compte remis à 0 : le message
est appliqué, vide. Pour `NetPlayerInfo` et `NetGameConfig` (racines lues par
`DeserializeNamedVariantMap`), il faut savoir si l'application remplace la configuration ou la complète.
Si elle la remplace, la machine qui a reçu le message abîmé peut diverger des autres, ce qui reste
mieux qu'un gel, mais doit être su (et dit dans le chat).

**La signature de `ReadSerializerField`.** Le détour suppose quatre arguments (lecteur, descripteur,
destination, octets) et aucun sur la pile ; un cinquième ne serait pas transmis par `lecture` à la
lecture d'origine.

**Les exceptions.** `lecture` est `extern "C"` : si la lecture d'origine lève une exception C++ (fin de
flux, par exemple), elle ne peut pas traverser ce cadre Rust et le processus s'arrête. Voir si
`ReadSerializerField` ou ses appelants lèvent (`_CxxThrowException`) ; dans le doute, déclarer
`lecture` et le type `Lecture` en `extern "C-unwind"`, comme les natives Lua du proxy de démarrage.

## À corriger avant le merge

1. **Le mode `test` abîme un vrai message** : il injecte le compte du gel dans le premier vrai message
   « carte : entrées », que la garde vide ensuite (configuration de la partie au chargement, par
   exemple). En multijoueur, la machine qui l'a réglé diverge. À réserver aux builds `diag`, et à
   refuser hors partie solo.
2. **`garde_messages` absent de l'onglet d'options** : « Réinitialiser » ne le retire pas, et le test
   `cles_communes_avec_le_proxy` ne lit pas `messages.rs` (seulement `lib.rs`, `maj.rs`, `zones.rs`),
   donc ne le voit pas. À ajouter à l'onglet (`demarrage = true`), la valeur `test` en diagnostic, et
   `messages.rs` aux fichiers lus par le test.
3. **`extern "C-unwind"`** pour `lecture` (voir plus haut), sauf si Ghidra montre qu'aucune exception
   ne peut en sortir.
4. **Formulation** : « would have frozen **your** game » se lit, chez celui qui reçoit, comme sa
   propre partie. « my game » ou une tournure neutre ; la traduction française est déjà neutre.
5. **CI** : la faire tourner sur le résultat du merge (tests Rust de `startup_proxy` sous Windows)
   avant de pousser main.

## À essayer en partie

- Arrivée d'un joueur en cours de partie : la sauvegarde transite par le réseau, vérifier que les
  bornes (16 Mo, 100 000 entrées) ne touchent rien de légitime. Les essais de la mise au point (deux
  parties de dix joueurs, 162 tours, 108 000 messages sans fausse alerte) ne disent pas s'il y en a eu.
- Mode `test` en solo : le journal doit montrer l'injection, puis la correction.
