# Raccourcis clavier coupés en multijoueur

**Corrigé dans le proxy** (2026-11-06, interface seule, sans réglage) : `patch_in_game` dans [proxy/src/ui.rs](../../proxy/src/ui.rs), bloc [ui_raccourcis.lua](../../proxy/src/ui_raccourcis.lua) ajouté à `InGame.lua` après celui du départ simultané. Pas encore essayé en partie.

Analyse statique (2026-11-06) des scripts d'interface du jeu et des mods (MPH, BSM, BBG). Symptôme :
dans une partie multijoueur un peu longue, tous les raccourcis (fin de tour, arbre des technologies,
ordres d'unités…) cessent de marcher. La souris marche, l'interface reste affichée. Échap répare
parfois, pas toujours. Ensuite, seul un nouveau salon répare.

Résumé : le jeu tient une **pile de contextes d'entrée** dans l'exe. Tous les raccourcis sont déclarés
dans le contexte `World` et ne partent que si `World` est en haut de la pile. Deux écrans du jeu de base
peuvent laisser un contexte en trop sur la pile : le menu Échap (`GameOptions`), et peut-être la
fenêtre du congrès (`Popup`). Le cas du menu Échap est lu dans le code. Il ne se produit qu'en
multijoueur, et Échap le répare. Le correctif proposé est d'interface seule, sans désynchro et sans
réglage.

## Comment un raccourci part

Les actions d'entrée sont décrites en base de données, chacune avec un contexte :

- `Base/Assets/Configuration/Data/InputConfiguration.xml:18-37` : `InputContexts` est un masque de
  bits (`World` = 16, `Diplomacy` = 32, `GameOptions` = 64, `EndGame` = 128, `Popup` = 256,
  `Reveal` = 512, `FullscreenMap` = 1024, plus des combinaisons comme `WorldOrPopup` = 272).
- `InputConfiguration.xml:48-88` : tous les raccourcis de la partie (`EndTurn`, `ToggleTechTree`,
  `NextUnit`, `Fortify`, `PauseMenu`, `QuickSave`…) sont en `ContextId="World"`. Seul `ToggleFSMap`
  est en `WorldOrFullScreenMap`.

L'exe ne lance `Events.InputActionTriggered` que si le contexte actif est compris dans le masque de
l'action. Le Lua manipule ce contexte avec `Input.PushActiveContext(c)`, `Input.PopContext()`,
`Input.SetActiveContext(c)` et `Input.GetActiveContext()`. `PopContext` n'a pas d'argument : il retire
le haut de la pile, quel qu'il soit.

Les gestionnaires Lua de raccourcis (`WorldInput.lua:4017`, `ActionPanel.lua:1294`,
`LaunchBar.lua:703`, `PartialScreenHooks.lua:397`, `MinimapPanel.lua:699`, `UnitPanel.lua:2934`,
`InGame.lua` pour `PauseMenu` et `QuickSave`) ont chacun leurs propres conditions
(`g_isOkayToProcess` de `UnitPanel` par exemple). Aucune de ces conditions ne coupe *tous* les
raccourcis à la fois, ni de façon durable. Une panne générale vient de l'exe qui ne lance plus
l'événement, donc du contexte actif.

Les touches brutes ne passent pas par ce filtre. `ContextPtr:SetInputHandler` reçoit toujours les
touches, d'où les deux faits observés :
- la souris et les champs de texte marchent toujours ;
- Échap, lu en touche brute par `InGame.lua` (`DefaultMessageHandler[KeyEvents.KeyUp]`, MPH
  `ui/ingame.lua:68`), ouvre toujours le menu, même quand l'action `PauseMenu` ne part plus.

## Qui empile, qui dépile

Liste complète des appels du jeu de base, de XP1 et de XP2 (hors scénarios). Dans le Workshop, seuls
MPH (`ui/ingame.lua`, recopie de celui de XP2) et BSM (`UI/LoadScreen.lua`, `SetActiveContext`) y
touchent :

| contexte | empile | dépile | en multi |
|---|---|---|---|
| `Diplomacy` | `InGame.lua:317` (XP2), MPH `ui/ingame.lua:324`, sur `DiplomacyActionView_HideIngameUI` | même fichier, sur `DiplomacyActionView_ShowIngameUI` | oui |
| `EndGame` | `InGame.lua:321` | `InGame.lua:322` | fin de partie |
| `FullscreenMap` | `InGame.lua:323` | `InGame.lua:324` | oui |
| `GameOptions` | `InGameTopOptionsMenu.lua:571-573` (`OnShow`) | `InGameTopOptionsMenu.lua:262-267` (`Close`), **sous condition** | oui |
| `Popup` | `WorldCongressPopup.lua:2383` (`ShowPopup`), `:2535` (`OnResumeCongress`) | `:2397` (`ClosePopup`), **sous condition** ; `:2465` (`OnShutdown`) | oui |
| `Reveal` | `PopupManager.lua:59`, `RockBandMoviePopup.lua:81` | `PopupManager.lua:96`, `RockBandMoviePopup.lua:36` | non : ces écrans sont coupés en multi |

Les écrans qui passent par `PopupManager` (merveilles, projets, merveilles naturelles, catastrophes,
ères) et `RockBandMoviePopup` ne se chargent pas en multijoueur (`WonderBuiltPopup.lua:208`,
`ProjectBuiltPopup.lua:209`, `NaturalWonderPopup.lua:244`, `NaturalDisasterPopup.lua:506`,
`EraCompletePopup.lua:271` de XP2, `RockBandMoviePopup.lua:115`). Ils sont hors de cause.

`Diplomacy`, `EndGame` et `FullscreenMap` sont toujours empilés en même temps qu'un `BulkHide(true)`
(`InGame.lua`), qui masque le HUD. Si l'un d'eux restait sur la pile, le HUD resterait masqué. Or le
HUD reste visible quand le bug se produit. Il reste deux suspects : `GameOptions` et `Popup`.

## Cause 1 (lue dans le code) : le menu Échap laisse `GameOptions`

`Base/Assets/UI/Menus/InGameTopOptionsMenu.lua`, version chargée en partie : celle du jeu, reprise
telle quelle par MPH (`ui/Replacements/ingametopoptionsmenu_MPH.lua:7`,
`include( "InGameTopOptionsMenu" )`, sans redéfinir `Close` ni `CloseImmediately`).

```lua
function OnShow()                                    -- :559
	if Input.GetActiveContext() ~= InputContext.GameOptions then
		Input.PushActiveContext( InputContext.GameOptions );   -- :571-573
	end

function Close()                                     -- :231
	-- Only pop the context if what we expect is the current context.
	if(Input.GetActiveContext() == InputContext.GameOptions) then
		Input.PopContext();                                     -- :262-267
	else
		print("Expected a different input active context...");
	end

function CloseImmediately()                          -- :221
	if UIManager:IsInPopupQueue( ContextPtr ) then
		LuaEvents.InGameTopOptionsMenu_Close();
		UIManager:DequeuePopup( ContextPtr );                   -- pas de PopContext
	…
LuaEvents.DiplomacyActionView_HideIngameUI.Add( CloseImmediately );   -- :726
```

### Chemin A : un écran diplomatique s'ouvre pendant que le menu est ouvert

1. Le joueur ouvre le menu Échap. La pile vaut `World, GameOptions`.
2. Un autre joueur déclare la guerre, propose un accord, ou une rencontre a lieu.
   `DiplomacyActionView.lua:2122` envoie `DiplomacyActionView_HideIngameUI`.
3. Le menu se ferme par `CloseImmediately`, sans dépiler. `InGame` empile `Diplomacy`. La pile vaut
   `World, GameOptions, Diplomacy`.
4. L'écran diplomatique se ferme (`DiplomacyActionView.lua:2874`). `InGame` dépile. La pile vaut
   `World, GameOptions`. Le menu est fermé, le HUD revient, mais aucun raccourci ne part.

### Chemin B : le menu se ferme alors qu'un autre contexte est au-dessus

1. Le menu est ouvert. La pile vaut `World, GameOptions`.
2. Le tour passe pendant ce temps (en multi, le temps ne s'arrête pas). Une étape du congrès commence
   (`Events.WorldCongressStage1/2`, `WorldCongressPopup.lua:2676-2677`), et `ShowPopup` empile `Popup`.
   La pile vaut `World, GameOptions, Popup`.
3. Le menu a une priorité plus haute que le congrès (`PopupPriority.InGameTopOptionsMenu = Utmost+…`,
   `PopupPriorityLoader_Base.lua:15`, contre `Medium+…` dans `PopupPriorityLoader_Expansion2.lua:12`).
   Il reçoit donc Échap ou « Retour au jeu » en premier. `Close` voit `Popup` en haut, ne dépile pas
   et écrit `Expected a different input active context...` dans `Lua.log`.
4. Le congrès se ferme et dépile `Popup`. La pile vaut `World, GameOptions`.

### Pourquoi en multijoueur seulement, et après un moment

En solo, rien n'arrive pendant que le menu est ouvert : c'est le tour du joueur, et l'IA ne joue
qu'ensuite. En multi, les tours sont simultanés. Les autres joueurs agissent et les tours passent
pendant que le menu est ouvert. MPH pousse en plus à l'ouvrir (bouton « Helper », chat « gg »
qui ouvre le menu, `ingametopoptionsmenu_MPH.lua:267-270`). Chaque ouverture qui croise une
diplomatie ou un congrès a une chance de déclencher le bug, donc plus la partie dure, plus il devient
probable.

### Pourquoi Échap répare ce cas

Échap arrive en touche brute à `InGame`, qui ouvre le menu. `OnShow` voit `GameOptions` déjà actif
et n'empile rien (`:571`). À la fermeture, `Close` voit `GameOptions` en haut et dépile. La pile
redevient `World`.

## Cause 2 (supposée) : le congrès laisse `Popup`

`DLC/Expansion2/UI/Additions/WorldCongressPopup.lua`, la seule version chargée (aucun mod du Workshop
ne la remplace ; le proxy y ajoute `ui_congres.lua` pour les spectateurs, ce qui ne touche pas à la
pile).

```lua
function ShowPopup(delayShow, fromHotLoad)                     -- :2377
	if ContextPtr:IsHidden() or fromHotLoad then
		UIManager:QueuePopup(ContextPtr, …, { AlwaysVisibleInQueue = true, DelayShow = delayShow });
		Input.PushActiveContext( InputContext.Popup );

function ClosePopup()                                          -- :2394
	if ContextPtr:IsVisible() then
		UIManager:DequeuePopup(ContextPtr);
		Input.PopContext();

function OnResumeCongress()                                    -- :2531
	if not m_HasAccepted and (not m_IsInSession or HasChoices()) then
		UIManager:QueuePopup(ContextPtr, …);
		Input.PushActiveContext( InputContext.Popup );        -- sans test de visibilité
```

L'empilement et le dépilement ne suivent pas le même test. On empile tant que la fenêtre est
**masquée**, même si elle est déjà dans la file. On ne dépile que si elle est **visible**. Avec
`DelayShow` (résultats ouverts depuis la notification, `:2654-2655`), la fenêtre attend dans la file,
masquée, jusqu'à ce que `InGame` libère les fenêtres différées. Deux appels pendant ce délai (deux
clics, une étape du congrès, `OnResumeCongress`) empilent deux fois. Ensuite, un seul `ClosePopup`
dépile. La pile garde `Popup`.

