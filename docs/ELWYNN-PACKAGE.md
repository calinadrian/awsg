# Elwynn player package

The collaboration build currently offers **Elwynn Forest only**, retaining all
current races and classes. The other arena definitions remain in the source for
future packs, but are not offered by the menu.

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

The pack keeps nine detailed terrain tiles around the arena, nearby coarse
horizon heights, referenced scenery, current character appearances and
equipment, class spell effects, audio, and shared UI. Other detailed zones,
dungeons, cinematics, and the full world-map UI are excluded. Shared spell/UI
libraries remain conservative; this is not a byte-minimal pack.

`asset-manifest.json` records included files, hashes, and references already
missing from the original data. In the current data set, these include the
Lightwell display and some older effect/audio references. Building and packaging
do not verify gameplay; manual gameplay testing remains necessary.

Generated output stays ignored under `out/`. The shared player ZIP is stored
separately at `playable/elwynn-playable.zip` using Git LFS. See
[playable/README.md](../playable/README.md) to clone, play, or run the source
against that data. Full-client archives and build caches remain ignored.
