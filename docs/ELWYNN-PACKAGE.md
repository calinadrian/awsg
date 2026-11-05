# Player data package

The player package carries **every arena** defined in the survivors' `ALL_MAPS`
(`crates/benilla-app/src/survivors/data.rs`), playable or still locked, with all
current races and classes. The script and output keep their historical
`elwynn` names so existing build paths keep working.

From the repository root, with Rust, Windows C++ Build Tools and Python 3.10+:

```powershell
python3.12 scripts/package_elwynn.py --zip
```

Substitute your Python interpreter's command if needed. Supply your own game
data under `wow-data/Data/`, or pass `--data D:/Games/WoW/Data`.

The script builds the asset reader and player executable, reads the existing MPQ
patch chain, and produces `out/elwynn-playable/` and `out/elwynn-playable.zip`.
Extract the ZIP and run `Play.cmd`; no Rust installation is needed to play.
The original data folder is kept intact. Controls are in `CONTROLS.md`.

For each arena the pack keeps the 3×3 detailed terrain tiles around its centre
(read from `Map.dbc` and the arena's `center`), coarse horizon heights five
tiles further out, and the minimap tiles for the packed terrain. A dungeon map
with no terrain tiles ships its tile index whole, which carries its WMO. Each
arena's foes and bosses, referenced scenery, character appearances and
equipment, class spell effects, area audio and shared UI are included. The rest
of each continent, cinematics and the full world-map UI are excluded. Shared
spell/UI libraries remain conservative; this is not a byte-minimal pack.

**Adding an arena to `ALL_MAPS` needs a rebuilt pack.** Without it the new
zone has no ground (a blue sky-only world on a continent already shipped) or
never finishes loading (a map not shipped at all).

`asset-manifest.json` records included files, hashes, the packed tiles per map,
and references already missing from the original data. Building and packaging
do not verify gameplay; manual gameplay testing remains necessary.

Generated output stays ignored under `out/`. The shared player ZIP is stored
separately at `playable/elwynn-playable.zip` using Git LFS. See
[playable/README.md](../playable/README.md) to clone, play, or run the source
against that data. Full-client archives and build caches remain ignored.
