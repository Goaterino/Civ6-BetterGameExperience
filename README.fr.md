# BetterGameExperience (BGE)

[English](README.md)

Correctifs et accélérations pour **Civilization VI: Gathering Storm**, sous la forme de deux DLL
posées à côté des fichiers du jeu. Aucun fichier du jeu n'est remplacé : ni la vérification des
fichiers par Steam ni les mises à jour du jeu n'y touchent. La DLL de démarrage pose aussi, au
lancement du jeu, les images et sons des signaux d'équipe dans `DLC\BGEPings`.

- Des tours plus rapides : les cascades de déclarations de guerre et les vérifications de l'IA qui
  alourdissent les gros tours en multijoueur sont regroupées et mises en cache, avec des résultats
  identiques à ceux du jeu de base.
- Des correctifs pour de vieux défauts du jeu : cases travaillées, boutons d'attaque cachés derrière
  les drapeaux d'unités, compteur de tours de production, merveilles retirées de la file, bandeau du
  haut figé, écran des émissaires, touche Entrée, et d'autres.
- **No First/Last moves** : une option de partie, cochée par défaut, qui fait commencer et finir
  chaque tour au même instant pour tous les joueurs.
- Des mods d'interface populaires intégrés et corrigés : Better Report Screen, Extended Policy Cards,
  TopPanel Extension Pro, NotificationClear, Better Builder Charges Tracking.
