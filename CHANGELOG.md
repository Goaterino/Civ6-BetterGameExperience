# Changelog

Other languages: [Français](changelog/fr.md) · [Deutsch](changelog/de.md) · [Español](changelog/es.md) ·
[Italiano](changelog/it.md) · [日本語](changelog/ja.md) · [한국어](changelog/ko.md) · [Polski](changelog/pl.md) ·
[Português (Brasil)](changelog/pt-BR.md) · [Русский](changelog/ru.md) · [简体中文](changelog/zh-CN.md)

## BetterGameExperience 0.7.0

Multiplayer games need every human player on 0.7.0 for the **[all players]** fixes: a lobby that
mixes 0.6.x and 0.7.0 turns them off.

### Gameplay fixes [all players]
- **Stacked units**: two military units of the same class can end up on the same tile (unit bought in
  an occupied city, interrupted move, teleport…). The extra unit no longer counts in flanking and
  support bonuses: two stacked units next to the target give +2, not +4. City defense is unchanged.

### World Congress
- **Removed**: the 0.6.x fixes that kept the turn from ending during the vote (no automatic end of
  turn, no forced end of turn on an empty list of resolutions, 240 s on MPH's timer). The game's
  and MPH's timer and end of turn apply again during the vote. The **Back to the vote** button,
  the welcome screen once per session and the luxuries recalculation stay.

### Interface fixes
- **City-States screen**: envoys you have placed but not confirmed yet are no longer lost when the
  screen refreshes (another player's envoy, a quest, a war or a peace). They are only removed if
  the city-state is destroyed or your team declares war on it.
- **Deal screen**: the available Gold button works again. It could stay unclickable after an earlier
  deal (game bug).
- **Whole map from the start** (on by default): the minimap and the camera cover the whole map
  from the first turn, so you no longer need map pins at the edges of the map. Unexplored areas stay
  hidden. Purely local.
- **Movement range of a unit on an enemy district**: no more lag spike when you select it (unfinished
  enemy encampment), zones of control (red tiles) are shown, and water tiles the
  unit can't enter yet (coast before embarkation, ocean before Cartography) are no longer offered.
- **Main menu**: the version of BetterGameExperience is shown in the bottom-left corner, after the
  game's and MPH's (`… - MPH / … - BGE / 0.7.0`).

### Settings
- **Reset BGE** button in the BetterGameExperience tab: puts every setting back to its default
  value.
- New advanced diagnostic setting, off by default: **Measure movement range cost (in time)**.

## BetterGameExperience 0.6.1

Includes everything from the 0.6.0 pre-release. Multiplayer games need every human player on 0.6.0
or 0.6.1 for the **[all players]** fixes: a lobby that mixes 0.5.0 and 0.6.x turns them off.

### Installation and updates
- The installer no longer shows the Steam launch option after installing: it just tells you
  everything is in place, and you can close it. The game's update notice is enough to stay up to
  date. A launch option you already added keeps working.
- The installer is now named `BetterGameExperience_V<version>.exe`. When you run it, it deletes the
  older installers from the same folder (usually Downloads).

### Multiplayer
- **"No First/Last moves (BGE)"** **[all players]**: you now stay on the turn processing screen,
  End Turn button included, until the host starts the turn, instead of seeing your turn open with
  your orders refused.

### World Congress
- *(experimental)* **[all players]** While you wait for the other players, a **Back to the vote**
  button reopens the vote window with your votes and the right amount of Diplomatic Favor.
- *(experimental)* The turn no longer ends against your will during the vote: no automatic end of
  turn, no forced end of turn on an empty list of resolutions, and MPH's timer gives the planned
  240 s for the vote.
- *(experimental)* **[all players]** The luxuries affected by a ban are recalculated at each session.
- The World Congress welcome screen and its sound show only once per session, instead of coming
  back each time another player returns to the vote.

### Interface fixes
- With Better Spectator Mod, the top bar shows your teammates' strategic resources again, and the
  resources a teammate unlocks.

## BetterGameExperience 0.6.0 (pre-release)

Multiplayer games need every human player on 0.6.0 for the **[all players]** fixes: a lobby that
mixes 0.5.0 and 0.6.0 turns them off.

### Installation and updates
- The installer no longer shows the Steam launch option after installing: it just tells you
  everything is in place, and you can close it. The game's update notice is enough to stay up to
  date. A launch option you already added keeps working.
- The installer is now named `BetterGameExperience_V<version>.exe`. When you run it, it deletes the
  older installers from the same folder (usually Downloads).

### Multiplayer
- **"No First/Last moves (BGE)"** **[all players]**: you now stay on the turn processing screen until
  the host starts the turn, instead of seeing your turn open with your orders refused.

### World Congress [all players]
- *(experimental)* While you wait for the other players, a **Back to the vote** button reopens the
  vote window with your votes and the right amount of Diplomatic Favor.
- *(experimental)* The turn no longer ends against your will during the vote: no automatic end of
  turn, no forced end of turn on an empty list of resolutions, and MPH's timer gives the planned
  240 s for the vote.
- *(experimental)* The luxuries affected by a ban are recalculated at each session.

### Interface fixes
- With Better Spectator Mod, the top bar shows your teammates' strategic resources again, and the
  resources a teammate unlocks.

## BetterGameExperience 0.5.0: first public release

BetterGameExperience (BGE) fixes and speeds up Civilization VI: Gathering Storm with two DLLs placed next to
the game files. **No game file is replaced**, so Steam file checks and game updates don't remove
it. Works with DirectX 11 and DirectX 12, BBG, Better Spectator Mod and MPH.

Every change is listed below, grouped by category. Changes marked **[all players]** change the game
rules: they only turn on when every human player in the lobby has the same BetterGameExperience version.
Everything else only affects your own machine. It is safe to play with people who don't have the mod.

### Installation and updates
- `BetterGameExperience.exe` installer: install, update, repair and uninstall. It finds the game in your
  Steam libraries, and checks the signed release manifest and the SHA-256 of every file before
  copying anything.
- Two files are added next to the game files: `winmm.dll` and `BGD_GameCore_XP2.dll`. Older
  installs (0.3.0 and before) are cleaned up automatically. A `winmm.dll` from another mod is never
  overwritten.
- In-game update notice in the main menu, including MPH's menu. **Update** closes the game, installs
  the new version and offers to restart the game.
- "Pre-release" checkbox in the installer to test upcoming versions, with a one-click return to the
  stable release.
- Optional Steam launch option that checks for updates every time you start the game.
- Installer in 11 languages, the ones Better Balanced Game supports. It follows the Windows language,
  and you can pick another one at the bottom of the window.

### Settings
- New **BetterGameExperience** tab in the game Options. It has a master switch that turns everything off
  without uninstalling, a setting for every feature, and advanced options hidden by default. It tells
  you when a change needs a restart.
- Credits for the authors of the built-in interface mods.
- All in-game text is available in the 11 languages of Better Balanced Game: English, French, German,
  Spanish, Italian, Japanese, Korean, Polish, Brazilian Portuguese, Russian and Simplified Chinese.

### Multiplayer
- **Lobby check**: every machine reports its BetterGameExperience version, and the host decides for
  everyone. Chat messages prefixed `BGE -` tell who is missing the mod or has another version.
  Rejoining players are handled. If someone joins without the mod while **[all players]** fixes are
  on, the host gets a warning.
- `BGE -` chat messages are sent in English, so players without the mod can read them. Players who
  have the mod see them in their own game language.
- **"No First/Last moves (BGE)" game option** **[all players]**:
  - Every player starts their turn together. Orders are held, and the timer stays stopped, until
    everyone has finished loading (60 s at most).
  - The turn ends for everyone when the host's timer runs out.
  - MPH's old turn sync is turned off.
- Loaded games in MPH's lobby: city-states no longer cause "Invalid player options" and a greyed-out
  Ready button.

### Gameplay fixes [all players]
- Worked tiles: a city no longer works a 2/1 tile while a 2/2 tile is free (Food and Production
  focus).
- Better Spectator Mod spectators no longer count in tourism. They can't vote in the World Congress
  and can't be the target of a resolution. They can still view the Congress screen.
- Naval units can teleport to a city from any naval district (Harbor, Water Park, Canal…), not only from
  a city center, just like they could teleport from a city to any naval district. (no more admiral stuck on a one tile lake)

### Interface fixes
- Attack buttons (city, encampment, nuclear strike from a silo) are drawn above unit flags, where you
  can click them.
- Ranged attack from a city that isn't selected takes one click, not two.
- In city/district ranged attack targeting mode, clicking an enemy unit's flag fires at it.
- The camera no longer recenters when you click an attack button (this can be turned back on).
- The production turn counter now includes government bonuses: Fascism, Autocracy and Merchant
  Republic.
- A wonder dropped from the production queue after two swaps is put back.
- The current production properly updates when you switch to another city.
- The top bar's empire yields update after a governor action or a great person.
- *(experimental)* Keyboard shortcuts no longer stop working after a diplomacy screen closes the Esc
  menu, or after a double Congress popup.
- Esc now closes the Envoys screen properly, so pending envoys no longer block the end of the turn.
- *(experimental)* The Settler lens is redrawn after a city is founded, and tiles too close to a city
  are shown as forbidden.
- Enter follows your "Next action" key binding, meaning you can remove it for another keybind. Shift+Enter still forces the end of the turn.
- The movement range now shows for a unit on an enemy defensible district.
- *(experimental)* Map pins are no longer rebuilt when nothing changed, which means less lag in multiplayer.

### Built-in interface mods
Each one can be turned off in the BetterGameExperience tab. If you also have the Workshop mod, the built-in
version from BGE replaces it.
- **Builder charges** on unit flags (from Better Builder Charges Tracking, by wltk).
- **Trade route turns remaining** on your traders' flags (new).
- **Clear notifications** button (from NotificationClear, by 336tarot).
- **Extended top panel**: food, production, population and luxuries, with details per city (from
  TopPanel Extension Pro, by 号码菌 / Azukii).
- **Policy card effects**: each card shows its actual gain for your empire (from Extended Policy
  Cards, by Aristos). Better Report Screen is no longer needed. Fixed values for:
  - Raj, Medina Quarter, Monasticism and Civil Prestige;
  - Conscription and Levée en Masse;
  - the Praetorium and Colonial Taxes cards;
  - great person points, now shown as [icon]+x.
- **Better report screen** with Deals, Units, Policies, City-States and Cities tabs (from Better
  Report Screen, by Infixo), with the same fixes.

### Performance and comfort
- Faster turn processing:
  - During a cascade of war declarations, city and diplomacy updates happen only once. The measured
    turn was 2.4 s faster (−82 % on the cascade). => drastically reduced lag from city-states switching sides in a war.
  - Two caches speed up the AI's turn: buildable improvements and tile yields.
  - Results are identical to the base game, so these are safe with players who don't have the mod.
- Nuclear strike animation runs 20× faster when Quick Combat is on: about 0.6 s instead of 11 s.
  Display only.
- The frame limiter offers 45, 75, 90 and 100 FPS.

### Known limitations
- The mod targets the current official game build. After a game update, the affected features turn off on
  their own until BetterGameExperience is updated.
- A player who joins mid-game without the mod while **[all players]** fixes are on will desync. The
  host is warned.
- Fixes marked *(experimental)* haven't been tested in a real game yet, or their impact has yet to be fully measured.
- Translations other than English and French were made without a native speaker's review. Corrections
  are welcome.
