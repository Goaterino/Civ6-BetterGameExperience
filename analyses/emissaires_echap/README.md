# Écran des émissaires : Échap ne lève pas le bloqueur de fin de tour

**Corrigé dans le proxy** (2026-11-06, interface seule, sans réglage) : `patch_city_states` dans [proxy/src/ui.rs](../../proxy/src/ui.rs) : `OnClose` remis dans le gestionnaire d'Échap de la copie de BSM. Confirmé en partie par l'utilisateur le 2026-11-06.

Question : pourquoi, après avoir fermé l'écran des cités-États avec Échap, l'action « Envoyer des
émissaires » (`ENDTURN_BLOCKING_GIVE_INFLUENCE_TOKEN`) reste-t-elle en place et empêche-t-elle de
passer le tour, alors que la croix en haut à droite la lève ? Analyse statique du 2026-11-06 des
scripts Lua de Base, XP1, XP2 et des mods du Workshop. Rien n'a été reproduit en partie : voir
[À vérifier](#à-vérifier).

## Conclusion

**Le défaut vient de Better Spectator Mod (BSM), pas du jeu.** Sa copie de `CityStates.lua` appelle
le gestionnaire de touches de l'animation sans fonction de fermeture. Échap fait alors seulement
glisser le panneau hors de l'écran. Il n'appelle pas `Close()`, qui est la seule fonction qui :

1. vide les émissaires en attente (`m_kEnvoyChanges = {}`, la « prévisualisation ») ;
2. marque les émissaires comme « examinés » (`SetGivingTokensConsidered(true)`), ce qui lève le
   bloqueur de fin de tour ;
3. joue le son de fermeture.

La croix appelle `OnClose` → `Close()`, d'où la différence. Dans la version de Firaxis, Échap appelle
aussi `OnClose` : le jeu de base n'a pas ce défaut.

Correctif proposé : un remplacement de texte d'une ligne dans `CityStates.lua`, par le proxy.
Interface seule, sans désynchronisation (`SetGivingTokensConsidered` est déjà appelé par la croix, sur
le même chemin). Bug pur : pas de réglage.

## Quel fichier est chargé

Le contexte `CityStates` est déclaré dans `InGame.xml` (`Base/Assets/UI/InGame.xml:50`, et dans la copie
de MPH `3307282026/ui/ingame.xml:58`). Avec XP2, `Expansion2_CityStates` (ReplaceUIScript) charge
`DLC/Expansion2/UI/Replacements/CityStates_Expansion2.lua`, qui fait `include("CityStates_Expansion1")`,
qui fait `include("CityStates")` (`CityStates_Expansion1.lua:9`).

Ce `CityStates.lua` est :
- celui de Firaxis, `Base/Assets/UI/PartialScreens/CityStates.lua`, sans BSM ;
- celui de BSM, `1916397407/UI/CityStates.lua`, quand BSM est dans la partie : ImportFiles
  `Spectator_CityStates`, LoadOrder 9999, sans critère (`Better Spectator Mod v1.modinfo:193-198`). Il
  s'annonce dans Lua.log par `CityState Panel for Better Spectator Mod` (ligne 14).

Aucun autre mod du Workshop installé ne remplace `CityStates.lua` (recherche dans tous les `.modinfo`).
BBG ne touche que les données (`sql/Base/CityStates.sql`). Les fichiers XP1 et XP2 ne redéfinissent ni
`Close`, ni `OnClose`, ni `OnInputHandler` : ils enveloppent seulement `GetData`, `AddCityStateRow`,
`AddInfluenceRow` et `GetSuzerainBonusText`.

## Les deux chemins de fermeture

### La croix

`Controls.CloseListButton:RegisterCallback( Mouse.eLClick, OnClose )` (Base `:1804`, BSM `:1858`).

```lua
function OnClose()          -- Base :463, BSM :472
	Close();
end

function Close()            -- Base :441, BSM :450
	m_kEnvoyChanges = {};		-- Zero out any pending envoy choices
	...
	if (... not localPlayer:GetInfluence():IsGivingTokensConsidered()) then
		localPlayer:GetInfluence():SetGivingTokensConsidered(true);
	end
	m_kScreenSlideAnim.Hide();
end
```

### Échap

`ContextPtr:SetInputHandler(OnInputHandler, true)` (Base `:1816`, BSM `:1870`).

Firaxis (`Base/Assets/UI/PartialScreens/CityStates.lua:1619-1622`) :

```lua
function OnInputHandler( input:table )
	return m_kScreenSlideAnim.OnInputHandler( input, OnClose );
end
```

BSM (`1916397407/UI/CityStates.lua:1659-1662`) :

```lua
function OnInputHandler( input:table )
	return m_kScreenSlideAnim.OnInputHandler( input );
end
```

Dans `Base/Assets/UI/AnimSidePanelSupport.lua`, `kAnim.OnInputHandler(input, closeCallback)` traite
Échap (KeyUp) ainsi :

```lua
if closeCallback ~= nil then closeCallback(); else kAnim.Hide(); end
return true;
```

et `kAnim.Hide()` n'appelle que `kAnim["_closeCallback"]`, le second argument de
`CreateScreenAnimation`. BSM le crée sans (`CreateScreenAnimation( Controls.SlideAnim )`, `:1855`).
Avec BSM, Échap ne fait donc qu'animer la sortie puis `ContextPtr:SetHide(true)`. Le reste du diff
BSM / Firaxis ne concerne que les spectateurs (`SPEC_ID_*`, joueurs anonymes) : la perte du
`OnClose` n'a aucune raison d'être.

Les autres fermetures passent toutes par `Close()`, avec ou sans BSM : bouton de la barre de
lancement (`PartialScreenHooks_CloseCityStates`, aussi dans `PartialScreenHooks_Spectator.lua:110`),
`PartialScreenHooks_CloseAllExcept`, `WorldRankings_CloseCityStates`, `LocalPlayerChanged`, passage en
lentille modale (`OnInterfaceModeChanged`), envoi du dernier émissaire (`OnConfirmPlacement`).

## Pourquoi on ne peut plus passer le tour

`ENDTURN_BLOCKING_GIVE_INFLUENCE_TOKEN` reste actif tant que le joueur a des émissaires et que
`IsGivingTokensConsidered()` est faux. Seul `Close()` le passe à vrai. Après Échap sous BSM :

1. le bouton de fin de tour, ou Entrée (`ActionPanel.lua:1096`, `DoEndTurn`), trouve ce bloqueur
   (`ActionPanel.lua:526-597`) ;
2. il active la notification (`LuaEvents.ActionPanel_ActivateNotification`) ;
3. `OnGiveInfluenceTokenActivate` (`NotificationPanel.lua:1338`) envoie
   `NotificationPanel_OpenCityStatesSendEnvoys` ;
4. `OnOpenSendEnvoys` rouvre l'écran des émissaires, avec les émissaires en attente d'avant, puisque
   `m_kEnvoyChanges` n'a pas été vidé.

Le joueur tourne en rond : fin de tour → écran rouvert → Échap → fin de tour… Seule la croix (ou
l'envoi de tous les émissaires) en sort.

### La touche Espace

L'écran ne traite aucune autre touche qu'Échap : `OnInputHandler` renvoie `false`, donc Espace et
Entrée continuent vers les autres contextes. Dans `InputSettings.json` de l'utilisateur
(`%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI`), Espace est lié à `SkipTurn` (passer le tour
de l'unité) et `EndTurn` n'a aucune touche ; la fin de tour au clavier est Entrée, câblée en dur dans
`ActionPanel.lua:1096`. L'impression que l'écran « ignore » la touche de fin de tour s'explique par la
boucle ci-dessus : la touche marche, mais elle retombe sur le bloqueur et rouvre l'écran (déjà ouvert,
donc rien ne bouge). Supposé : rien dans le code ne bloque Espace en particulier.

## Correctif proposé

Dans le proxy, une entrée de plus dans `FIXES` (`proxy/src/ui.rs:498`) pour `CityStates.lua`.
`is_file` compare le nom exact : `CityStates_Expansion1.lua` et `CityStates_Expansion2.lua` ne sont pas
concernés, et le chunk de `include("CityStates")` passe bien par `luaL_loadbuffer`, comme `CityPanel.lua`
inclus par ses remplaçants.

```rust
const ESC_BSM: &str = "m_kScreenSlideAnim.OnInputHandler( input );";
const ESC_OK: &str = "m_kScreenSlideAnim.OnInputHandler( input, OnClose );";

/// Better Spectator Mod : Échap fermait l'écran des cités-États sans `Close()`, donc sans vider les
/// émissaires en attente ni lever le bloqueur de fin de tour. On rend `OnClose` à l'animation, comme
/// dans la version de Firaxis.
pub fn patch_city_states(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "pas en UTF-8".to_string())?;
    if text.contains(ESC_OK) {
        return Err("déjà correct (version de Firaxis)".into());
    }
    if !text.contains(ESC_BSM) {
        return Err("OnInputHandler introuvable".into());
    }
    Ok(text.replacen(ESC_BSM, ESC_OK, 1).into_bytes())
}
```

```rust
("CityStates.lua", patch_city_states, "Échap ferme l'écran des cités-États comme la croix"),
```

Les deux fichiers commencent par un BOM UTF-8, valide pour `from_utf8`. Pour ne pas réécrire les octets,
on peut aussi chercher et remplacer sur `&[u8]`, comme `patch_production_panel`.

Variante plus tolérante à la mise en forme : ajouter à la fin du fichier un bloc qui réinstalle le
gestionnaire (les locales du fichier sont visibles, comme pour `ui_production.lua`) :

```lua
-- Civ6BetterGameDLLs : Échap ferme l'écran comme la croix (Better Spectator Mod l'avait perdu)
if m_kScreenSlideAnim ~= nil then
	ContextPtr:SetInputHandler(function(input)
		return m_kScreenSlideAnim.OnInputHandler(input, OnClose);
	end, true);
end
```

Elle est sans effet sur la version de Firaxis (même comportement) et sur une future version corrigée
de BSM. Le remplacement d'une ligne reste préférable : il dit dans le journal s'il a servi.

Caractéristiques :
- **interface seule** : `SetGivingTokensConsidered` est déjà appelé par la croix chez tous les joueurs,
  avec ou sans proxy. Pas de POL, pas de désynchronisation ;
- **pas de réglage** : bug pur ;
- s'applique quel que soit le fournisseur de `CityStates.lua` ; seul BSM en a besoin aujourd'hui.

## À vérifier

- Reproduire en partie avec BSM : Échap sur l'écran des émissaires, puis fin de tour. Avant : l'écran
  se rouvre. Après : le tour passe. Vérifier `CityState Panel for Better Spectator Mod` dans Lua.log et
  la ligne du proxy dans son journal.
- La dernière partie de Lua.log / Modding.log (2026-11-06) n'avait pas BSM dans ses mods cibles (seul son
  écran de chargement, action FrontEnd, était actif). Le bug a donc dû être vu dans une partie avec
  BSM. Si l'utilisateur le revoit **sans** BSM, la cause est ailleurs : chercher alors un contexte qui
  consommerait Échap avant `CityStates` (aucun trouvé dans MPH, BBG, BBG Expanded, Better Report Screen
  ni le proxy).