- Fonctionne en DirectX 11 et DirectX 12, avec [Better Balanced Game](https://steamcommunity.com/sharedfiles/filedetails/?id=2865001760),
  Better Spectator Mod et MPH.
- Disponible dans les 11 langues de Better Balanced Game : English, Français, Deutsch, Español,
  Italiano, 日本語, 한국어, Polski, Português (Brasil), Русский et 简体中文.

La liste complète des changements est dans le [journal des modifications](changelog/fr.md), aussi
disponible en [English](CHANGELOG.md), [Deutsch](changelog/de.md), [Español](changelog/es.md),
[Italiano](changelog/it.md), [日本語](changelog/ja.md), [한국어](changelog/ko.md),
[Polski](changelog/pl.md), [Português (Brasil)](changelog/pt-BR.md), [Русский](changelog/ru.md) et
[简体中文](changelog/zh-CN.md).

## Installation

### Windows

1. Fermez Civilization VI.
2. Téléchargez `BetterGameExperience_V<version>.exe` depuis la [dernière release](https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest)
   et lancez-le.
3. Cliquez sur **Installer**. L'installeur trouve le jeu dans vos bibliothèques Steam, télécharge la
   release, et vérifie sa signature et le SHA-256 de chaque fichier avant de copier quoi que ce soit.
   Windows peut demander les droits d'administrateur si le jeu est dans `Program Files`.

C'est tout : lancez le jeu comme d'habitude. Un onglet **BetterGameExperience** apparaît dans les Options
du jeu.

**Mises à jour.** Quand une nouvelle version sort, le menu principal du jeu la propose. **Mettre à
jour** ferme le jeu, l'installe et propose de le relancer. Vous pouvez aussi relancer l'installeur à
tout moment : un installeur plus récent retire les anciens de son dossier.

**Désinstallation.** Lancez l'installeur et cliquez sur **Désinstaller**. Le jeu retrouve ses
fichiers d'origine.

### Linux (Steam Play / Proton)

BGE fonctionne avec la version Windows du jeu, lancée par Proton. La version Linux native de
Civilization VI n'est pas prise en charge.

1. Fermez Civilization VI.
2. Dans un terminal, lancez :
   ```sh
   curl -fsSL https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/install.sh | sh
   ```
   L'installeur trouve le jeu dans vos bibliothèques Steam, télécharge la release, et vérifie sa
   signature et le SHA-256 de chaque fichier avant de copier quoi que ce soit. Si l'installeur ne
   trouve pas le jeu, ajoutez cet argument à la fin de votre commande :
   `| sh -s -- --game-dir "/chemin/vers/Sid Meier's Civilization VI"`
3. Dans Steam, clic droit sur Civilization VI, puis **Propriétés** :
   - Dans **Général**, mettez dans **Options de lancement** :
     ```
     WINEDLLOVERRIDES="winmm=n,b" %command%
     ```
     Si le champ contient déjà du texte, mettez `WINEDLLOVERRIDES="winmm=n,b"` tout au début, avant
     les autres variables et avant `%command%`. Les arguments du jeu restent après `%command%`.
     Par exemple, `MANGOHUD=1 %command% -windowed` devient :
     ```
     WINEDLLOVERRIDES="winmm=n,b" MANGOHUD=1 %command% -windowed
     ```
     Sans cela, Proton prend sa propre `winmm.dll` et BGE ne se charge pas.
   - Dans **Compatibilité**, imposez une version de Proton pour le jeu.

C'est tout : lancez le jeu depuis Steam. Un onglet **BetterGameExperience** apparaît dans les Options
du jeu.

**Mises à jour.** Quand une nouvelle version sort, le menu principal du jeu la propose. **Mettre à
jour** ferme le jeu et installe la nouvelle version. Le jeu ne redémarre pas tout seul : une fois que
la notification du bureau indique que la mise à jour est installée, relancez le jeu depuis Steam. Si la
mise à jour échoue, un message d'erreur en donne la raison, et la version précédente reste en place.
Vous pouvez aussi relancer la commande d'installation à tout moment.

**Désinstallation.** Lancez la commande d'installation avec `uninstall` à la fin :
```sh
curl -fsSL https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/install.sh | sh -s -- uninstall
```
Le jeu retrouve ses fichiers d'origine, et l'installeur retire ses propres fichiers.

Pour un menu (état, réinstallation, canal pré-release), lancez
`~/.local/share/BetterGameExperience/BetterGameExperience` dans un terminal.

## En multijoueur

L'essentiel de BetterGameExperience ne concerne que votre machine : vous pouvez jouer avec des joueurs qui
ne l'ont pas, et rien ne se désynchronise.

Quelques correctifs changent les règles de la partie. Ils sont marqués **[tous les joueurs]** dans le
journal des modifications, et ne s'activent que si tous les joueurs humains ont la même version de
BetterGameExperience. Le salon le vérifie tout seul. Au lancement de la partie, un message `BGE -` dans le
chat dit si ces correctifs sont actifs, et qui n'a pas le mod. Les joueurs sans le mod lisent ces
messages en anglais. Ceux qui l'ont les voient dans leur langue.

L'un d'eux marche aussi entre versions différentes : quand un joueur se déconnecte, son emplacement
reste humain, l'IA ne prend donc pas le contrôle de ses unités, et l'hôte termine ses tours à sa place.
Chaque joueur humain doit avoir une version qui gère les déconnexions de la même façon (le journal
des modifications dit quand une version change cela). Jusqu'à la 0.15.0, il faut que tous aient la
même version, comme pour les autres correctifs.

**No First/Last moves (BGE)** est cochée par défaut dans les options de partie ; l'hôte peut la
décocher avant le lancement. Tous les joueurs doivent avoir BetterGameExperience, à la même version.

**Disable Advanced UI indicators (BGE)**, juste en dessous, est décochée par défaut. Quand l'hôte la
coche, aucun joueur ne voit l'avancement de la recherche et du dogme (**x/y (+z)**) ni la production
en réserve des villes, quels que soient ses propres réglages.

**Signaux d’équipe (BGE)** est désactivé par défaut pour les nouvelles parties en réseau. L’hôte peut
le cocher dans les options de partie. Tous les joueurs et observateurs connectés doivent avoir la
même version de salon BGE. Appuyez sur **X** pour marquer une case, ou maintenez **X**, déplacez la
souris et relâchez. En haut indique un danger, à droite « J’arrive », en bas un appel à l’aide et
à gauche un ennemi disparu. Les alliés voient le signal et le nom pendant quatre secondes.
Les raccourcis se modifient dans **Options → Raccourcis clavier**. Les observateurs voient les deux équipes. Le volume se règle dans **Audio**. Dans le chat,
`/mute nom` et `/unmute nom` contrôlent les messages et les signaux de ce joueur.

## Réglages

Tout se règle dans **Options → BetterGameExperience** : un réglage par fonctionnalité, et un interrupteur
général qui coupe tout le mod sans le désinstaller. Survolez un réglage pour voir ce qu'il fait. Les
réglages sont enregistrés dans `rollover_proxy.cfg`, à côté de `BGD_GameCore_XP2.dll`.

## Un problème ?

- Le journal est dans `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`. Joignez-le à tout
  signalement de bug, avec le `Lua.log` du jeu si le problème touche l'interface.
- Après une mise à jour du jeu, BetterGameExperience coupe de lui-même les fonctionnalités concernées
  jusqu'à sa propre mise à jour. Le journal le dit.
- Signalez les bugs et les corrections de traduction dans les
  [issues](https://github.com/Goaterino/Civ6-BetterGameExperience/issues). Le fonctionnement des
  traductions : [textes/README.md](textes/README.md).

## Crédits

Les mods d'interface intégrés reprennent le travail de leurs auteurs :

- Better Report Screen, Real Modifier Analysis et ReportsList Loader : Infixo. Onglets Unités et
  Accords d'origine : Greg Miller. Remerciements : nightfearer, Tyolan, copysoul, RohBang, Gorlod,
  Zergus, Rac, Aristos.
- Extended Policy Cards : Aristos.
- TopPanel Extension Pro : 号码菌 (Azukii). Remerciements : JNNR.
- NotificationClear : 336tarot.
- Better Builder Charges Tracking : wltk. Remerciements : DeepLogic, 小小雨10101.

## Pour les développeurs

L'index technique du dépôt est dans [SOMMAIRE.md](SOMMAIRE.md) : les proxys, la façon dont ils
s'accrochent au jeu, les analyses derrière chaque correctif, et comment compiler, tester et publier.
Les releases ne sont compilées et signées que par GitHub Actions ([release/README.md](release/README.md)).

Licence : [MIT](LICENSE).
