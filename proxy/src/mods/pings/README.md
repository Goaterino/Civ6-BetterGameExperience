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
Digits 1–9 are available as Alt shortcuts.

- Set `icon` to text or a Civ font icon such as `[ICON_Movement]`. Use `texture`
  for a DDS available to Civ’s texture manager. The map and wheel use the same entry.
- Drop a signed 16-bit PCM WAV in [audio/](audio/) and set `sound` to its filename
  without the extension. Cargo automatically embeds it in the GameCore DLL. The cue
  honors the ping, master, effects and mute-on-focus-loss settings.
- An active copy of the standalone v9 mod supplies its local DDS and Wwise events
  for entries with a `skin` name. BGE handles its input and delivery. The public
  build ships its own sounds and uses Civ’s installed UI art by default.
- Put new label keys in each [translation XML](../../../../textes/README.md) and run
  `python textes/gen_textes.py --strict` from the repository root.

Keep existing IDs stable when changing presentation. Coordinate changes to the wire
format with the repository’s [lobby version](../../../../release/README.md).
Run `lua proxy/tests/test_pings.lua` for the multi-seat input, chat and routing fixture.
