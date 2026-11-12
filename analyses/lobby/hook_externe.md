# External hook: creating and joining a lobby from a draft site

Follows [the lobby analysis](README.md#pistes-pour-automatiser-la-création), which establishes that
creating and configuring a lobby is done entirely through the front-end Lua API, on the host side.
What remains is to choose **how the order arrives from outside**, and **how the game receives it**.

Design of 2026-11-04 (first target: [CivUp](https://github.com/Rettend/civup), a draft bot and
Discord activity), revised on 2026-11-12 for any draft site. Nothing is implemented.

## The intended flow

1. The players run the draft on the site.
2. Once the draft is over, one of them clicks "Host the game": the site opens
   `civ6bge://creation_salon?data=…`, and their game creates the configured lobby.
3. That player's proxy sends the lobby's `steam://joinlobby/...` link back to the site
   (`feedback_url`).
4. The others click "Join": the site opens `civ6bge://rejoindre_salon?steamurl=…`, and their game,
   if it is open, joins the lobby.

The site only knows two links and one HTTPS route. The settings specific to each community are
**presets maintained by BGE**, which the site refers to by name.

## Architecture: a frozen relay, all the logic in the proxy

The installer does not update itself: a release's manifest lists only `winmm.dll` and
`BGD_GameCore_XP2.dll` ([release/manifest.py](../../release/manifest.py)), and an update started
from the game (`--after-game`) goes through the copy of the installer already in place. Only a
downloaded and opened `BetterGameExperience_V<x>.exe` replaces that copy (`copy_self`). The startup
proxy, on the other hand, changes with every release.

Hence the split:
- **the installer is a dumb relay, written once and for all**: it hands the URI to the game without
  reading it;
- **the startup proxy does all the checks**: scheme, verbs, format versions, presets, expiry,
  confirmation. The scheme can grow without touching the installer.

### The registry

Three values under `HKCU`, no administrator rights needed:

```
HKCU\Software\Classes\civ6bge
    (default)    = "URL:BetterGameExperience"
    URL Protocol = ""          ← the empty value: this is what makes it a scheme
HKCU\Software\Classes\civ6bge\shell\open\command
    (default)    = "%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe" --uri "%1"
```

The command always points to **the copy at the fixed location**, never to the downloaded
`BetterGameExperience_V<x>.exe` (which `remove_old_installers` deletes anyway). It never changes: a
new release does not have to rewrite the registry.

The argument must be `--uri "%1"`, not `"%1"` alone: a first argument that does not start with `--`
is taken as the `%command%` of the Steam launch option (`launch(&args)`,
[main.rs](../../updater/src/main.rs)) and launched as the game.

### The relay (`--uri`)

What is frozen, and must not change once published:
- the `--uri` branch comes **first of all** in `main()`, before `migrate_data_dir`, `lang::init` and
  `copy_self`, with no window and no network;
- it writes the string it received, raw, to `%LOCALAPPDATA%\BetterGameExperience\uri.txt`: first to
  `uri.txt.tmp`, then by renaming, so that the proxy never reads a half-written file. A file already
  present is replaced: **the last click wins**;
- it exits immediately.

No checks: neither on the scheme nor on the size (the Windows command line already caps at 32,767
characters). The installer's manifest is `asInvoker`: a click does not ask for elevation.

When the game is closed, the relay does not start it: the file waits for the next launch, and the
proxy then decides whether it is stale.

### Bootstrapping by the installer

An `assurer_amorcage()` step, independent of the state of the DLLs, on **every** launch of the
installer (window, `--after-game`, Steam launch option; not with `--uri`):

1. `copy_self()` to `%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe`, as today;
2. **check that the copy has the bytes of the running exe.** If the old copy is locked (the Steam
   launch option keeps it open during the game), `std::fs::copy` fails silently, and the key would
   point to an exe that does not know `--uri`: every click would open the installer window.
   Different copy: no key, one log line, and another try on the next launch;
3. write the key if it is missing or differs from the expected command.

**None of this blocks**: no window, no elevation, no network. A missing key does **not** move the
installation to "needs repair" (which would lead to reinstalling the DLLs, as administrator): with
the DLLs up to date, the window says "up to date", and the bootstrap was done without asking
anything. Uninstalling removes the key.

A player who only updates through the in-game notification never receives the new installer, so
never gets the key. The proxy sees it (key missing) and the main menu says so once: "open the latest
installer once to enable civ6bge:// links".

### Watching `uri.txt`, on the proxy side

A **Rust thread of the startup proxy**, started on load:
- every second, it reads only the file's date and size (`GetFileAttributesExW`).
  `FindFirstChangeNotificationW` on the folder, with a one-second timeout, reacts at once and keeps
  the second as a safety net;
- on a change, it reads the file (capped at 64 KB) and stores it in a shared slot, under a lock, with
  a generation counter;
- no `unwrap`: `panic = "abort"` would kill the game. Every error is logged, then ignored.

On the Lua side, a native `Civ6Proxy_Uri()` returns the generation and the content; a second one,
`Civ6Proxy_UriFinie(etat)`, deletes the file. The contexts concerned query it from their `SetUpdate`,
accumulating `dt` up to about one second. A context has **only one** `SetUpdate`, and
[maj.lua](../../startup_proxy/src/maj.lua) already sets one on `MainMenu`: they must be chained,
otherwise the last one set overwrites the other.

The file is deleted only once the URI has been applied, refused or found stale: a crash in between
does not lose it.

## When a URI is handled

| where the player is | what happens |
|---|---|
| main menu | handled at once, no popup |
| in a lobby | **confirmation popup**, then handled (see below) |
| launch committed (countdown, `Network.LaunchGame`, loading) | waits |
| in a game | waits, silently: one log line, no popup |

By construction: only the main menu and staging room contexts query `Civ6Proxy_Uri()`. No in-game
context does, so a click during a game cannot make a player leave it.

A waiting URI is handled on the return to the menu or to a lobby if it is still fresh: the recipe's
`expire` if it has one, otherwise ten minutes after the file's date. Otherwise it is deleted and
logged, without showing anything: a click forgotten during a three-hour game must not make a lobby
pop up at the end.

**The popup is only for this case**: the player is already in a lobby, and the link will change it
or make them leave it.
- `creation_salon`, **host** of the open lobby: "apply the draft to this lobby?". Neither `HostGame`
  nor `LeaveGame` (see [Applying to a lobby that is already open](#applying-to-a-lobby-that-is-already-open));
- `creation_salon`, **client** in someone else's lobby: "leave this lobby and host the draft's
  one?", then a clean `Network.LeaveGame()` and the same steps as from the menu;
- `rejoindre_salon` while in another lobby: "leave this lobby and join that one?". The link of the
  lobby one is already in is ignored.

A refusal deletes the URI.

## The links

### `civ6bge://creation_salon?data=<base64url JSON>`

The whole recipe fits in the link: no server to query, any site or message can carry it. Twelve
slots with plain identifiers fit under 2,000 characters.

```json
{
  "v": 1,
  "preset": "cpl-ffa",
  "nom": "Draft #4812",
  "feedback_url": "https://civdraft.example/api/draft/4812/lobby",
  "feedback_token": "<opaque token provided by the site>",
  "expire": "2026-11-12T22:00:00Z",
  "overrides": { "MAP_SCRIPT": "Pangaea.lua" },
  "emplacements": [
    { "id": 0, "leader": "LEADER_TRAJAN", "civ": "CIVILIZATION_ROME", "equipe": 0, "steam": "76561198000000001" },
    { "id": 1, "leader": "LEADER_GANDHI", "civ": "CIVILIZATION_INDIA", "equipe": 1, "attendu": "Zoé" }
  ]
}
```

- `v`: format version. A version unknown to the proxy shows "update BGE".
- `preset`: name of a BGE preset (see [The presets](#per-community-presets)). Unknown: the lobby is
  not created, and a message says so.
- `overrides`: only the keys the preset declares as modifiable. Any other key is logged, not
  written.
- `emplacements`: the draft result. `equipe` serves team modes (`PlayerConfigurations[id]:SetTeam`);
  `attendu` is a display name; `steam` is optional (see [The optional Steam ID](#the-optional-steam-id)).
- `feedback_url` / `feedback_token`: where to send the lobby link back, and what ties it to the draft
  on the site side (see [Sending the link back](#sending-the-link-back-feedback_url)). Optional.
- `expire`: past this moment, the recipe is deleted without showing anything.

**Bans do not go down into the game.** They belong to the draft, which takes place entirely on the
site; once the leaders are assigned, there is nothing left to forbid. MPH's `BAN_*` and `DRAFT_MAP`
keys serve its chat-based draft: the recipe does not touch them.

**What we expect from the site**: the game's internal identifiers (`LEADER_ABRAHAM_LINCOLN`,
`CIVILIZATION_AMERICA`), not display names or *slugs*. CivUp, for example, derives its slugs from
the English names ([sync-bbg-leaders.ts](https://github.com/Rettend/civup/blob/main/packages/game/scripts/sync-bbg-leaders.ts)),
but its script already reads `LeaderType` and `CivilizationType` from the game database: it only
needs to keep them. Same for maps: script file names (`Continents.lua`), not slugs.

### `civ6bge://rejoindre_salon?steamurl=steam://joinlobby/289070/<lobbyId>/<steamId64>`

- **Strict form**: appid 289070 and two numeric identifiers, nothing else. The proxy only ever hands
  Steam a `joinlobby` link **rebuilt** from these three numbers.
- **To join**, the rebuilt URL is handed to Steam (`ShellExecute`): this is the path a player takes
  today when pasting the link, and Steam passes the request to the open game as an invite. To be
  checked in game, from the main menu and from another lobby.
- The site can offer the raw Steam link as a fallback, to players without BGE. What BGE adds: the
  guard against a click during a game, leaving another lobby cleanly, and later the automatic move
  to the drafted slot.

## Per-community presets

Settings vary from one community to another; they change rarely. They are therefore **embedded in
the startup proxy**, and follow the releases.

A preset contains:
- the game and map settings (including `MPH_PRESET` if the community uses it);
- the required mods (BBG version…);
- the lobby type and visibility (Internet, private or public);
- the keys the site may change (`overrides`);
- the **hosts allowed for `feedback_url`**.

A required mod missing: the lobby is not created, and a message says which one to enable. Open
point no. 6 (enabling a mod from the front-end, through the `Modding.*` API) will tell whether we can
do better.

## Sending the link back (`feedback_url`)

As soon as the lobby's `steam://joinlobby/289070/<lobbyId>/<steamId64>` link is known, the proxy
sends it:
- JSON `POST`: `{ "token": <feedback_token>, "steam_url": <link>, "bge": <version> }`;
- HTTPS required;
- through `curl.exe`, with no window, like [maj.rs](../../startup_proxy/src/maj.rs); 20 s timeout,
  two retries, response ignored beyond the HTTP status code;
- **only if the `feedback_url` host is in the preset's list.** Otherwise the lobby is created,
  nothing is sent, and a message says so. A link forged by some random page therefore does not send
  the lobby link (host's Steam ID included) to a stranger. The preset list also serves as a registry
  of trusted sites: a community that wants the automatic link-back asks to be added.

The link-back only sends what the players in the lobby already see: the link. No local path, no mod
list.

## Creating the lobby from the menu

The order comes from the pitfalls found in the analysis:

1. `GameConfiguration.SetToDefaults(gameMode)`;
2. `Network.HostGame(serverType)` according to the preset;
3. wait for `MultiplayerJoinGameComplete` — nothing is written before;
4. `MPH_PRESET` **first**, then the rest: the MPH preset cascades its `ConfigurationUpdates`, and
   `CheckPreset` overwrites the lists of whoever wrote before it (L2);
5. write through `SetupParameters:SetParameterValue`, not through raw `SetValue`, so that domains
   are revalidated at once and a refresh undoes nothing;
6. recheck the values sensitive to the map size range — `CITY_STATE_COUNT` falls back to 0 outside
   `[MAP_MIN_MINOR_PLAYERS, MAP_MAX_MINOR_PLAYERS]` (L2b) — and log any gap between the recipe and
   the resulting config;
7. **a single** broadcast at the end: `Network.BroadcastGameConfig(true)`, then one
   `BroadcastPlayerInfo(id)` per slot touched, and only for non-human slots;
8. log the applied recipe, key by key, and the result;
9. read the lobby link, and send it to `feedback_url`.

**Who writes a player's leader, and when.** Since the draft is played in advance, the host writes
**all** the slots right after `HostGame`, while they are still AI or open: no human is there to
contradict (L1). A player who joins receives the host's full config, then rebroadcasts their own
slot, with the leader they just received: no harm. What remains is the race of conclusion 4 (a
rebroadcast sent from an earlier state). Hence a **check after each arrival**: compare the slot's
leader and team with the recipe, and rebroadcast only if they drifted, **at most twice per slot**.
The anonymous-mode ping-pong (L5) shows what an uncapped correction costs.

The Lua block goes at the end of `hostgame.lua` (dispatch by file name: MPH's replacement is caught
too), the context that carries `HostGame`, `CheckPreset` and the `SetupParameters`. Watching
`uri.txt` lives in the `mainmenu.lua` and `stagingroom.lua` blocks, which talk to it through
`LuaEvents`. The staging room already tracks the slots for the POL verdict.

## Applying to a lobby that is already open

The case of the host who confirms the popup. Humans are already connected:
- their slots are **never** written by the host (L1);
- leaders and teams are written on AI or open slots only;
- the Steam IDs in the recipe trigger the moves (next section);
- `MPH_PRESET` first, then the rest, and a single broadcast at the end, as from the menu;
- the lobby link is sent to `feedback_url`.

## The optional Steam ID

The host knows each player's Steam ID in Lua: `PlayerConfigurations[id]:GetNetworkIdentifer()`,
already used by [rapport.lua](../../startup_proxy/src/rapport.lua).

**Without `steam` in the recipe**, leaders and teams are written on the slots, and everyone takes
theirs by hand, as today.

**With `steam`**, on all slots or only some:
- **the host**: if their Steam ID is in the recipe, their entry is **renumbered as slot 0** before
  writing. No swap: the slot order carries nothing that the recipe does not rewrite;
- **a player who joins** lands on the first open slot, which the host does not choose. If they are
  expected elsewhere:
  - **with BGE**, the host sends them "slot N" through the lobby's BGE announcement channel, and it
    is their own machine that calls `Network.RequestPlayerIDChange(N)`. The host accepts it: the
    target slot is AI or open. The host never writes a connected human's slot (L1);
  - **without BGE**, the staging room shows "Zoé is expected in 3", and the player moves by hand;
- slots without `steam` stay free: first come, first served;
- a Steam ID absent from the recipe is **flagged** to the host. A recipe kicks nobody and does not
  block the launch;
- validation: 17 digits, starting with `7656119`.

## The Steam lobby link: the Steamworks API, no disassembly

This is step 3 of the flow, and **the first point to settle**: without the link, nothing goes back
to the site.

Our DLL runs **inside the game process**, and `steam_api64.dll`, which the exe imports, exposes the
Steamworks *flat API* in C. The game has already called `SteamAPI_Init`; a `GetProcAddress` is
enough. The flat functions take the `CSteamID` as a `uint64`: no C++ ABI to imitate, no SDK header
to ship.

**Getting the interfaces**, without depending on the SDK version number:

```
SteamAPI_GetHSteamUser()                                        → HSteamUser
SteamInternal_FindOrCreateUserInterface(h, "SteamUser023")      → ISteamUser*
SteamInternal_FindOrCreateUserInterface(h, "SteamFriends017")   → ISteamFriends*
```

We try the few neighbouring versions, and keep the first one that returns a non-null pointer. A
`steam_api64.dll` from an old SDK may not have these two exports: the directly exported accessors
(`SteamFriends`, `SteamUser`) serve as a fallback.

**`steamId64`**: `SteamAPI_ISteamUser_GetSteamID(user)`.

**`lobbyId`: the hard half.** The Steamworks API has **no** "which lobby am I in?" call: it is up to
the game to remember it, from `LobbyCreated_t` / `LobbyEnter_t`. Three ways:

1. `SteamAPI_ISteamFriends_GetFriendGamePlayed(friends, <our own Steam ID>, &info)` →
   `info.m_steamIDLobby`. `FriendGameInfo_t` is 24 bytes: `m_gameID` (8), `m_unGameIP` (4),
   `m_usGamePort` (2), `m_usQueryPort` (2), `m_steamIDLobby` (8, offset 16). The
   [ISteamFriends doc](https://partner.steamgames.com/doc/api/ISteamFriends) only describes the call
   for a friend: on oneself, it is undocumented. **To test first.**
2. Subscribe to `LobbyCreated_t` / `LobbyEnter_t` through `SteamAPI_RegisterCallback`, an export of
   the same DLL, as the SDK's `CCallback` template does. Nothing is hooked in the exe, at the cost
   of depending on the vtable of `CCallbackBase`, public in the SDK but internal.
3. On the site side, with no code in the game: the Web API's `GetPlayerSummaries` would return a
   `lobbysteamid` field. Valve does not document it, and it depends on profile visibility. It can be
   tested with a `curl` and a Web API key, while a player with a public profile is hosting a lobby.
   `ILobbyMatchmakingService.GetLobbyData`, the official interface, requires a publisher key: out of
   reach.

The `steam://joinlobby/<appid>/<lobbyId>/<steamId64>` format is not documented by Valve either, but
it is what players paste today, and what CivUp validates: it works.

### The probe

Neither the exe nor `steam_api64.dll` is on the development machine: this is settled in real
conditions, through a startup proxy probe shipped in a pre-release. It only reads and logs, when a
lobby is hosted (`MultiplayerJoinGameComplete`):
1. the exports present in `steam_api64.dll` (recent and old);
2. the interface version strings that answer (`SteamFriends017` and earlier, same for
   `SteamUser0xx`);
3. the Steam ID from `GetSteamID`, compared with `GetNetworkIdentifer()` on the Lua side: proof that
   we are reading the right interface;
4. `GetFriendGamePlayed(self)`: return value and `m_steamIDLobby`;
5. the rebuilt link;
6. the names of the functions in the Lua `Network` table: the exe registers 85 of them, and we may
   have missed an accessor to the lobby.

The tester hosts, a friend joins **with the logged link**. If they land in the lobby, way 1 is
validated; otherwise the log says where it breaks, and we move on to way 2.

Details of the calls, the expected log, the protocol and the next steps for each result:
[sonde_steam.md](sonde_steam.md).

## Guards

- **Never in a game**: only the menu and staging room contexts handle a URI, and never once the
  launch is committed.
- **A recipe does nothing else** than configure a lobby: no `Network.LaunchGame`, no kick, no write
  to `rollover_proxy.cfg`, no file path. Only a `LeaveGame()` confirmed in the popup makes the player
  leave a lobby.
- **Whitelist**: keys and values taken from `SetupParameters.xml` and from the `Config.xml` of the
  installed mods, plus the `overrides` declared by the preset. An unknown key is logged, not written.
- **Link-back** limited to the preset's hosts, over HTTPS; responses capped (64 KB), 20 s timeout.
- **A single waiting URI**, deleted as soon as it is applied, refused or stale.
- **Registry** written by the installer only, removed on uninstall.
- **Setting in the BetterGameExperience tab** (required for every feature):
  `liens_civ6bge = true|false`, default `true`, `libre = true` like `verif_maj`.

## To check before writing code

1. **The lobby link**: the probe, way 1 then 2. This is the lock on step 3.
2. **`rejoindre_salon` through `ShellExecute`**: from the main menu, and from another lobby.
3. **`serverType` outside the UI**: `ServerTypeForMPLobbyType` depends on the mode set by
   `LuaEvents.ChangeMPLobbyMode`; confirm that we can host on the Internet from the menu without
   going through the `Lobby` screen.
4. **The 76 `NetPlayerInfo` sent when hosting** (test of 2026-11-04): where they come from, and
   whether a recipe write right after crosses them.
5. **The launch commitment**: which staging room event marks the start of the countdown, to stop
   handling URIs there.
6. **Required mods**: does the `Modding.*` API allow enabling a mod from the front-end? Otherwise,
   message and refusal.

## Plan by stages

| stage | content | depends on |
|---|---|---|
| 0 | **The Steam probe**, in a pre-release ([sonde_steam.md](sonde_steam.md)) | a Windows machine with the game and Steam |
| 1 | **The relay and the bootstrap**: `--uri`, `uri.txt`, `assurer_amorcage()`, removal on uninstall; watching `uri.txt` from the proxy (thread, natives) | nothing |
| 2 | **`creation_salon`**: format v1, embedded presets, applying from the menu and in a lobby, popup; Rust tests (decoding, versions, expiry, size) and Lua tests (in the style of [test_salon.lua](../../startup_proxy/tests/test_salon.lua)) | stage 1 |
| 3 | **`feedback_url`** and **`rejoindre_salon`** | stages 0 and 2 |
| 4 | **Steam ID**: renumbering the host, moving players who have BGE, display for the others | stage 2 |

The relay (stage 1) is the only part in the installer: once published, it no longer changes.
Everything else follows the proxy releases.

## Ruled-out options

- **Local micro-server in the game** (`127.0.0.1`, called with `fetch` by the page). A Discord
  activity is an iframe served through Discord's proxy, which cannot reach `127.0.0.1`. In a normal
  tab, it would have to get past Chrome's local network policy (preflight
  `Access-Control-Allow-Private-Network`, permission asked of the user), find a port, and keep a
  listener open in the game process. The file relay does the same without opening anything. If a
  local dashboard ever comes: `127.0.0.1` only, `Origin` whitelist, `Host` header check (against DNS
  rebinding), no `unwrap`.
- **A link that only carries a token, and pairing with the site** (design of 2026-11-04). It assumed
  a server contract specific to each site. The recipe in the link and the `feedback_url` give the
  same flow without pairing.
- **A recipe downloaded from a URL in the link.** Any page could make the game call an arbitrary
  host.
- **An installer that updates itself.** Not needed here: the relay no longer changes, and the logic
  lives in the proxy.
- **Placing players without BGE by the host**, which would rewrite their slot after they arrive:
  that is the L1 race.

## CivUp's Discord activity

CivUp opens its external links through `discordSdk.commands.openExternalLink`, and **falls back to
the clipboard** when opening fails
([platform/external-links.ts](https://github.com/Rettend/civup/blob/main/apps/activity/src/client/platform/external-links.ts)).
It remains to measure whether a custom scheme goes through in the Discord client (`steam://` first,
then `civ6bge://`). Outside the activity (bot message, browser tab), a clickable link works without
reservation. On the first click, the browser asks "Open BetterGameExperience?", with an "always
allow" checkbox.
