# Mods d'interface repris dans le proxy : analyse et conception

Cinq mods du Workshop, présents dans nos parties, à reprendre dans le proxy de GameCore en options
activables. Si un mod reste activé, **la version du proxy prime** : c'est l'occasion de corriger
leurs défauts, à commencer par RMA, qui calcule mal ou pas du tout l'effet de certaines cartes. Analyse statique des fichiers des mods, comparés à
ceux du jeu, et du journal `ui_journal=on` d'une partie avec les quatre premiers mods et BSM (novembre 2026).
**Implémenté le 3 novembre 2026, pas encore essayé en jeu** : voir
[proxy/README.md](../proxy/README.md#mods-dinterface-repris) pour le code, les réglages et les tests,
et [À vérifier en jeu](#à-vérifier-en-jeu) plus bas.

| mod | Workshop | id du mod | taille | ce qu'il apporte |
|---|---|---|---|---|
| Extended Policy Cards (Aristos) | 2266952591 | `382a187f-c8ba-4094-a6a7-0d5315661f33` | 2 fichiers remplacés, diff de 30 lignes | effet chiffré de chaque carte de politique, sur la carte et dans l'infobulle |
| TopPanel Extension Pro (号码菌 / Azukii) | 2997927787 | `9E881F4F-017F-29F5-D849-04537424BE3F` | 600 lignes Lua, 1 contexte, 7 textes × 12 langues | nourriture, production, population et luxes dans le bandeau ; ressources des alliés |
| Better Report Screen (Infixo) | 1312585482 | `6f2888d4-79dc-415f-a8ff-f9d81d7afb53` | ≈ 8 000 lignes Lua, XML de 2 300 lignes, base de données, 56 textes × 12 langues, icônes | écran des rapports refait, 5 onglets de plus, analyse des modificateurs (RMA) |
| NotificationClear (336tarot) | 3063966792 | `ae166b3b-fe6b-4848-8a66-a48d91bc2966` | 80 lignes Lua, 1 texte | bouton « tout effacer » dans le panneau des notifications |
| Better Builder Charges Tracking (wltk) | 2409116842 | `c6477d9f-6bad-4d24-9e76-49cda4f0a966` | 60 lignes Lua, aucun texte en jeu | charges restantes des bâtisseurs sur leur drapeau |

Les cinq sont des mods d'interface (`AffectsSavedGames=0`) : ils ne changent aucune règle, et ne
demandent donc **pas** de correctif POL. Les reprendre dans le proxy évite surtout que l'hôte impose
à tous les joueurs de s'abonner aux mêmes mods.

## Ce que fait chaque mod

### Extended Policy Cards

- **Fichiers.** Remplace `GovernmentScreen.lua` et `GovernmentScreen.xml` (`ImportFiles`, même nom
  que ceux de `Base/Assets/UI/Screens`). Le reste est la copie à l'identique du fichier du jeu.
- **Lua** (deux ajouts, marqués `-- ARISTOS`) :
  - en tête : `isBRSActive = Modding.IsModActive(<BRS>)` et `RMA = ExposedMembers.RMA` ;
  - à la fin de la fonction qui remplit une carte (après `cardInstance.Background:SetTexture(…)`) :
    `RMA.CalculateModifierEffect("Policy", policyType, 0)` donne le texte de l'effet, mis dans
    l'infobulle et dans un nouveau cadre en bas de la carte.
- **XML** : un `Grid ID="EffectContainer"` (et son `Label ID="Effect"`) ajouté au modèle de carte,
  après la balise `NewIcon`.
- **Dépend de Better Report Screen** : sans lui, `isBRSActive` est faux et le mod ne fait rien. Tout le
  calcul est dans `RealModifierAnalysis.lua` de BRS.

### TopPanel Extension Pro

- **Fichiers.** `ReplaceUIScript` du contexte `TopPanel` par `TopPanelExtension.lua`, qui inclut
  `TopPanel_Expansion2` (donc toute la chaîne du jeu), puis :
  - enveloppe `RefreshYields` : quatre boutons de plus, nourriture (excédent réel, modificateurs de
    croissance compris), production, population (croissance par tour) et luxes possédés ;
  - remplace `RefreshResources` (copie de celle de XP2) : ressources stratégiques des coéquipiers dans
    l'infobulle, et ressources affichées dès qu'un coéquipier les voit ;
  - enveloppe `LateInitialize` (équipes, liste des ressources, `ResearchCompleted` / `CivicCompleted`).
- **Infobulles.** Un contexte de plus, `TopPanel_TT` (`AddUserInterfaces` dans `InGame`) :
  3 `ToolTipType` et leurs instances dans `TopPanel_TT.xml`, remplis par `TopPanel_TT.lua` sur
  `LuaEvents.TopPanelToolTip_*_Refresh`.
- **Textes** : 7 clés, en 12 langues.
- **Défaut constaté** : son `ReplaceUIScript` (LoadOrder 10000) passe devant celui de BSM. Dans nos
  parties, `TopPanel_Spectator.lua` n'est donc **jamais chargé** (journal : `TopPanelExtension.lua`
  puis la chaîne XP2, sans le fichier de BSM).

### Better Report Screen

- **Remplacements.**
  - `ReportScreen.lua` et `ReportScreen.xml` (même nom que ceux du jeu) ;
  - le script du contexte `ReportScreen` (`ReplaceUIScript`, LoadOrder 1020), qui saute
    `ReportScreen_Expansion1/2` ;
  - le script de `ReportsList` par `ReportsListLoader.lua` (RLL), qui construit la liste des boutons
    depuis la table `RLLReports`.
- **Fichiers nouveaux** : 9 pages `BRSPage_*.lua`, incluses par `include("BRSPage_…")` depuis
  `reportscreen.lua`, et un contexte `RealModifierAnalysis` (RMA, 2 946 lignes) ajouté à `InGame`.
  RMA ne touche pas `ContextPtr` : il publie ses fonctions dans `ExposedMembers.RMA` (dont
  `CalculateModifierEffect`, dont se sert Extended Policy Cards) et marque son cache sale sur une
  trentaine d'événements.
- **Base de données** : `GlobalParameters` (`BRS_VERSION_*`, `BRS_OPTION_MODIFIERS=0`), table
  `RLLReports` (créée par le mod), raccourci F8 (`InputActions` `ToggleReports`, au menu principal),
  56 textes en 12 langues, dont 27 cités dans le XML (`String="LOC_BRS_…"`).
- **Icônes** : `FontIconsRMI.dds` et `UpdateIcons` redéfinissent 15 icônes de police (`DISTRICT_CANAL`,
  `DISTRICT_GOVPLAZA`, `PressureUpRed`, `VoteUp`…).
- **Commande de jeu** : la page des unités envoie `UnitManager.RequestCommand(unit, UPGRADE)`, une
  commande ordinaire, rejouée par tous.
- **Avec BSM** : le script du contexte est celui de BSM (`ReportScreen_Spectator.lua`), qui inclut
  `ReportScreen_Expansion2` → `_Expansion1` → `ReportScreen`, résolu en celui de BRS. Les
  surcharges de XP2 passent donc par-dessus BRS, ce que BRS évite sans BSM.

### NotificationClear

- **Fichier** : `NotificationPanel_QuickClear.lua`, inclus par le joker
  `include("NotificationPanel_", true)` en fin de `NotificationPanel.lua`, juste avant `Initialize()`.
- **Effet** : enveloppe `OnDefaultAddNotification` et `ReleaseNotificationEntry`. Une instance
  générique du panneau sert de bouton (icône `QueueTrash`) ; un clic appelle
  `NotificationManager.Dismiss` sur chaque notification que `CanUserDismiss()` autorise.
- **Défauts** : les notifications sont suivies dans un anneau de 100 identifiants par joueur, qui
  peut en perdre ; `toolTip` fuit en global. `NotificationManager.GetList(playerID)` donnerait la
  liste exacte.

### Better Builder Charges Tracking

- **Fichier** : `ReplaceUIScript` du contexte `UnitFlagManager` (LoadOrder 666) par
  `UnitFlagManager_BuilderCharges.lua`. Celui-ci inclut `UnitFlagManager_BarbarianClansMode.lua`
  s'il définit `Initialize`, sinon `UnitFlagManager.lua` : avec BSM, c'est celui de BSM, importé sous ce
  nom.
- **Effet.** `UnitFlag.UpdatePromotions` est surchargée pour `UNIT_BUILDER` et
  `UNIT_MILITARY_ENGINEER` : le macaron des promotions (`Promotion_Flag` / `UnitNumPromotions`, dans
  le XML du jeu) montre `unit:GetBuildCharges()` au lieu du nombre de promotions.
- **Mise à jour.** `Subscribe` / `Unsubscribe` sont enveloppées pour suivre
  `Events.UnitChargesChanged`. Elles sont appelées depuis `OnInit`, après le chargement de toute la
  chaîne : la surcharge passe donc à temps.
- **Défauts :**
  - **Pas de remise à zéro.** L'originale commence par `Promotion_Flag:SetHide(true)`, pas la version
    du mod quand l'unité est un bâtisseur. Sur un drapeau recyclé par l'`InstanceManager`, un
    bâtisseur sans charge garderait le macaron de l'unité précédente.
  - **Liste en dur.** La légion romaine (`UNIT_ROMAN_LEGION`, 1 charge dans `Units.xml`) n'est pas
    couverte, ni les unités de mods qui ont des charges. Pour une unité qui a à la fois des charges et
    des promotions, il faut choisir quoi montrer.
  - **Unités étrangères.** Les charges des bâtisseurs des autres joueurs sont affichées aussi, ce que le
    jeu ne montre nulle part. À garder ou non : c'est un choix, pas une faute.
  - **Script de contexte remplacé.** Un autre mod qui fait aussi un `ReplaceUIScript` de
    `UnitFlagManager` est évincé, ou évince celui-ci, selon les LoadOrder.
- **Dans nos parties** : pas encore activé. Le journal ne montre que `UnitFlagManager.lua` de BSM,
  déjà patché par le proxy (clic sur un drapeau en visée, `patch_unit_flags`).

## Ce que le proxy sait déjà faire, et ce qui manque

L'accroche `luaL_loadbuffer` ([proxy/src/ui.rs](../proxy/src/ui.rs)) voit le chemin complet de chaque
script compilé, et la lecture XML celui de chaque XML. Elle sait réécrire un fichier qui existe ou y
ajouter un bloc (`ui_merveilles`, `ui_rendements`). Ces mods demandent en plus :

| besoin | mods | solution proposée |
|---|---|---|
| charger des fichiers qui n'existent pas dans le jeu | BRS (pages, RMA), TPE (infobulles) | **fichiers embarqués** : `Civ6Proxy_Charger(nom)`, fonction native posée dans la table globale du contexte (comme `Civ6RolloverProxy_Clock`). Elle compile le texte embarqué par le vrai `luaL_loadbuffer`, sous le nom `@<nom>.lua`, et rend la fonction. Le Lua l'appelle, ou lui donne son propre environnement (`setfenv`) pour RMA. HavokScript exporte `lua_pcall`, `lua_cpcall` et `hksi_lua_getfenv` |
| un contexte de plus | TPE (`TopPanel_TT`), BRS (`RealModifierAnalysis`) | pas de contexte neuf. Les `ToolTipType` de TPE vont à la fin de `TopPanel.xml`, et son Lua dans le contexte `TopPanel`. RMA est chargé dans un environnement isolé par le premier contexte qui en a besoin (`GovernmentScreen` ou `ReportScreen`). Un drapeau dans `ExposedMembers` évite un second chargement |
| textes absents de la base | les quatre | **table de textes embarquée**, générée depuis les XML des mods (toutes leurs langues, l'anglais en dernier recours). Lua : nos copies appellent `BGD_Lookup` au lieu de `Locale.Lookup`, sans toucher à la table `Locale` partagée. `Locale.Lookup` sur un texte brut le met en forme (`{1_Num}`), comme le jeu le fait déjà. XML : les `String="LOC_…"` sont remplacés à la lecture du fichier, dans la langue rendue par `Localization::GetCurrentLanguage()` → `Language::GetType()`, deux exports de `Localization_FinalRelease.dll` |
| `GlobalParameters`, `GameInfo.RLLReports` | BRS | remplacés dans nos copies : `BRS_OPTION_MODIFIERS` → `0`, version → constante, liste des rapports écrite en dur dans le bloc ajouté à `ReportsList.lua` |
| raccourci F8 | BRS | `Input.GetActionId("ToggleReports")` rend une valeur nulle sans le mod. Pas de raccourci dans un premier temps ; ensuite, peut-être une touche lue dans l'accroche d'entrée d'`InGame` |
| icônes de police | BRS | impossible sans le `.dds` du mod. Les 15 icônes sont remplacées dans nos copies par des icônes du jeu, ou retirées |

La lecture XML n'est détournée aujourd'hui que si `ui_boutons` ou `ui_journal` est actif : il faudra
l'étendre aux nouveaux réglages. Les branchements de `hook_loadbuffer` (une suite de `if`) gagneraient à
devenir une table `(fichier, réglage, correctif)`, comme `FIXES`.

## Intégration, mod par mod

Principe commun : **le proxy prime**. Réglage actif, le proxy neutralise les fichiers du mod au moment
où l'exe les compile ou les lit, et met les siens à la place. Réglage inactif, il ne touche à rien :
le mod fonctionne comme avant.

- **Reconnaître un fichier du mod.** Par son nom et une marque de son contenu, pas par le chemin :
  un mod peut aussi être installé à la main dans `Documents\My Games\…\Mods`, sous un autre dossier.
  Marques : `-- ARISTOS` (EPC), `TPE_BASE_RefreshYields` (TPE), `Better Report Screen` en première
  ligne (BRS), `NPQC_` (NotificationClear), `OnUnitChargesChanged` (Better Builder Charges
  Tracking).
- **Trois façons de neutraliser.**
  - **Vider** un fichier que le mod ajoute : son texte devient un bloc vide, ou `<Context/>` pour un
    XML. Exemples : pages de BRS, RMA du mod, contexte d'infobulles de TPE, fichier de
    NotificationClear.
  - **Rendre le fichier du jeu** quand le mod remplace un fichier du même nom, puis y appliquer notre
    patch. Le texte d'origine est relu sur le disque, depuis le dossier du jeu (trois niveaux au-dessus
    de l'exe).
  - **Rendre le script de contexte du jeu** quand le mod fait un `ReplaceUIScript`. Notre texte n'est
    alors qu'un `include` de ce que le jeu aurait chargé.
- **Le journal :** `ui : <fichier> du mod <nom> neutralisé, remplacé par la reprise du proxy`.
- **Ce que le mod ajoute en base reste là** (textes, `RLLReports`, raccourci F8, icônes). C'est sans
  effet sur nos fichiers, qui ont leurs propres textes, et parfois utile : avec BRS actif, F8 et les
  icônes de police existent, et notre écran des rapports s'en sert.
- **Multijoueur :** si l'hôte impose un mod, les joueurs sans proxy ont la version du mod, et ceux qui
  l'ont la version du proxy. C'est sans risque : interface seulement.

### `ui_politiques` : Extended Policy Cards

- **Lua** : deux insertions dans `GovernmentScreen.lua` du jeu, quel que soit le script du contexte
  (celui du jeu ou `governmentscreen_spec.lua` de BSM, qui inclut la chaîne) :
  - en tête, le chargement de RMA (`Civ6Proxy_Charger`) ;
  - après `cardInstance.Background:SetTexture(GetPolicyBGTexture(policyType));` dans la fonction
    qui remplit une carte, l'effet de la carte.
- **XML** : l'`EffectContainer` après la balise `NewIcon` de `GovernmentScreen.xml`.
- **Mod présent** : le `GovernmentScreen.lua` et le `GovernmentScreen.xml` du mod (marque `ARISTOS`)
  sont remplacés par ceux du jeu, relus sur le disque, puis patchés comme ci-dessus. BSM y passe
  aussi, puisque `governmentscreen_spec.lua` inclut la chaîne jusqu'à `GovernmentScreen`.
- **Garde-fous** : chaque ancre doit être trouvée une fois exactement, sinon le fichier est laissé tel
  quel.
- **Mieux que le mod** : ne dépend plus de BRS, puisque le proxy apporte RMA lui-même, et corrigé (voir
  [RMA](#rma--les-effets-mal-calculés)).
- **Test hors jeu** : patch du vrai fichier, `luac -p`, XML bien formé (comme `top_panel_du_jeu`).

### `ui_notifications` : NotificationClear

- **Lua** : un bloc inséré dans `NotificationPanel.lua`, juste après `include("NotificationPanel_",
  true);`, à l'endroit où le joker aurait chargé le mod.
- **Réécrit plutôt que copié** : effacement sur `NotificationManager.GetList`, sans anneau.
- **Mod présent** : `NotificationPanel_QuickClear.lua` (marque `NPQC_`) est vidé, sans quoi le joker
  le chargerait après notre bloc et aurait deux boutons.
- **Texte de l'infobulle** : en français et en anglais, comme l'onglet BetterGameExperience.

### `ui_charges` : Better Builder Charges Tracking

- **Lua** : un bloc ajouté à la fin de `UnitFlagManager.lua`, celui du jeu ou celui de BSM, après le
  correctif des clics (`patch_unit_flags`), dans le même passage de l'accroche.
  - Il enveloppe `UnitFlag.UpdatePromotions` et `Subscribe` / `Unsubscribe`, comme le mod.
  - Ajouté au fichier de base, il passe avant les surcharges du mode Clans barbares et des scénarios :
    celles-ci gardent la nôtre comme fonction d'origine.
- **Corrigé :**
  - le macaron est d'abord masqué, comme dans l'originale ;
  - toute unité dont le type a `BuildCharges > 0` est couverte, pas seulement deux types écrits en dur.
    Pour la légion, les charges sont montrées tant qu'il en reste, puis les promotions.
- **Unités étrangères** : leurs charges sont montrées, comme le mod.
- **Couverture** : avec BBG et BBG Expanded, sept types ont des charges (bâtisseur, ingénieur militaire,
  légion romaine, toa maori, nau portugaise, jahazi, traîneau de Thulé).
- **Mod présent** : `UnitFlagManager_BuilderCharges.lua` (marque `OnUnitChargesChanged`) devient
  `include("UnitFlagManager_BarbarianClansMode.lua")` ou `include("UnitFlagManager.lua")`, avec le
  même choix que le mod. Notre bloc fait le reste.

### `ui_bandeau` : TopPanel Extension Pro

- **XML** : les `ToolTipType` et `Instance` de `TopPanel_TT.xml` ajoutés à la fin de `TopPanel.xml`.
- **Lua** : les surcharges de TPE doivent passer après celles de XP2 et de BSM. Le bloc est donc
  ajouté à la fin du script du contexte, le plus externe de la chaîne. Le proxy le reconnaît : c'est
  le premier script compilé après la lecture de `TopPanel.xml` (`mods_ui::xml_lu`). À la fin de ce
  script, toute la chaîne est chargée : le bloc enveloppe les fonctions définitives (`RefreshYields`,
  `LateInitialize`), avant l'initialisation du contexte.
- **`RefreshResources` de TPE** est une copie de celle de XP2, avec les ressources stratégiques des
  coéquipiers. Si le fichier de BSM est chargé, son `RefreshResources` reste (bandeau du joueur
  observé par un spectateur). Pour les autres joueurs, il appelle la version de XP2 qu'il a gardée
  dans `XP2_RefreshResources` : c'est celle-là qui est remplacée. Le test porte sur
  `XP2_RefreshResources`, pas sur BSM actif : avec TPE présent, BSM est actif mais son fichier est
  perdu. Jusqu'au 6 novembre 2026, la surcharge était sautée dès que BSM était actif : plus de
  stratégiques des coéquipiers dans les parties avec BSM.
- **Infobulles** : les gestionnaires de `TopPanel_TT.lua` vivent dans le même contexte, appelés
  directement, sans `LuaEvents`. Les noms des `ToolTipType` sont préfixés (`BGD_…`), pour ne pas
  heurter ceux du mod.
- **Mod présent** :
  - `TopPanelExtension.lua` devient le chargement de la chaîne du jeu : `TopPanel_Spectator` si BSM
    est actif, puis `TopPanel_Expansion2`, jusqu'à ce que `Initialize` soit défini, comme TPE.
  - BSM ne fait que remplacer le script du contexte, sans rendre son fichier disponible à `include`.
    Avec TPE actif, `include("TopPanel_Spectator")` ne trouve rien : le bandeau de BSM reste perdu,
    comme avec le mod (premier essai en jeu, 3 novembre 2026 : sans repli, plus aucun fichier du
    bandeau n'était chargé et le bandeau était cassé).
  - `TopPanel_TT.lua` est vidé. `TopPanel_TT.xml` reste : ses types d'infobulles ont d'autres noms que
    les nôtres.
- **Mieux que le mod** : sans TPE, le bloc est ajouté à `TopPanel_Spectator.lua`, et BSM garde son
  bandeau.
- **Ajout : activités moyennes par ville** (novembre 2026, pas encore essayé en jeu). Le bouton de
  la population montre, à la place de la population et de sa croissance, la moyenne des activités
  nettes des villes (obtenues moins requises, la colonne des activités de son infobulle), arrondie à
  l'entier inférieur : « +2 », « 0 », « −1 » ; vert dès +3 (ville en extase), rouge sous 0. Son
  infobulle ne change pas (population totale, puis chaque ville : population, logements, activités).
  Un luxe gagné ou perdu en cours de tour ne redessinait que les ressources : `PlayerResourceChanged`
  (joueur local) demande aussi les rendements. D'après un module séparé (`AmenitiesTopPanel_custom`),
  même calcul ; il détaillait les villes dans une infobulle à lui, en double de celle de la population.

### `ui_rapports` : Better Report Screen

Le gros morceau : ≈ 530 Ko de Lua et de XML embarqués.

- **Lua** :
  - `ReportScreen.lua` du jeu : son texte est remplacé par celui de BRS. Les `include("BRSPage_…")`
    deviennent `BGD_Inclure("BRSPage_…")`, et les lectures de `GlobalParameters` des constantes ;
  - `ReportScreen_Expansion1/2.lua` : réduits à leur `include`, comme le `ReplaceUIScript` de BRS.
    Avec BSM non plus, les surcharges de XP2 ne passent donc plus par-dessus BRS. C'est l'intention de
    BRS, même si ce n'est pas ce qu'on voyait jusqu'ici avec BSM.
- **XML** : `DLC/Expansion2/UI/Replacements/ReportScreen.xml` remplacé par celui de BRS, ses 27
  `LOC_BRS_…` traduits à la lecture.
- **`ReportsList.lua`** : bloc ajouté qui redéfinit `LateInitialize`, depuis `OnInit` comme RLL.
  - Il prend la liste de BRS, écrite en dur.
  - Si `GameInfo.RLLReports` existe, il y ajoute les lignes de cette table : les rapports d'autres mods
    qui passent par RLL ne sont pas perdus.
- **RMA** : une copie privée par contexte (`BGD_ChargerRMA`), dans l'écran des rapports comme dans
  celui du gouvernement. Elle n'est pas mise dans `ExposedMembers.RMA`, que le RMA du mod pourrait
  écraser, et ne meurt pas avec un autre contexte.
- **Mod présent.** Tout est neutralisé :
  - `ReportScreen.lua` et `ReportScreen.xml` du mod : remplacés par les nôtres.
  - `BRSPage_*.lua` : jamais inclus, puisque notre texte charge les pages embarquées.
  - `RealModifierAnalysis.lua` : vidé.
  - `ReportsListLoader.lua` : devient `include("ReportsList")`, et notre bloc fait le reste.
  - La table `RLLReports`, F8 et les icônes de police du mod restent en base : notre écran s'en sert
    quand elles sont là (`Input.GetActionId("ToggleReports")` non nul).
- **Pertes connues sans le mod** : le raccourci F8 et les 15 icônes de police.

### `ui_routes` : tours restants des routes commerciales

Même méthode que les charges, et même bloc ([drapeaux.lua](../proxy/src/mods/drapeaux.lua)) : sur le
drapeau d'un négociant du joueur local en route, le macaron montre le nombre de tours avant la fin de
sa route. Son infobulle le rappelle, et ajoute que le négociant rentre ensuite à sa ville d'origine.

- **Le Lua n'en sait rien.** `GetOutgoingRoutes()` ne donne que les villes, le négociant et les
  rendements. C'est une copie de 0x58 octets que fait l'instantané de la ville, sans le tour de départ.
- **GameCore le sait.** Analyse statique (projet Ghidra `civ6xp2`) :
  - `TRADE_ROUTE_TURN_DURATION_BASE` (20) est rangé en `+0x714` des paramètres globaux ;
  - seules deux fonctions le lisent, dont `18038f720(gestionnaire, départ)`, qui rend la fin de la
    durée minimale (voir [La durée minimale](#la-durée-minimale-dune-route)) ;
  - `18066bf70`, la fin de trajet d'un négociant, termine la route quand le négociant est à sa ville
    d'origine et que le tour courant (`[180b8aa68] + 0xc0`) atteint ce tour de fin ;
  - le départ est dans la fiche de la route, en `+0x4`. À la création de la route (`1803913f0`),
    `180394510` y écrit le tour courant, et en `+0x0` un numéro d'ordre des routes. Premier essai
    en jeu : `+0x20`, lu d'abord, valait −1 ; les fiches lues donnaient des départs aux tours 78 et
    88 pour une partie au tour 92 ;
  - `180386480(gestionnaire + 0xc0, &origine, &destination)` trouve cette fiche, avec le négociant en
    `+0x8` (joueur) et `+0xc` (unité), comme le vérifie `180386730`. Les clés sont
    `{ 0, joueur, ville }` ;
  - `18038f6c0()` rend le gestionnaire de commerce.
- **`Civ6Proxy_FinRoute`** (fonction native) appelle ces trois fonctions avec les identifiants de
  `GetOutgoingRoutes()`. Elle vérifie que la fiche est bien celle du négociant, et rend la fin de la
  durée minimale et le tour de départ.
- **La fin réelle vient au retour du négociant.** Le Civilopédie le dit : la route dure au moins
  20 tours à vitesse normale, et ne se termine qu'une fois le négociant revenu sur la case de sa ville
  d'origine. Sur une route, il avance d'une case par tour. Le bloc compte donc :
  - L, le nombre de pas du trajet, par `TradeManager:GetTradeRoutePath` (comme l'écran de choix des
    routes) : un aller-retour dure 2 × L tours ;
  - le sens, d'après le temps écoulé depuis le départ, et la place du négociant sur le trajet ;
  - le premier retour chez lui qui tombe à la fin de la durée minimale ou après.
#### La durée minimale d'une route

Rien n'est écrit en dur, sauf la valeur de secours de 20 si le paramètre manque. Tout vient de la
base de jeu, donc de BBG ou d'un autre mod s'ils la changent :

```
fin minimale = départ + ⌊TRADE_ROUTE_TURN_DURATION_BASE × CostMultiplier / 100⌋
                      + ⌊TradeRouteMinimumEndTurnChange × CostMultiplier / 100⌋     (au moins départ + 1)
```

- **`TRADE_ROUTE_TURN_DURATION_BASE`** : table `GlobalParameters`, 20. Rangé en `+0x714` des
  paramètres globaux (`1801f02a0`).
- **`CostMultiplier`** : table `GameSpeeds`, ligne de la vitesse de la partie.
  - Valeurs : 50 en ligne, 67 rapide, 100 standard, 150 épique, 300 marathon ; 100 si la vitesse est
    introuvable.
  - Le chargeur (`18075bc40`, `SELECT rowid, CivicUnlockMaxCost, CivicUnlockMinCost,
    CivicUnlockPerTurnDrop, CostMultiplier, …`) la range en `+0x28`. La mise à l'échelle
    (`18036f620`) la lit là, et tronque le résultat.
- **`TradeRouteMinimumEndTurnChange`** : table `Eras_XP2`, ère courante de la partie.
  - Valeurs : 0 pour l'Antiquité et l'ère classique, 10 pour l'ère médiévale et la Renaissance, 20
    pour les ères industrielle, moderne et atomique, 30 pour l'ère de l'information et l'ère future.
  - L'ère vient de l'objet de la partie (`1801e5460`, `+0xb8`) ; sa ligne dans `Eras`, par son type,
    mène à celle d'`Eras_XP2`.
  - Le chargeur (`18075aac0`) range la colonne en `+0x30`, avec en `+0x2c` un indicateur « non NULL ».
    Une ère sans valeur n'ajoute rien.

| vitesse | Antiquité, classique | médiévale, Renaissance | industrielle à atomique | information, future |
|---|---|---|---|---|
| en ligne (50) | 10 | 15 | 20 | 25 |
| rapide (67) | 13 | 19 | 26 | 33 |
| standard (100) | 20 | 30 | 40 | 50 |
| épique (150) | 30 | 45 | 60 | 75 |
| marathon (300) | 60 | 90 | 120 | 150 |

- **Vérifié sur le journal** : toutes les routes de la partie en ligne du 3 novembre 2026, au tour 91,
  avaient 15 tours de durée minimale (départ 89, fin 104 ; départ 67, fin 82…).
- **L'ère est lue au moment du calcul**, quand le négociant rentre, et non au départ de la route. Un
  changement d'ère en cours de route en change donc la fin. Le compteur appelle la même fonction à
  chaque rafraîchissement, et suit ce changement. Confirmé dans GameCore : `18038f720` n'a qu'un
  appelant, `18066bf70`, l'étape de mission qui teste le retour du négociant. Il lui passe le départ
  rangé dans les données de la mission (+0x20), et l'ère courante est relue à chaque appel.
- **Non résolu, vu en jeu le 3 novembre 2026** : les mêmes routes, relues après un passage de la
  Renaissance à l'ère industrielle, avaient 20 tours de durée minimale au lieu de 15. Un négociant à
  1 tour de chez lui affichait 27 tours (un aller-retour de plus) ; la route a pourtant pris fin au
  tour suivant. GameCore ne semble donc pas allonger les routes en cours comme le laisse penser
  `18066bf70` (l'ère vue au retour, ou le départ de la mission en +0x20, diffèrent peut-être de ceux
  que lit le compteur). Pas reproduit depuis : à reprendre si le cas revient.

#### Le décompte

- **Premier décompte, essayé en jeu le 3 novembre 2026.** Il ne montrait que la durée minimale : les
  routes longues restaient à 0 pendant que le négociant rentrait. Une route partie au tour 67, avec une
  fin minimale au tour 82, était encore active au tour 91.
- **Le rafraîchissement.** Le bloc rafraîchit les drapeaux des négociants à chaque tour, et quand une
  route commence ou finit. `TradeRouteAddedToMap` ne suffisait pas : un négociant qui venait de partir
  restait sans compteur jusqu'au tour suivant. Le drapeau d'un négociant du joueur local est donc aussi
  rafraîchi quand il bouge (`UnitMoveComplete`, `UnitMovementPointsChanged`, `UnitOperationStarted`,
  chacun seulement s'il existe).
- **Limite.** Le trajet est recalculé à chaque lecture. S'il a changé depuis le départ (routes,
  frontières), le décompte peut se décaler d'un aller-retour près.
- **Garde-fous.** Les octets des trois fonctions sont vérifiés au chargement (aussi par
  `cargo test`, `sites_de_commerce`). Sinon, pas de tours, et le journal le dit.
- **Risque.** La lecture se fait depuis le thread de l'interface, pendant que GameCore peut travailler
  sur son propre thread. Elle ne fait que lire, et seulement aux rafraîchissements ; mais une route
  créée ou supprimée au même instant n'est pas exclue.

### RMA : les effets mal calculés

RMA (`CalculateModifierEffect`) décode chaque modificateur de la carte : son `EffectType`, ses
arguments et ses prérequis. Il chiffre ensuite le gain sur l'état du joueur. Quand l'effet ou un
prérequis n'est pas géré, il rend un texte vide, et la carte n'affiche rien. Quand il est mal géré, le
chiffre est faux. Le corriger dans notre copie profite aux deux écrans, cartes et rapports.

- **Inventaire**, avant toute correction : pour chaque politique de la base de jeu (BBG compris), lister
  ses modificateurs, leur `EffectType` et leurs `RequirementType`, puis les marquer « géré » ou « non
  géré » d'après les tables de décodage de RMA. On peut le faire hors jeu, sur `DebugGameplay.sqlite`
  du cache du jeu.
- **Faux chiffres** : comparer en jeu, carte par carte, l'aperçu au gain réel mesuré (rendements de
  l'empire avant et après la pose).
- **Tests hors jeu** : les fonctions de décodage sont du Lua pur sur des tables. On peut les nourrir
  avec des lignes de la base extraites en fixtures, comme les tests Lua existants du proxy.

#### Les cartes signalées (diagnostic du 3 novembre 2026)

Modificateurs lus dans `DebugGameplay.sqlite` du cache (BBG 7.5 et BBG Expanded chargés). Le code est
celui de `RealModifierAnalysis.lua` de BRS 7.0.

| carte | modificateurs (BBG) | ce que fait RMA | correction |
|---|---|---|---|
| **Raj** | 6 × `EFFECT_ADJUST_CITY_STATE_TRADE_ROUTE_FLAT_YIELD` : +1 nourriture, +1 production, +2 or, science, culture et foi par route commerciale vers une cité-État. BBG a remplacé l'effet d'origine, par suzerain (`…_PER_TRIBUTARY`, que RMA gère) | effet inconnu : rien | gérer l'effet : montant × nombre de routes du joueur vers une cité-État |
| **Grands personnages** (Inspiration…) | les `BBG_POLICY_*_FOOD_*` de nourriture ont un **prérequis de propriétaire** `REQSET_CVS_LEADER_IS_TAINO` (dirigeant Anacaona) | `DecodeModifier` affiche l'`OwnerRequirementSet` mais **ne le vérifie jamais** : la nourriture est comptée pour tout le monde | vérifier le prérequis de propriétaire sur le joueur avant de construire les sujets. Gérer `REQUIREMENT_PLAYER_LEADER_TYPE_MATCHES`, et `…_CIVILIZATION_TYPE_MATCHES` au passage. Corrige aussi l'agrément Taino de Raj (`LIME_LILI_POLICY_RAJ_AMENITIES`, prérequis de sujet) |
| **Médina** | BBG : `EFFECT_ADJUST_DISTRICT_HOUSING` +1 sur chaque quartier spécialisé (`COLLECTION_PLAYER_DISTRICTS`, 29 types listés) | effet inconnu : rien. RMA ne gère que la version du jeu (3 quartiers, `REQUIREMENT_CITY_HAS_X_SPECIALTY_DISTRICTS`) | gérer l'effet : `HOUSING` = montant, par quartier qui remplit le prérequis |
| **Conscription, Levée en masse** | `EFFECT_ADJUST_UNIT_MAINTENANCE_DISCOUNT` 1 et 2 | `min(carte, entretien de base)` par unité militaire, l'entretien venant de `GameInfo.Units` (×1,5 en corps, ×2 en armée). Les réductions déjà en place sont ignorées | partir de l'entretien restant de chaque unité : la base moins les réductions actives. Ce sont le **Trône du seigneur de guerre** (`BUILDING_GOV_CONQUEST`, −1 avec BBG), l'autre carte d'entretien si elle est posée, Harald et Saladin (variantes, −2 et −1), et Forces d'élite (+2). `Treasury:GetUnitMaintenance()` donne le total réel pour recouper. « Aux armes ! » ne touche pas à l'entretien dans la base (production +15 %, or des victoires, ressources, et un emplacement militaire de plus avec BBG) |
| **Prestige civil** | `EFFECT_ADJUST_POLICY_AMENITY` +1 et `…_HOUSING` +2, prérequis `REQUIREMENT_CITY_HAS_GOVERNOR_WITH_X_TITLES` (`Amount=2`, `Established=1`) | `GetGovernorData` compte `pGovernor:HasPromotion(row.Index)`. Le jeu appelle toujours `HasPromotion(…Hash)` (GovernorPanel, GovernorAssignmentChooser) : avec un index, le compte est faux, d'où un effet absent | `HasPromotion(row.Hash)`, à vérifier en jeu |
| **Prétoire** | BBG : `EFFECT_ADJUST_CITY_YIELD_MODIFIER` +20 % de production et `EFFECT_ADJUST_CITY_IDENTITY_PER_TURN` +4 dans les villes annexées, prérequis `REQUIREMENT_CITY_IS_ORIGINAL_OWNER` inversé | type de prérequis inconnu : faux, avant même l'inversion. Aucune ville, aucun effet | gérer le prérequis : `GetOriginalOwner() == GetOwner()` |
| **Taxe coloniale** | BBG : +10 % de production, +25 % d'or (et la loyauté, la croissance) dans les villes à 8 cases ou plus de la capitale, `REQUIREMENT_PLOT_NEAR_CAPITAL` (`MinDistance=8`) | prérequis inconnu : rien | gérer le prérequis : `Map.GetPlotDistance` jusqu'à la capitale, entre `MinDistance` et `MaxDistance`. La croissance reste sans chiffre, comme pour toutes les cartes de croissance |
| **Points de personnages illustres** (Invention, Laissez-faire, Organisation militaire, Prix Nobel, Symphonies, Fresques) | `EFFECT_ADJUST_GREAT_PERSON_POINTS` sur le joueur (fixe) et sur les villes qui ont un bâtiment (`BUILDING_IS_WORKSHOP`…) | effet inconnu : rien | ajout : une clé `PG_<classe>` par classe de `GreatPersonClasses` dans les tables de rendements de RMA (pas dans `YieldTypes`, partagée avec le jeu), affichée « [icône de la classe]+x », sur toutes les cartes de points : fixes (Inspiration…) comme par ville (Invention : le total, fixe compris) |
| **Monachisme** | `EFFECT_ADJUST_CITY_YIELD_MODIFIER` : +75 % de science (villes avec Lieu saint), −25 % de culture (toutes) | impact = rendement **total** de la ville × pourcentage | les pourcentages du jeu s'additionnent sur le rendement de base : impact = total × p / (100 + somme des % déjà appliqués). Le total contient déjà les autres bonus (et le −25 % lui-même si la carte est posée) : RMA surestime les deux lignes. Trouver une lecture du rendement sans modificateurs, ou reconstituer la somme des %. Même défaut pour toutes les cartes en `…_YIELD_MODIFIER` |

**Les titres de gouverneur** (question posée avec le Prestige civil). Le titre dépensé pour nommer le
gouverneur **compte**. Dans GameCore :
- `REQUIREMENT_CITY_HAS_GOVERNOR_WITH_X_TITLES` est évalué par `1808ea820` : il prend le gouverneur de
  la ville, exige qu'il soit établi si `Established=1`, puis compare `Amount` à `1802598f0(gouverneur)`.
- `1802598f0` compte les cases vraies du tableau des promotions du gouverneur (`+0x68`, longueur
  `+0x78`).
- Ce même compte, additionné sur tous les gouverneurs (`1802dea90`), est le total des titres dépensés
  (`GetGovernorPointsSpent` côté Lua). La nomination (`1802dd760`) et la promotion (`1802ddbb0`) ne
  sont permises que s'il reste en dessous des titres obtenus (`+0x118`). Si la capacité de base n'était
  pas dans le tableau, nommer un gouverneur ne coûterait rien.
- **Conclusion :** pour le Prestige civil, il faut un gouverneur **établi, nommé puis promu une fois**,
  soit 2 titres dépensés sur lui. Le « au moins deux promotions » de la description compte la
  nomination. Les seuils 1 à 6 de BBG pour Seondeok vont dans le même sens : 6 promotions par
  gouverneur, capacité de base comprise.

## Réglages

D'après la règle du dépôt, chaque fonctionnalité réglable a sa clé dans `read_config`
([proxy/src/lib.rs](../proxy/src/lib.rs)), son paramètre dans `install.ps1` et sa ligne dans l'onglet
BetterGameExperience (`PROXY_REGLAGES` de `options_proxy.lua`, que vérifie `cles_communes_avec_le_proxy`).

| clé | `install.ps1` | défaut |
|---|---|---|
| `ui_politiques=on\|off` | `-UiPolitiques` | on |
| `ui_bandeau=on\|off` | `-UiBandeau` | on |
| `ui_rapports=on\|off` | `-UiRapports` | on |
| `ui_notifications=on\|off` | `-UiNotifications` | on |
| `ui_charges=on\|off` | `-UiCharges` | on |

Défaut on : les reprises corrigent les défauts des mods, et chacun peut couper celles qu'il ne veut pas. Un réglage actif prime
sur le mod du même rôle ; inactif, il laisse le mod tel quel. Interface seule, donc sans effet sur la
synchronisation, et sans rien à annoncer dans le salon.

## Crédits

Le code des mods est repris, avec ses corrections, et leurs auteurs sont crédités. Aucun des cinq n'a
de licence, l'usage du Workshop de Civ6 étant le partage. Les en-têtes d'origine (auteurs, dates)
restent dans les fichiers embarqués.

- **Dans le jeu** : une section **Crédits**, en bas de l'onglet BetterGameExperience. Pour chaque reprise :
  le nom du mod, ses auteurs et remerciements, et le numéro du Workshop. Les différences avec le mod
  n'y figurent pas : elles sont dans ce document.
  Elle n'est jamais grisée par l'interrupteur général. Textes en français et en anglais, comme le
  reste de l'onglet.
- **Dans le dépôt** : la même liste dans le README du proxy.

| mod | auteurs | remerciements du mod |
|---|---|---|
| Extended Policy Cards | Aristos | – |
| TopPanel Extension Pro | 号码菌 (Azukii) | JNNR |
| Better Report Screen, RLL et RMA | Infixo | Greg Miller (onglets Unités et Accords d'origine), nightfearer, Tyolan, copysoul, RohBang, Gorlod, Zergus, Rac, Aristos |
| NotificationClear | 336tarot | – |
| Better Builder Charges Tracking | wltk | DeepLogic, 小小雨10101 |

## À vérifier en jeu

Rien n'a encore tourné dans le jeu. Hors jeu, les patchs passent sur les vrais fichiers du jeu et des
mods, la syntaxe de tous les scripts est vérifiée, et les corrections de RMA comme le bloc des
drapeaux ont leurs tests Lua. Restent à voir en jeu :

1. **HavokScript** : `getfenv` / `setfenv` (RMA isolé), et `Locale.Lookup` qui met en forme un texte
   brut (les textes des mods passent par là). Le journal `Lua.log` dira tout échec
   (`BetterGameExperience : …`).
2. **Les fonctions natives** : `mods : fonctions natives posées` au journal, puis un écran des
   rapports qui s'ouvre avec ses pages.
3. **Les tours des routes** : comparer le macaron au tour où la route prend fin, sur une route
   lancée en cours de partie et sur une partie rechargée.
4. **Le bandeau** : le bloc doit atterrir dans le script du contexte. Le journal doit montrer
   `bandeau de TopPanel Extension Pro` sur `TopPanel_Expansion2.lua`, ou sur `TopPanel_Spectator.lua`
   avec BSM. Le bouton de la population doit porter l'icône des activités et la moyenne (entière,
   arrondie vers le bas) de la colonne des activités de son infobulle, elle-même inchangée, et
   changer dès qu'un luxe est gagné ou perdu en cours de tour.
5. **Les cartes signalées** : Raj, cartes de personnages illustres (hors Anacaona), Médina,
   Conscription et Levée en masse avec le Trône du seigneur de guerre, Prestige civil, Monachisme.
6. **Avec les mods encore actifs** : la version du proxy doit s'afficher seule (un seul bouton de
   notifications, pas de double bandeau, l'écran des rapports du proxy).
7. **Les crédits**, en bas de l'onglet BetterGameExperience.

**Pertes connues sans le mod** : le raccourci F8 de l'écran des rapports, et les 15 icônes de police de
BRS (quelques quartiers dans l'onglet des villes).

## Choix faits sans retour en jeu

- **BRS et XP2 avec BSM** : `ReportScreen_Expansion1/2` réduits à leur `include`, comme BRS le fait
  sans BSM. Pour revenir à ce qu'on voyait avec BSM, il suffit de retirer ces deux cas de
  `mods_ui::remplacer`.
- **Charges des bâtisseurs étrangers** : montrées, comme le mod. Les tours des routes ne concernent que
  les négociants du joueur local, la route d'un autre joueur n'étant pas dans ses villes.
- **Inventaire complet des cartes** mal gérées par RMA : pas fait. Seules les six cartes signalées sont
  corrigées, mais les corrections valent pour toutes les cartes du même type d'effet :
  - le prérequis de propriétaire ;
  - les pourcentages de rendement de ville ;
  - l'entretien ;
  - le compte des promotions de gouverneur.
