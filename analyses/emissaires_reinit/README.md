# Écran des émissaires : les choix en attente s'effacent tout seuls

**Corrigé dans le proxy** (2026-11-07, interface seule, sans réglage) : `patch_city_states` dans
[proxy/src/ui.rs](../../proxy/src/ui.rs) ajoute [proxy/src/ui_emissaires.lua](../../proxy/src/ui_emissaires.lua)
à la fin de `CityStates.lua`. Confirmé en partie le 2026-11-08 (multijoueur, avec BSM) : les émissaires
en attente restent quand un autre joueur envoie les siens. Guerre par suzerain et déclaration de guerre
à une cité-État : pas encore essayées.

Question : pourquoi, pendant qu'on répartit ses émissaires (boutons + / − de l'écran des cités-États),
les choix pas encore confirmés disparaissent-ils d'un coup ? L'hypothèse : un autre joueur envoie des
émissaires au même moment. Analyse statique du 2026-11-07 des scripts Lua de Base, XP1, XP2 et de la
copie de Better Spectator Mod (BSM). Rien n'a été reproduit en partie : voir [À vérifier](#à-vérifier).

## Conclusion

**L'hypothèse est juste, mais ce n'est qu'une cause parmi d'autres.** L'écran se reconstruit
entièrement (`Refresh()`) à chaque événement de jeu qui touche l'influence, les quêtes ou les
cités-États, **quel que soit le joueur concerné**, et chaque reconstruction commence par effacer les
émissaires en attente :

```lua
function GetData()              -- Base/Assets/UI/PartialScreens/CityStates.lua:1411 (BSM :1450)
	...
	m_kCityStates = {};		-- Clear any previous data
	m_kPlayerData = {};
	m_kEnvoyChanges = {};       -- :1420 (BSM :1455) ← les choix en attente
```

`Refresh()` (`:579`) appelle `GetData()` puis `ViewList()`. Les choix du joueur n'existent que dans
`m_kEnvoyChanges`, une table locale du script : rien n'est envoyé à la partie avant
« Confirmer » (`OnConfirmPlacement`, `:748`, un `GIVE_INFLUENCE_TOKEN` par émissaire). Tout
`Refresh()` les perd donc.

Le défaut est dans le jeu de Firaxis, pas dans un mod : la copie de BSM a exactement les mêmes
gestionnaires. XP1 et XP2 enveloppent `GetData` (`CityStates_Expansion1.lua:43`, gouverneurs) sans
changer ce comportement.

## Événements qui déclenchent l'effacement

Abonnements dans `Initialize()` (`:1820-1830`). Tous appellent `Refresh()` si l'écran est visible, sans
regarder les arguments :

| Événement | Gestionnaire | Se déclenche pour un autre joueur ? |
|---|---|---|
| `Events.InfluenceGiven` | `OnInfluenceGiven` `:1721` | oui : chaque émissaire posé par n'importe qui |
| `Events.InfluenceChanged` | `OnInfluenceChanged` `:1708` | oui : voir ci-dessous |
| `Events.QuestChanged` | `OnQuestChanged` `:1743` | oui : `(fromPlayerID, toPlayerID)` |
| `Events.CityLiberated` | `OnCityLiberated` `:1665` | oui : n'importe quelle ville libérée |
| `Events.DiplomacyDeclareWar` | `OnDiplomacyDeclareWar` `:1678` | oui : `Refresh()` pour toute guerre, même entre deux autres joueurs |
| `Events.DiplomacyMakePeace` | `OnDiplomacyMakePeace` `:1693` | oui : idem pour la paix |
| `Events.LocalPlayerTurnBegin` / `TurnEnd` | `:1771` / `:1777` | non : joueur local seulement |

Preuves que ces événements ne sont pas filtrés sur le joueur local :
- `TutorialUIRoot.lua:2111` : `OnInfluenceChanged(player)` teste lui-même
  `if player == localPlayer` : l'événement arrive donc aussi pour les autres joueurs ;
- `CityBannerManager.lua:2075` : `OnQuestChanged(fromPlayerID, toPlayerID)` met à jour les bannières de
  toutes les cités-États, pour tous les joueurs ;
- en multijoueur, chaque client exécute la simulation : le `GIVE_INFLUENCE_TOKEN` d'un autre joueur est
  appliqué chez tout le monde, et les bannières des cités-États affichent ses émissaires. L'exe déclare
  bien les trois événements (`InfluenceGiven`, `InfluenceChanged`, `QuestChanged` dans
  `CivilizationVI.exe`) ; le site d'émission n'a pas été décompilé (voir À vérifier).

Ce qu'un autre joueur fait et qui efface les choix du joueur local, en jeu simultané :
1. **il confirme des émissaires** : `InfluenceGiven`, et `InfluenceChanged` pour lui (ses jetons
   baissent), plus `QuestChanged` si cela change une quête ou une suzeraineté ;
2. il gagne un émissaire (quête accomplie, district, merveille, politique…) : `InfluenceChanged`,
   `QuestChanged` ;
3. il déclare la guerre ou fait la paix avec n'importe qui, ou libère une ville ;
4. une quête de cité-État change pour n'importe qui (nouvelle quête, quête accomplie par un autre).

Le point 1 est le plus fréquent : en début de tour tout le monde a des émissaires à poser en même
temps. Plus il y a de joueurs, plus l'écran se vide souvent.

