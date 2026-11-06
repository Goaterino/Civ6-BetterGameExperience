# Changelog

Other languages: [Français](changelog/fr.md) · [Deutsch](changelog/de.md) · [Español](changelog/es.md) ·
[Italiano](changelog/it.md) · [日本語](changelog/ja.md) · [한국어](changelog/ko.md) · [Polski](changelog/pl.md) ·
[Português (Brasil)](changelog/pt-BR.md) · [Русский](changelog/ru.md) · [简体中文](changelog/zh-CN.md)

## BetterGameDLLs 0.5.0: first public release

BetterGameDLLs fixes and speeds up Civilization VI: Gathering Storm with two DLLs placed next to
the game files. **No game file is replaced**, so Steam file checks and game updates don't remove
it. Works with DirectX 11 and DirectX 12, BBG, Better Spectator Mod and MPH.

Every change is listed below, grouped by category. Changes marked **[all players]** change the game
rules: they only turn on when every human player in the lobby has the same BetterGameDLLs version.
Everything else only affects your own machine. It is safe to play with people who don't have the mod.

### Installation and updates
- `BetterGameDLLs.exe` installer: install, update, repair and uninstall. It finds the game in your
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
- New **BetterGameDLLs** tab in the game Options. It has a master switch that turns everything off
  without uninstalling, a setting for every feature, and advanced options hidden by default. It tells
  you when a change needs a restart.
- Credits for the authors of the built-in interface mods.
- All in-game text is available in the 11 languages of Better Balanced Game: English, French, German,
  Spanish, Italian, Japanese, Korean, Polish, Brazilian Portuguese, Russian and Simplified Chinese.

### Multiplayer
- **Lobby check**: every machine reports its BetterGameDLLs version, and the host decides for
  everyone. Chat messages prefixed `BGD -` tell who is missing the mod or has another version.
  Rejoining players are handled. If someone joins without the mod while **[all players]** fixes are
  on, the host gets a warning.
- `BGD -` chat messages are sent in English, so players without the mod can read them. Players who
  have the mod see them in their own game language.
- **"No First/Last moves (BGD)" game option** **[all players]**:
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
Each one can be turned off in the BetterGameDLLs tab. If you also have the Workshop mod, the built-in
version from BGD replaces it.
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
  their own until BetterGameDLLs is updated.
- A player who joins mid-game without the mod while **[all players]** fixes are on will desync. The
  host is warned.
- Fixes marked *(experimental)* haven't been tested in a real game yet, or their impact has yet to be fully measured.
- Translations other than English and French were made without a native speaker's review. Corrections
  are welcome.
