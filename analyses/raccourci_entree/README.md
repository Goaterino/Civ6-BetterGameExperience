# Raccourci Entrée : « Action suivante » supprimée, Entrée agit encore

**Corrigé dans le proxy** (2026-11-06, interface seule, sans réglage) : `patch_action_panel` dans [proxy/src/ui.rs](../../proxy/src/ui.rs), bloc [ui_entree.lua](../../proxy/src/ui_entree.lua) inséré avant `OnInputHandler`. Confirmé en partie par l'utilisateur le 2026-11-06.

Analyse statique (2026-11-06) des scripts d'interface du jeu et des mods. Bug 5 : le joueur efface dans
Options > Raccourcis la touche Entrée de « Action suivante » ; Entrée continue de passer à l'unité
suivante, d'ouvrir la notification bloquante ou de finir le tour. Correctif d'interface seul,
fait dans le proxy.

## L'action configurable

`Base/Assets/Configuration/Data/InputConfiguration.xml` :

| ligne | contenu |
|---|---|
| 78 | `ActionId="EndTurn"`, nom `LOC_OPTIONS_HOTKEY_GLOBAL_ENDTURN`, contexte `World` |
| 174 | `InputActionDefaultGestures` : `EndTurn`, index 0, `KBMouse`, `LOC_OPTIONS_KEY_RETURN` |
| 85, 181 | `NextUnit` (« Unité prête suivante »), touche Point par défaut |
| 127, 211 | `NextAction` : manette seulement (`LayoutType="Console"`, `LOC_OPTIONS_PAD_X`) |

Le nom affiché de `EndTurn` est « Next Action » / « Action suivante »
(`Text/en_US/UI_Options_Text.xml:862`, `Text/Vanilla_fr_FR.xml:51282`). C'est donc bien l'action
`EndTurn` que le joueur délie. Le schéma est dans `Configuration/Data/Schema/InputConfiguration.sql`
(`InputActions`, `InputActionDefaultGestures`, `InputKeyData`).

Les touches du joueur sont dans `%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI\InputSettings.json`
(pas dans My Games, ni dans `UserOptions.txt`). Une entrée par action, deux emplacements :

```
"EndTurn": [{}, {}],
"CivilopediaBack": [{"type": "KBMouse", "data": "LOC_OPTIONS_KEY_CONTROL+LOC_OPTIONS_KEY_COMMA"}, {}],
```

Vérifié sur la machine : `EndTurn` y est vide. Le réglage du joueur est bien enregistré ; c'est le script
qui l'ignore.

## La cause : ActionPanel.lua teste Entrée en dur

Script chargé en partie : contexte `ActionPanel` (`Base/Assets/UI/InGame.xml:43`, et MPH
`3307282026/ui/ingame.xml:51`). Avec XP2, `ReplaceUIScript` le remplace par
`DLC/Expansion2/UI/Replacements/ActionPanel_Expansion2.lua` (`Expansion2.modinfo:366`), qui inclut
`ActionPanel_Expansion1.lua`, qui inclut `ActionPanel` (le fichier de base). Ni XP1 ni XP2 ne redéfinissent
`OnInputHandler` ni `Initialize`. Aucun mod du Workshop (BBG, BBG Expanded, BBG BETA, BSM, MPH, DMT, BRS…)
ne remplace ActionPanel. Le code fautif est donc celui du jeu de base, dans tous les cas.

Deux chemins mènent à `DoEndTurn` :

1. L'action configurable, qui respecte le réglage (`Base/Assets/UI/ActionPanel.lua:125`, `:1294`, `:1330`) :

```lua
local m_EndTurnId : number = Input.GetActionId("EndTurn");
function OnInputActionTriggered( actionId )
	if m_EndTurnId ~= nil and actionId == m_EndTurnId then
        UI.PlaySound("Play_UI_Click");
		DoEndTurn();
```

2. Le gestionnaire de touches, qui l'ignore (`ActionPanel.lua:1093-1107`, posé en `:1364` par
   `ContextPtr:SetInputHandler( OnInputHandler, true )`) :

