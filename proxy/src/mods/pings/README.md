# Team pings

The startup proxy registers **Team pings (BGE)** in the host's network game options.
The GameCore proxy checks the existing POL verdict when it loads the world and chat UI.
`GenVFX` hosts the wheel and four-second markers beneath the HUD. Each accepted call
sends a private chat message to each eligible teammate and observer. Reconnecting seats
must have a token in the host’s current verdict. Rate limits use elapsed seconds on
both sender and receiver. Pings do not write game properties or enter a save.

## Editing a call

Edit [catalog.lua](catalog.lua). Each entry supplies a stable wire ID, a translated
label, a color, an icon, a sound, and optionally a wheel angle and direct shortcut.
The call without an angle is the center tap. Other angles are clockwise from the top.
Digits 1–9 define default Alt shortcuts. Players can rebind the wheel and direct calls in Options → Key Bindings. Keep the action ID stable to preserve saved bindings.
Saved native actions remain visible under **Team pings (BGE)** when their original
mod is disabled. **Team ping volume** appears below sound effects in the shared
Audio options, both at the main menu and in a match.

- Set `icon` to text or a Civ font icon such as `[ICON_Movement]`. Use `texture`
  for a DDS available to Civ’s texture manager. The map and wheel use the same entry.
- Drop a signed 16-bit PCM WAV in [audio/](audio/) and set `sound` to its filename
  without the extension. Cargo automatically embeds it in the GameCore DLL. The cue
  honors the ping, master, effects and mute-on-focus-loss settings.
- BGE installs the approved wheel, selection ring, ground ripples, blocked
  indicator, call icons and Wwise cues from [ping_assets](../../../../ping_assets).
  The media component registers automatically when Civ opens the main menu.
  Entries with a `skin` name use the matching DDS and audio event in that package.
  The signed release manifest covers every asset as well as both DLLs.
- Put new label keys in each [translation XML](../../../../textes/README.md) and run
  `python textes/gen_textes.py --strict` from the repository root.

Keep existing IDs stable when changing presentation. Coordinate changes to the wire
format with the repository’s [lobby version](../../../../release/README.md).
Run `lua proxy/tests/test_pings.lua` for the multi-seat input, chat and routing fixture.
