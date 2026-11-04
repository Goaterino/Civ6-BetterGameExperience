# Civ6BetterGameDLLs

Modding de Civilization VI par DLL : un proxy de `GameCore_XP2_FinalRelease.dll` qui pose des
détours dans GameCore et dans l'exe, et les analyses qui l'ont motivé. Séparé de Civ6SimSolver
(le solveur hors partie) le 2026-10-30, avec l'historique des fichiers concernés.

- [proxy](proxy/README.md) : la DLL proxy (Rust), son installation et ses réglages
  (`rollover_proxy.cfg`) ;
- [rollover](rollover/README.md) : mesure du passage de tour (capture WPR, lecteur ETL, analyse) et
  [cascade d'une déclaration de guerre](rollover/cascade_guerre.md) ;
- [nuke](nuke/README.md) : animation des WMD (trace Frida, mod artdef, accélération dans le proxy) ;
- [ui_strike](ui_strike/README.md) : boutons d'attaque des bannières, patchés depuis le proxy ;
- [production_turns](production_turns/README.md) : compteur de tours de production qui ignore les
  bonus plats de gouvernement (fascisme), corrigé dans le proxy ;
- [wonder_queue](wonder_queue/README.md) : merveille retirée de la file de production après deux
  échanges, rattrapée par l'interface du proxy sans désynchro ;
- [empire_yields](empire_yields/README.md) : rendements d'empire du bandeau figés après un gouverneur ou
  un personnage illustre, rafraîchis par le proxy sans désynchro ;
- [map_pins](map_pins/README.md) : lag des épingles de carte en multijoueur, reconstructions inutiles
  retirées par l'interface du proxy ;
- [citoyens](citoyens/README.md) : choix des cases travaillées qui préfère une 2/1 à une 2/2,
  corrigé dans le proxy (premier correctif POL, actif si tous les joueurs ont le proxy) ;
- [mods_ui](mods_ui/README.md) : mods d'interface du Workshop (Better Report Screen, Extended Policy
  Cards, TopPanel Extension Pro, NotificationClear, Better Builder Charges Tracking) repris dans le
  proxy, corrigés (RMA) et prioritaires sur le mod ; tours restants des routes commerciales ;
- [pantheon](analyses/pantheon/README.md) : joueur désigné pour le panthéon, égalités et restriction d'un tour
  (analyse statique) ;
- [congres](analyses/congres/README.md) : congrès mondial en multijoueur, triplets divergents, tours passés et
  votes ignorés (analyse préliminaire statique) ;
- [espions](analyses/espions/README.md) : chances de succès annoncées sans le contre-espion du défenseur, et
  succès avec fuite comptés comme des succès (analyse statique) ;
- [lobby](analyses/lobby/README.md) : création du salon multijoueur, format de la config, presets MPH
  réappliqués par chaque machine, et swaps d'emplacement (réseau compris, courses pendant une arrivée) ;
  hook Frida et décodeur de config (analyse statique) ;
- [spectateurs](spectateurs/README.md) : spectateurs de BSM hors du tourisme et du vote du congrès,
  qu'ils peuvent toujours consulter (correctif POL) ;
- [netcode](netcode/README.md) : reconnexions et boucles de désynchro (hooks Frida, empreintes), analyse dans
  [docs/netcode](docs/netcode/README.md) ;
- [docs/gamecore_xp2](docs/gamecore_xp2/README.md) : carte des fichiers source de GameCore_XP2.

## Installation

Prérequis : Rust (`stable-x86_64-pc-windows-gnu` suffit), Python 3.10+.

```bash
python -m venv .venv
.venv/Scripts/python.exe -m pip install frida
```

Les commandes des README se lancent depuis la racine du dépôt.