```lua
	if uiMsg == KeyEvents.KeyUp then
		if pInputStruct:GetKey() == Keys.VK_RETURN then
			if pInputStruct:IsShiftDown() and not IsTutorialRunning() then
				UI.RequestAction(ActionTypes.ACTION_ENDTURN, { REASON = "UserForced" } );
			else
				DoEndTurn();
			end
			return true;
```

Délier Entrée coupe le chemin 1, pas le chemin 2. `DoEndTurn` (`ActionPanel.lua:526-608`) fait l'« action
suivante » : retirer le prêt (`ACTION_UNREADYTURN`) si possible, sinon unité prête suivante
(`UI.SelectNextReadyUnit`), ville qui peut tirer, notification bloquante, ou fin du tour. D'où le symptôme.

Maj+Entrée force en plus la fin du tour (`REASON = "UserForced"`), même avec des blocages. Ce raccourci
caché n'a aucune action configurable.

Supposé, non vérifié dans l'exe : quand Entrée est liée (réglage par défaut), le `return true` du
gestionnaire mange la touche et l'action `EndTurn` ne se déclenche pas une seconde fois. Sinon un appui
appellerait `DoEndTurn` deux fois (fin du tour puis retrait du prêt), ce qu'on ne voit pas en jeu. Conséquence
annexe : si le joueur lie Entrée à une autre action, ActionPanel la lui prend aussi.

## Les autres Entrée en dur

Aucun ne dépend de « Action suivante ». Ce sont des validations de fenêtre, à garder.

| fichier:ligne | rôle |
|---|---|
| `Base/Assets/UI/Screens/TechTree.lua:1797` | mange Entrée « pour qu'elle n'atteigne pas l'action panel » (commentaire Firaxis) |
| `Base/Assets/UI/Screens/CivicsTree.lua:2166` | idem |
| `Base/Assets/UI/Screens/GovernmentScreen.lua:2744` | valide les doctrines, et mange Entrée pour la même raison |
| `DLC/Expansion1/UI/Additions/GovernorDetailsPanel.lua:477` | valide la promotion du gouverneur |
| `DLC/Expansion2/UI/Additions/WorldCongressPopup.lua:2479` | valide le vote du congrès (étape 4) |
| `DLC/Expansion2/UI/Additions/WorldCongressIntro.lua:104` | ferme l'intro (Échap ou Entrée) ; `:91` la ferme aussi sur l'action `EndTurn` |
| `Base/Assets/UI/Popups/InGamePopup.lua:67` | bouton Confirmer de la fenêtre modale |
| `Base/Assets/UI/Popups/BoostUnlockedPopup.lua:326`, `EventPopup.lua:286` | ferme la fenêtre |
| `Base/Assets/UI/Popups/PlayerChange.lua:27` | partie en tour par tour sur un poste : joueur suivant |
| `Base/Assets/UI/Menus/SaveGameMenu.lua:296` | valide la sauvegarde |
| `Base/Assets/UI/Popups/AdvisorPopup.lua:403` | débogage seulement (`not UI.IsFinalRelease()`) |
| `Base/Assets/UI/WorldBuilderToolsPalette.lua:190` | éditeur de cartes |
| `DLC/BlackDeathScenario/.../GovernmentScreen_BlackDeathScenario.lua:2642` | scénario |

Dans les mods, seul `2266952591` (Extended Policy Cards, hors liste des mods utilisés) a un `VK_RETURN`,
dans sa copie de GovernmentScreen. Aucun test du code numérique 13. `WorldInput.lua`, `UnitPanel.lua`,
`InGame.lua` et MPH `ui/ingame.lua` (`:66-107` : Échap, Maj+Alt+B/J) ne testent pas Entrée.

## API Lua pour lire les touches

Utilisées par `Base/Assets/UI/Options.lua:1770-1890` :

- `Input.GetActionId(nom)` → identifiant (`Input.GetActionId("EndTurn")`) ;
- `Input.GetGestureDisplayString(actionId, index)` → texte affiché dans les options pour l'emplacement 0
  ou 1, `nil` si vide (Options fait `or false`) ;
- `Input.BindAction(actionId, index, geste)`, `Input.ClearGesture(actionId, index)`.

Table `Input` de l'exe (chaînes de `CivilizationVI_DX12.exe`) : aussi `HasGesture`, `GetGestureDisplayIcons`,
`GetActionEnabled`, `GetActionContext`, `ClearGestures`. Aucune ne rend la touche Windows d'un geste.
Signatures de `HasGesture` et `GetActionEnabled` non vérifiées (aucun script ne les appelle).

