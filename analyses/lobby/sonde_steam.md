# Steam probe: getting the lobby link from the proxy

The flow of [hook_externe.md](hook_externe.md) needs, at its step 3, the
`steam://joinlobby/289070/<lobbyId>/<steamId64>` link of the lobby the host has just created, to
send it to the site's `feedback_url`. The Steamworks API has no "which lobby am I in?" call: we have
to settle, in real conditions, which of the ways in
[The Steam lobby link](hook_externe.md#the-steam-lobby-link-the-steamworks-api-no-disassembly)
works with the `steam_api64.dll` shipped with the game.

Design of 2026-11-12. **Nothing is written**: the probe will be built and run from a Windows machine
with the game and Steam (the development machine has neither the exe nor `steam_api64.dll`).

## What we want to know

1. Which exports `steam_api64.dll` exposes: the recent flat API, the old one, both, none.
2. Which interface version strings answer (`SteamFriends0xx`, `SteamUser0xx`,
   `SteamMatchmaking0xx`).
3. Whether `GetFriendGamePlayed` on **our own** Steam ID returns the lobby (`m_steamIDLobby`), and
   how long after hosting.
4. Whether that lobby is really ours: `GetLobbyOwner` returns our Steam ID, and a friend joins with
   the rebuilt link.
5. Whether the Lua `Network` table has an accessor to the lobby that we missed.

A single test session must answer all five questions.

## Where it lives

- **A `startup_proxy/src/steam.rs` module.** The same logic will then serve the real feature (the
  link sent to `feedback_url`): only the logging part will be removed.
- **A native `Civ6Proxy_SondeSteam()`**, registered with the other lobby natives
  ([salon.rs](../../startup_proxy/src/salon.rs)). It makes all the calls, logs, and returns the
  rebuilt link, or nil.
- **The trigger, in the block added to `stagingroom.lua`**
  ([salon.lua](../../startup_proxy/src/salon.lua)):
  - on `LuaEvents.HostGame_ShowStagingRoom` (the host), already listened to by `BGD_Ouverture`;
  - on `LuaEvents.JoiningRoom_ShowStagingRoom` (a client): the same reading on the client side will
    tell whether a joining player sees the same lobby, useful later for `rejoindre_salon`;
  - then **again at 2, 5, 10 and 30 seconds**. The game presence that `GetFriendGamePlayed` reads may
    be updated by Steam with a delay: the first reading alone is not enough to conclude. The timer
    goes through the context's `SetUpdate`, chained with any one that already exists.
- **Enabled by a `rollover_proxy.cfg` key**: `sonde_steam = true`, absent by default. It is a
  temporary diagnostic tool, not a feature for players: it does not belong in the tab, and goes away
  once the question is settled.
- **Shipped in a pre-release** (the installer's pre-release channel), with no POL version change.

## The calls, in order

All on the Lua thread (the one calling the native), which is the game's: the Steam API is not safe
from another thread.

### 1. The module

`GetModuleHandleW("steam_api64.dll")`, **never** `LoadLibrary`: we do not load a DLL the game would
not have loaded. Null: log "steam_api64 missing", and return nil.

### 2. The exports

Walk the module's export table (PE headers, already read elsewhere by the proxy), and log every name
that contains `SteamFriends`, `SteamUser`, `SteamMatchmaking`, `GetHSteamUser`, `FindOrCreate` or
`RegisterCallback`. The full list settles question 1 at once, even if the attempts that follow fail.

### 3. The interfaces

Three ways to get them, tried in this order, logging the one that answers:

| way | exports | SDK |
|---|---|---|
| versioned flat accessors | `SteamAPI_SteamFriends_v017()`, `SteamAPI_SteamUser_v023()`, `SteamAPI_SteamMatchmaking_v009()` | 1.48 and later |
| interface by name | `SteamAPI_GetHSteamUser()`, then `SteamInternal_FindOrCreateUserInterface(h, "SteamFriends017")`; versions 017 to 015, `SteamUser023` to 019, `SteamMatchmaking009` to 008 | 1.37 and later |
| old accessors | `SteamFriends()`, `SteamUser()`, `SteamMatchmaking()` | before 1.37 |

The version numbers are those of the public SDK. The export list (step 2) gives the right ones if
none answers.

### 4. The flat calls

| call | signature (x64, `uint64` for a `CSteamID`) | what we log |
|---|---|---|
| `SteamAPI_ISteamUser_GetSteamID` | `(user) -> uint64` | our Steam ID |
| `SteamAPI_ISteamFriends_GetFriendGamePlayed` | `(friends, uint64 id, FriendGameInfo_t*) -> bool` | the return value, the 24 bytes in hex, then each field |
| `SteamAPI_ISteamMatchmaking_GetLobbyOwner` | `(mm, uint64 lobby) -> uint64` | the owner of the lobby read |
| `SteamAPI_ISteamMatchmaking_GetNumLobbyMembers` | `(mm, uint64 lobby) -> int` | the number of members |

`FriendGameInfo_t`, 24 bytes:

| offset | field | size | check |
|---|---|---|---|
| 0 | `m_gameID` | 8 | the low 24 bits are **289070**: proof that the structure is read correctly |
| 8 | `m_unGameIP` | 4 | |
| 12 | `m_usGamePort` | 2 | |
| 14 | `m_usQueryPort` | 2 | |
| 16 | `m_steamIDLobby` | 8 | non-zero, and different from our Steam ID |

The structure is passed filled with zeros: a call that returns `true` without writing anything shows.

**The check with no second player**: on the host, `GetLobbyOwner(m_steamIDLobby)` must return our
Steam ID, and `GetNumLobbyMembers` at least 1. Together, the two say that the identifier read is a
lobby we are in, and that we are its host. That is almost proof on its own; the friend who joins
(protocol, test 2) completes it.

### 5. On the Lua side

In the same trigger:
- `PlayerConfigurations[Network.GetLocalPlayerID()]:GetNetworkIdentifer()`, compared with the Steam
  ID from step 4: if they differ, we are not reading the right interface;
- the names of the functions in the `Network` table (`for k, v in pairs(Network)`), sorted: the exe
  registers 85 of them, and we may have missed an accessor to the lobby.

### 6. The link

`steam://joinlobby/289070/<m_steamIDLobby>/<our Steam ID>`, logged in full.

## Safety

- `panic = "abort"`: no `unwrap`, no indexing that could panic. Every pointer returned by Steam is
  tested before use.
- An interface is never passed to a flat function while its pointer is null.
- No writes, no callback registered, no hook in the exe: the probe only reads.
- The log only contains identifiers that the lobby link already contains: our Steam ID and the
  lobby. No friends list, no account name.

## Expected log

In `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`, with the `sonde steam :` prefix:

```
sonde steam : t=0s host, exe CivilizationVI_DX12.exe
sonde steam : exports SteamAPI_GetHSteamUser, SteamInternal_FindOrCreateUserInterface, SteamAPI_ISteamFriends_GetFriendGamePlayed, …
sonde steam : interfaces through FindOrCreate: SteamFriends017, SteamUser021, SteamMatchmaking009
sonde steam : GetSteamID 76561198000000001 ; GetNetworkIdentifer 76561198000000001 (identical)
sonde steam : GetFriendGamePlayed(self) true, bytes 2e6904…, appid 289070, lobby 109775241000000000
sonde steam : GetLobbyOwner 76561198000000001 (us), 1 member
sonde steam : link steam://joinlobby/289070/109775241000000000/76561198000000001
sonde steam : Network = BroadcastGameConfig, BroadcastPlayerInfo, …
```

## Test protocol

| test | who | what it validates |
|---|---|---|
| 1 | the host alone: `sonde_steam = true`, host an Internet lobby from the menu, wait 30 s | questions 1, 2, 3 and half of 4 (`GetLobbyOwner`); question 5 |
| 2 | a friend, without BGE: click on the link logged in test 1 | the second half of 4: they land in **this** lobby |
| 3 | the friend, with BGE and the probe: join normally | does the client read the same lobby? |
| 4 | test 1 with the other exe (DX11 and DX12) | same Steam DLL, same results |
| 5 | test 1 in a private lobby, then friends-only | does the game presence, and so `GetFriendGamePlayed`, depend on visibility? |
| 6 | test 1 then host migration (the host leaves) | does the new host read the same lobby? |

Get the log through the bug report button in the Escape menu, or directly from the folder above.

## What to do with the results

| result | next step |
|---|---|
| non-zero lobby, appid 289070, `GetLobbyOwner` = us, the friend lands in it | **way 1 validated**: `steam.rs` becomes the link feature, without the logging. Note the delay observed before the first valid reading |
| `true` but zero lobby, or valid only after a variable delay | keep way 1 with re-reads, capped (30 s), and prepare way 2 as a fallback |
| `false`, or a lobby that is not ours | **way 2**: subscribe to `LobbyCreated_t` / `LobbyEnter_t` through `SteamAPI_RegisterCallback`. A second probe, on the same model |
| no interface obtained | read the export list, fix the names or versions, try again |
| link valid according to the probe, but the friend does not land in it | the `steam://joinlobby` format is at fault, not the reading: compare with the link Steam itself gives (right-click on the host in the friends list, "Join game") |

Way 3 (the Web API's `lobbysteamid`) is tested separately, without the game: a `curl` on
`GetPlayerSummaries` with a Web API key, during test 1. It would only serve the site, as a fallback.
