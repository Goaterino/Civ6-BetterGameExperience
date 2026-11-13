# BetterGameExperience (BGE)

[Français](README.fr.md)

Fixes and speed-ups for **Civilization VI: Gathering Storm**, delivered as two DLLs placed next to
the game files. No game file is replaced, so Steam file checks and game updates leave it alone.

- Faster turns: the war-declaration cascades and AI checks that slow down big multiplayer turns are
  batched and cached, with results identical to the base game.
- Fixes for long-standing game bugs: worked tiles, attack buttons hidden behind unit flags, production
  turn counter, wonders dropped from the queue, stale top bar, Envoys screen, Enter key, and more.
- **No First/Last moves**: a game setting, on by default, that makes every player start and end each
  turn at the same moment.
- Popular interface mods built in and fixed: Better Report Screen, Extended Policy Cards, TopPanel
  Extension Pro, NotificationClear, Better Builder Charges Tracking.
- Works with DirectX 11 and DirectX 12, [Better Balanced Game](https://steamcommunity.com/sharedfiles/filedetails/?id=2865001760),
  Better Spectator Mod and MPH.
- Available in the 11 languages of Better Balanced Game: English, Français, Deutsch, Español,
  Italiano, 日本語, 한국어, Polski, Português (Brasil), Русский and 简体中文.

The full list of changes is in the [changelog](CHANGELOG.md), also available in
[Français](changelog/fr.md), [Deutsch](changelog/de.md), [Español](changelog/es.md),
[Italiano](changelog/it.md), [日本語](changelog/ja.md), [한국어](changelog/ko.md),
[Polski](changelog/pl.md), [Português (Brasil)](changelog/pt-BR.md), [Русский](changelog/ru.md) and
[简体中文](changelog/zh-CN.md).

## Install

1. Close Civilization VI.
2. Download `BetterGameExperience_V<version>.exe` from the [latest release](https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest)
   and run it.
3. Click **Install**. The installer finds the game in your Steam libraries, downloads the release,
   and checks its signature and the SHA-256 of every file before copying anything. Windows may ask
   for administrator rights if the game is in `Program Files`.

That's it: start the game as usual. A **BetterGameExperience** tab now appears in the game Options.

**Updates.** When a new version is out, the game's main menu offers it. **Update** closes the game,
installs it and offers to restart. You can also run the installer again at any time: a newer installer
removes the older ones from its folder.

**Uninstall.** Run the installer and click **Uninstall**. The game gets its original files back.

## In multiplayer

Most of BetterGameExperience only affects your own machine: you can play with people who don't have it, and
nothing desyncs.

A few fixes change the game rules. They're marked **[all players]** in the changelog, and they only
turn on when every human player has the same BetterGameExperience version. The lobby checks this on its
own. When the game starts, a `BGE -` message in the chat says whether these fixes are on, and who is
missing the mod. Players without the mod read these messages in English. Players who have it see
them in their own language.

**No First/Last moves (BGE)** is ticked by default in the game options; the host can untick it before
launching. Every player needs BetterGameExperience, at the same version.

**Disable Advanced UI indicators (BGE)**, just below, is unticked by default. When the host ticks it,
no player sees the research and civic progress (**x/y (+z)**) or the stored production of cities,
whatever their own settings.

**Team pings (BGE)** is off by default for new network games. The host can turn it on in game setup.
It uses the same lobby version check as No First/Last moves, including connected observers.
Tap **X** over the map to mark a tile, or hold **X**, move toward a call and release. Up is danger,
right is on my way, down is assist, and left is enemy missing. **Alt+1–4** send missing, danger,
on my way and assist directly. Change the wheel or individual shortcuts in **Options → Key Bindings**. Teammates see the sender’s name for four seconds. Spectators see both teams.
Adjust **Team ping volume** under **Audio**. Type `/mute name` or `/unmute name` in chat to control
both chat and pings from that player. A blocked ping gives a silent local animation.
[Editing ping types, icons and sounds](proxy/src/mods/pings/README.md).

## Settings

Everything can be changed in **Options → BetterGameExperience**: one setting per feature, and a master
switch that turns the whole mod off without uninstalling it. Hover a setting to see what it does.
The settings are stored in `rollover_proxy.cfg`, next to `BGD_GameCore_XP2.dll`.

## Something went wrong?

- The log is at `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`. Please attach it to any bug
  report, with the game's `Lua.log` if the problem is in the interface.
- After a game update, BetterGameExperience turns off the affected features on its own until it is updated.
  The log says so.
- Report bugs and translation fixes in the [issues](https://github.com/Goaterino/Civ6-BetterGameExperience/issues).
  How the translations work: [textes/README.md](textes/README.md).

## Credits

The built-in interface mods build on the work of their authors:

- Better Report Screen, Real Modifier Analysis and ReportsList Loader: Infixo. Original Units and
  Deals tabs: Greg Miller. Thanks: nightfearer, Tyolan, copysoul, RohBang, Gorlod, Zergus, Rac,
  Aristos.
- Extended Policy Cards: Aristos.
- TopPanel Extension Pro: 号码菌 (Azukii). Thanks: JNNR.
- NotificationClear: 336tarot.
- Better Builder Charges Tracking: wltk. Thanks: DeepLogic, 小小雨10101.

## For developers

The technical index of the repository is in French: [SOMMAIRE.md](SOMMAIRE.md). It covers the
proxies, how they hook the game, the analyses behind each fix, and how to build, test and release.
Releases are built and signed by GitHub Actions only ([release/README.md](release/README.md)).

License: [MIT](LICENSE).