Le test le plus sûr avec ce qui est vérifié : comparer le texte affiché à celui de la touche Entrée.

```lua
Locale.Lookup("LOC_OPTIONS_KEY_RETURN")   -- "Return" / "Entrée"
```

Supposé : le texte d'un geste sans modificateur est exactement ce `Lookup` (le JSON stocke la clé
`LOC_OPTIONS_KEY_RETURN`, et les gestes composés `CLE+CLE`). À contrôler une fois en jeu par un `print`
(voir Tests).

## Correctif proposé (interface seule)

Fichier à patcher : `ActionPanel.lua` (le fichier de base, compilé via l'`include` de XP1). Ajouter à
`FIXES` dans `proxy/src/ui.rs`, sur le modèle de `patch_production_panel`. Pas de désynchronisation :
l'action passe déjà par `UI.RequestAction`. Pas de réglage : c'est un bug pur (le réglage existe déjà, c'est
celui des raccourcis du jeu).

Remplacement de texte, une occurrence (`ActionPanel.lua:1096`) :

```
		if pInputStruct:GetKey() == Keys.VK_RETURN then
```

devient

```
		if pInputStruct:GetKey() == Keys.VK_RETURN and BGD_EntreeEstActionSuivante() then
```

et, ajouté en tête du fichier :

```lua
-- Civ6-BetterGameExperience : Entrée ne fait l'« Action suivante » que si le joueur l'y a laissée.
function BGD_EntreeEstActionSuivante()
	local id = Input.GetActionId("EndTurn");
	if id == nil then return true; end			-- action absente : comportement du jeu
	local entree = Locale.Lookup("LOC_OPTIONS_KEY_RETURN");
	for i = 0, 1 do
		if Input.GetGestureDisplayString(id, i) == entree then
			return true;
		end
	end
	return false;
end
```

Effets :
- Entrée liée (défaut) : rien ne change, Maj+Entrée compris.
- Entrée déliée : le gestionnaire rend `false` pour Entrée seule, la touche passe aux autres contextes et aux actions.
  Maj+Entrée force toujours la fin du tour (demandé par l'utilisateur le 2026-11-06 : raccourci très utilisé, sans
  action réglable). La condition devient `Keys.VK_RETURN and (pInputStruct:IsShiftDown() or BGD_EntreeEstActionSuivante())`.
- Entrée liée à une autre action (par exemple `NextUnit`) : cette action reçoit enfin la touche.
- L'action `EndTurn` liée à une autre touche continue de passer par `OnInputActionTriggered`, inchangé.

Le test est fait à chaque appui sur Entrée seulement, donc sans coût, et suit un changement d'options sans
recharger l'interface.

Les écrans qui mangent Entrée pour protéger l'action panel (TechTree, CivicsTree, GovernmentScreen) n'ont
pas besoin d'être modifiés.

Alternative écartée : supprimer le gestionnaire et ne garder que l'action. Elle perd Maj+Entrée, et l'action
est du contexte `World` alors que le gestionnaire voit la touche dans tous les contextes : le comportement
changerait pour les joueurs qui gardent Entrée.

## Tests

1. Vérifier le texte comparé : dans un bloc de débogage ou FireTuner,
   `print(Input.GetGestureDisplayString(Input.GetActionId("EndTurn"), 0), Locale.Lookup("LOC_OPTIONS_KEY_RETURN"))`,
   Entrée liée, en français et en anglais. Les deux doivent être égaux. Sinon, comparer avec
   `string.find(texte, entree, 1, true)` en excluant un `+`.
2. Options par défaut : Entrée passe aux unités puis finit le tour ; Maj+Entrée force le tour.
3. Effacer « Action suivante », valider : Entrée ne fait plus rien en jeu, Maj+Entrée force toujours le tour.
4. Lier « Action suivante » à une autre touche : cette touche marche, Entrée non.
5. Lier Entrée à « Unité prête suivante » : Entrée change d'unité sans finir le tour.
6. Écrans des technologies, des doctrines, du congrès, fenêtres modales : Entrée valide comme avant.