Échap ne répare pas ce cas. `OnShow` du menu voit `Popup` et empile `GameOptions`. `Close` dépile
`GameOptions`. `Popup` reste.

Ce chemin est possible d'après le code, mais je ne l'ai pas reproduit. Cela suppose que `IsHidden()`
réponde vrai pour une fenêtre en file avec `DelayShow`, et que l'exe ne dédouble pas un
`QueuePopup` du même contexte. La journalisation proposée plus bas dira s'il se produit.

## Pourquoi seul un nouveau salon répare

La pile vit dans l'exe (objet `Input`), pas dans un état Lua. Recharger l'interface ne la touche pas,
y compris le bouton « UI Refresh » de MPH (`ui/Additions/MPHOptions.lua:101-115`, qui relance
`Initialize` d'`InGame`). Un nouveau salon, lui, recharge la partie. `LoadScreen.lua` appelle alors
`Input.SetActiveContext` (`Loading`, puis `World` et `Ready` : jeu `FrontEnd/LoadScreen.lua:57,485,503`,
BSM `UI/LoadScreen.lua:57,638,680`), et le menu principal fait de même avec `Shell`
(`FrontEnd.lua:101`). `SetActiveContext` remet la pile à zéro. C'est déduit de son usage, pas
vérifié dans l'exe. Tout passage par l'écran de chargement devrait donc réparer : quitter et revenir
dans la même partie, ou recharger une sauvegarde. Les options de touches (`Options` / `AppOptions`)
ne sont pas en cause : elles ne changent pas pendant la partie, et elles marchent de nouveau après
un rechargement.

## Fausses pistes écartées

- **`UI.GetInterfaceMode`, `UIManager:IsModal`, drapeaux `m_isOpen` des écrans** : ils gardent chacun
  un seul gestionnaire Lua, jamais l'ensemble des raccourcis.
- **File des fenêtres bloquée** (`UIManager:ShowDelayedPopups`, `m_bulkHideTracker` d'`InGame`) :
  elle retarde l'affichage des fenêtres, pas les raccourcis. Un `m_bulkHideTracker` resté au-dessus
  de 0 masquerait le HUD.
- **`TurnProcessing` de MPH** (`ui/Additions/TurnProcessing.lua:488-499`) : quand il est affiché, il
  avale toutes les touches enfoncées (sauf Alt, Ctrl, Maj). Il n'est actif qu'avec l'option
  `CPL_SYNCTURN` (désactivée par défaut, `configuration/Config.xml:17`). C'est un voile plein écran
  « PROCESSING », donc visible, et `U` le ferme. À garder en tête si cette option est un jour activée.
- **« UI Refresh » de MPH** : `ui/ingame.lua:401` inscrit `Initialize` sur `InGame_OnLocalUIRefresh`,
  et `Initialize` se réinscrit à chaque appel. Chaque rafraîchissement double tous les gestionnaires
  d'`InGame` (et charge une seconde fois les interfaces ajoutées par les mods). Les empilements
  `Diplomacy` restent appariés, donc la pile reste juste. C'est un défaut de MPH, mais pas la cause
  ici, et ce bouton ne répare pas le bug.
- **Bug 5 (Entrée)** : sans rapport direct. Une touche lue en brut (`SetInputHandler`) passe même
  quand la pile est bloquée, contrairement à une action d'entrée.

## Correctif proposé

Correctif **d'interface seule** : la pile de contextes est locale, sans effet sur GameCore ni sur
le réseau, donc pas de désynchro et pas de POL. C'est un bug pur, donc **pas de réglage**.

### Où

`InGame.lua`, nom exact. `is_file` ignore la casse, donc le correctif prend aussi la version de MPH
(`3307282026/ui/ingame.lua`, importée sous ce nom) et celle de XP2
(`DLC/Expansion2/UI/Replacements/InGame.lua`). Aucun autre mod ne remplace `InGame`. Il suffit
d'ajouter une entrée à `FIXES` dans `proxy/src/ui.rs`, avec un bloc `src/ui_raccourcis.lua` ajouté
en fin de fichier. Le bloc utilise la locale `DefaultMessageHandler` du fichier, comme `ui_congres.lua`
utilise celles de `WorldCongressPopup.lua`. Il faut vérifier qu'elle existe, avec un `has(...)` sur
`local DefaultMessageHandler`.

Un seul point de réparation, dans `InGame`, couvre les deux causes, et celles qu'on n'a pas vues. Il
évite de corriger séparément `InGameTopOptionsMenu.lua` et `WorldCongressPopup.lua` (ce dernier est
déjà repris par `patch_congres` quand les spectateurs sont posés, et `patch_ui` ne fait qu'un patch par
fichier).

### Quoi

On retire un contexte `GameOptions` ou `Popup` du haut de la pile quand l'écran qui l'a empilé est
fermé : masqué, et absent de la file des fenêtres. Les autres contextes ne sont jamais touchés
(`Diplomacy`, `Reveal`, `FullscreenMap`, `Tutorial`…).

```lua
-- ===========================================================================
-- Civ6RolloverProxy (src/ui_raccourcis.lua), ajouté par le proxy à la fin de InGame.lua.
-- Raccourcis clavier coupés : un contexte d'entrée resté sur la pile après la fermeture de son écran
-- (menu Échap fermé par une diplomatie, congrès empilé deux fois) bloque toutes les actions du
-- contexte World. On le retire dès que l'interface est au repos ou qu'une touche est enfoncée.
-- ===========================================================================
do
	local function BGD_Ferme(pContext)
		return pContext == nil or (pContext:IsHidden() and not UIManager:IsInPopupQueue(pContext));
	end

	local function BGD_Orphelin(eActif)
		if eActif == InputContext.GameOptions then
			return BGD_Ferme(ContextPtr:LookUpControl("/InGame/TopOptionsMenu"));
		elseif eActif == InputContext.Popup then
			return BGD_Ferme(ContextPtr:LookUpControl("/InGame/WorldCongressPopup"));
		end
		return false;
	end

	local function BGD_RetablirContexte(qui)
		for _ = 1, 8 do
			local eActif = Input.GetActiveContext();
			if not BGD_Orphelin(eActif) then
				return;
			end
			print("BGD raccourcis : contexte d'entrée " .. tostring(eActif) .. " retiré (" .. qui .. ")");
			Input.PopContext();
		end
	end

	local BGD_KeyDown = DefaultMessageHandler[KeyEvents.KeyDown];
	DefaultMessageHandler[KeyEvents.KeyDown] = function(pInputStruct)
		BGD_RetablirContexte("touche");
		if BGD_KeyDown ~= nil then
			return BGD_KeyDown(pInputStruct);
		end
		return false;
	end

	Events.UIIdle.Add(function() BGD_RetablirContexte("repos"); end);
	-- Après InGame, qui dépile Diplomacy sur le même événement (inscrit avant) : chemin A.
	LuaEvents.DiplomacyActionView_ShowIngameUI.Add(function() BGD_RetablirContexte("diplomatie"); end);
end
```

Pourquoi c'est sûr :
- `GameOptions` n'est empilé que par le menu Échap, qui est dans la file pendant tout le temps où il
  est ouvert, sous-écrans compris (options, sauvegarde). `Popup` n'est empilé en partie que par le
  congrès, qui se met dans la file *avant* d'empiler et s'en retire *avant* de dépiler
  (`WorldCongressPopup.lua:2382-2383`, `:2396-2397`, `:2534-2535`). Il n'y a donc aucun moment où un
  écran ouvert verrait son contexte retiré.
- `OnShow` du menu empile seulement si `GameOptions` n'est pas déjà actif. Après la réparation, il
  empile normalement, et `Close` dépile normalement.
- `InGame.lua` est le contexte racine : sa fonction `OnInputHandler` reçoit les touches que les
  fenêtres n'ont pas prises. Quand la pile est bloquée, aucune fenêtre n'est ouverte, donc les touches
  y arrivent. `UIIdle` répare en général avant même la première touche.
- La boucle est bornée et ne dépile jamais `World` (`BGD_Orphelin` est faux pour `World`).

Le `print` laisse une trace dans `Lua.log` (`BGD raccourcis : contexte d'entrée 64 retiré (...)`).
Elle confirmera en partie réelle le chemin emprunté (64 = menu Échap, 256 = congrès). Elle peut
rester : elle ne s'écrit que lorsqu'il y a eu un bug.

### Variante plus étroite

Corriger seulement la cause 1 dans `InGameTopOptionsMenu.lua` (le patch s'applique aussi à MPH, qui
inclut ce fichier) :
- dans `CloseImmediately`, dépiler si `GameOptions` est en haut ;
- sinon, ou si `Close` tombe sur `Expected a different input active context`, noter un dépilement en
  attente, à faire dès que `GameOptions` revient en haut, depuis
  `LuaEvents.DiplomacyActionView_ShowIngameUI` ou `Events.UIIdle`.

Elle demande le même mécanisme différé et ne couvre pas le congrès. La version dans `InGame` est
préférable.

## Vérifier

- **Avant le correctif**, en multi à deux : ouvrir le menu Échap, puis faire déclarer la guerre (ou
  proposer un accord) par l'autre joueur. Fermer l'écran diplomatique, puis essayer `Entrée` ou `T`.
  Dans la console Lua (FireTuner, contexte `InGame`), `print(Input.GetActiveContext())` doit donner
  `64`. Ensuite, Échap puis fermer le menu doit réparer.
- **Sans second joueur** : dans la console, contexte `InGameTopOptionsMenu`, appeler
  `LuaEvents.InGame_OpenInGameOptionsMenu()`, puis `LuaEvents.DiplomacyActionView_HideIngameUI()` et
  `LuaEvents.DiplomacyActionView_ShowIngameUI()`. On obtient la même pile `World, GameOptions`.
- **Après le correctif** : même scénario. La ligne `BGD raccourcis : … 64 retiré (diplomatie)` doit
  apparaître dans `Lua.log`, et les raccourcis doivent marcher tout de suite.
- **Test hors jeu**, sur le modèle de `proxy/tests/test_ui_*.lua` : simuler `Input` (une pile),
  `UIManager:IsInPopupQueue`, `ContextPtr:LookUpControl`, puis rejouer les chemins A et B et la
  double ouverture du congrès. Contrôler aussi qu'un menu ouvert, ou un congrès en file avec
  `DelayShow`, garde son contexte.
- **Non vérifié dans l'exe** : que l'action n'est filtrée que par le haut de la pile, et que
  `SetActiveContext` vide la pile. Si besoin, à confirmer dans le projet Ghidra `civ6exe` à partir des
  chaînes `PushActiveContext` et `PopContext` de la table Lua `Input`. Ce n'est pas nécessaire au
  correctif.
