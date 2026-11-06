# BetterGameExperience (BGE)

[English](README.md)

Correctifs et accélérations pour **Civilization VI: Gathering Storm**, sous la forme de deux DLL
posées à côté des fichiers du jeu. Aucun fichier du jeu n'est remplacé : ni la vérification des
fichiers par Steam ni les mises à jour du jeu n'y touchent.

- Des tours plus rapides : les cascades de déclarations de guerre et les vérifications de l'IA qui
  alourdissent les gros tours en multijoueur sont regroupées et mises en cache, avec des résultats
  identiques à ceux du jeu de base.
- Des correctifs pour de vieux défauts du jeu : cases travaillées, boutons d'attaque cachés derrière
  les drapeaux d'unités, compteur de tours de production, merveilles retirées de la file, bandeau du
  haut figé, écran des émissaires, touche Entrée, et d'autres.
- **No First/Last moves** : une option de partie qui fait commencer et finir chaque tour au même
  instant pour tous les joueurs.
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

## En multijoueur

L'essentiel de BetterGameExperience ne concerne que votre machine : vous pouvez jouer avec des joueurs qui
ne l'ont pas, et rien ne se désynchronise.

Quelques correctifs changent les règles de la partie. Ils sont marqués **[tous les joueurs]** dans le
journal des modifications, et ne s'activent que si tous les joueurs humains ont la même version de
BetterGameExperience. Le salon le vérifie tout seul. Au lancement de la partie, un message `BGE -` dans le
chat dit si ces correctifs sont actifs, et qui n'a pas le mod. Les joueurs sans le mod lisent ces
messages en anglais. Ceux qui l'ont les voient dans leur langue.

Pour jouer avec **No First/Last moves (BGE)**, l'hôte coche l'option dans les options de partie
avant le lancement. Tous les joueurs doivent avoir BetterGameExperience, à la même version.

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