Côté joueur local, deux causes s'ajoutent :
- `LocalPlayerTurnEnd` puis `LocalPlayerTurnBegin` : finir son tour, ou l'annuler, vide l'écran ;
- confirmer une partie de ses émissaires vide le reste (attendu : `OnConfirmPlacement` les envoie tous).

Ce qui vide l'écran **volontairement** et doit le rester : `Close()` (`:441`), `OnConfirmPlacement`,
et la guerre / paix **du joueur local** avec une cité-État (`:1682`, `:1697`, la cité-État peut ne plus
accepter d'émissaires).

## Correctif

Règle voulue (2026-11-07) : les émissaires en attente vers une cité-État ne sont retirés que si :
- la cité-État est **détruite** ;
- l'**équipe du joueur local déclare explicitement la guerre** à cette cité-État.

Ils restent dans tous les autres cas, en particulier quand la cité-État entre en guerre contre le
joueur parce que son suzerain y est (« parti pris » dans une guerre entre deux civilisations
majeures). Fermer l'écran (`Close()`) les vide toujours, comme dans le jeu.

[proxy/src/ui_emissaires.lua](../../proxy/src/ui_emissaires.lua), ajouté après `Initialize();` (les
locales du fichier restent visibles ; `CityStates_Expansion1.lua` fait `BASE_GetData = GetData` après
l'`include`, donc il enveloppe la nouvelle version) :

| fonction | changement |
|---|---|
| `GetData` | garde `m_kEnvoyChanges` d'avant ; retire seulement les cités-États absentes de `GetAliveMinors` ou mortes ; borne le total aux jetons restants (sécurité : ils ne baissent que par la confirmation du joueur, qui vide déjà la liste) |
| `ViewList` | réaffiche le compteur de chaque ligne avec les émissaires en attente (`AddCityStateRow` ne montre que ceux posés) et l'en-tête |
| `RealizeEnvoyChangeButtons` | « − » reste actif sur une ligne qui a des émissaires en attente, même si la cité-État les refuse ; « Confirmer » ne s'active que s'il y a de quoi envoyer maintenant |
| `OnConfirmPlacement` | n'envoie que vers les cités-États qui acceptent (`CanGiveTokensToPlayer`) ; les autres restent en attente. Rebranché sur `ConfirmButton` |
| `OnDiplomacyDeclareWar` | ne vide plus tout : retire la seule cité-État cible (`reactingPlayer`) quand le déclarant (`actingPlayer`) est de l'équipe du joueur local. Rebranché sur l'événement |
| `OnDiplomacyMakePeace` | ne vide plus rien, rafraîchit seulement |

### Distinguer « explicite » et « parti pris »

`DiplomacyDeclareWar(actingPlayer, reactingPlayer)` (noms des arguments dans
`CityBannerManager.lua:2125`) est émis par `DoDeclareWar(déclarant, cible)` pour chaque paire de la
cascade ([rollover/cascade_guerre.md](../../rollover/cascade_guerre.md)) :
- **déclaration de l'équipe sur la cité-État** : `DeclareWar(A, CS)` donne `DoDeclareWar(x, CS)` pour
  chaque membre `x` de l'équipe de A : déclarant de l'équipe, cible la cité-État → **retiré** ;
- **parti pris du suzerain** : `1802ec560` fait `DeclareWar(CS, l'autre camp)`, raison `SUZERAIN_WAR` :
  la cité-État est le déclarant → **gardé**. Vrai que le joueur local déclare la guerre au suzerain ou
  l'inverse.

Cas limite non couvert : le pacte défensif (`1802acac0`, « si j a une alliance avec y →
DeclareWar(j, x) »). Si une cité-État déclare la guerre à un allié du joueur local par son suzerain,
et que l'alliance fait déclarer le joueur local à son tour, le déclarant est le joueur local : ses
émissaires en attente vers cette cité-État sont retirés. L'événement ne porte pas la raison ; le
distinguer demanderait de lire la raison dans GameCore. Hors de ce cas, les émissaires ne sont plus
retirés.

### Banc d'essai

Simulé en Lua 5.4 avec des imitations du jeu (`Players`, `Events`, `Controls` et les fonctions de
`CityStates.lua` réduites à leur effet sur `m_kEnvoyChanges`) : émissaire d'un autre joueur, guerre
entre d'autres, paix du joueur local, guerre par suzerain (gardé, « − » actif, seul l'envoyable part),
guerre du joueur ou d'un coéquipier à une cité-État (retiré), cité-État détruite (retirée), tous les
émissaires envoyés (écran fermé). Le test `cites_etats_de_bsm` vérifie le patch sur les fichiers du
jeu et de BSM.

## À vérifier

- Reproduire en multijoueur : ouvrir l'écran des émissaires, poser deux émissaires sans confirmer, faire
  confirmer un émissaire par un autre joueur. Attendu sans correctif : les deux choix disparaissent.
  Même essai avec une déclaration de guerre entre deux autres joueurs.
- Confirmer par un `print` dans `OnInfluenceGiven` / `OnInfluenceChanged` (Lua.log) que l'événement
  arrive bien chez le joueur local pour l'action d'un autre ; ou décompiler l'émission dans le projet
  Ghidra `civ6exe`.
- Après correctif : les choix restent, le compteur d'émissaires par cité-État et l'en-tête restent
  justes, et « Confirmer » envoie bien le nombre affiché. Déclarer la guerre au suzerain d'une
  cité-État où l'on a des émissaires en attente : ils restent, grisés à l'envoi, retirables par « − ».
- Ligne `ui : …CityStates.lua : émissaires en attente gardés…, compilation réussie` dans le journal du
  proxy.
