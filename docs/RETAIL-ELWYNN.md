# Retail Elwynn assets

`scripts/import_retail_elwynn.py` converts Elwynn's playable arena and the surrounding
nine terrain tiles from the player's retail CASC install to a local `patch-Q.MPQ`.
The existing arena position and 120-yard boundary stay the same. The overlay contains
retail heightfields, terrain normals, ground textures and blend weights, water,
placed scenery, buildings, building props and outdoor minimap tiles.
The import also bakes tree and building shadows into vanilla MCSH ground maps,
using the converted scenery geometry and leaf texture alpha at the game's fixed
daylight time. Retail terrain has no MCSH maps of its own. These shadows are static,
fall on terrain, and do not add a per-frame shadow-rendering pass.
Retail ground-effect definitions and their grass and flower models are translated
into private DBC IDs. Original ground-effect records remain available to other
zones. The terrain's predominant-layer and no-clutter masks are preserved, so
ground cover follows authored ground layers and clearings.

The import covers Azeroth tiles X 32–34, Y 48–50. Everything outside that square
uses the original data. The scenery and its textures use private asset paths under
`World/ASRRetailElwynn`, so shared vanilla asset names are not replaced. Only the
nine ADT paths and their minimap index entries override existing assets.

## Build and validate

Run from the repository root, with Python and the local ASR asset tooling:

```powershell
python scripts/import_retail_elwynn.py `
  --source 'D:/Personal/Battle.net/World of Warcraft' `
  --tools D:/Projects/ASR/launcher `
  --listfile D:/Projects/ASR/downloads/wow-listfile/community-listfile.csv
python -m unittest discover -s scripts -p test_import_retail_elwynn.py
cargo run -p benilla-formats --example verify_retail_elwynn -- out/retail-elwynn/patch-Q.MPQ
```

The source install is read-only. All output is staged under the ignored `out/`
directory. The adjacent JSON manifest records the retail build, source FileDataIDs
and hashes, converted model counts and the archive hash. Missing files and unknown
layouts fail the import before a new archive replaces the previous output.

To add shadows to an existing converted overlay without extracting retail again:

```powershell
python scripts/bake_elwynn_shadows.py --tools D:/Projects/ASR/launcher
```

The baker keeps the previous archive under `out/retail-elwynn/shadows/` and
records shadow coverage and the new archive hash in the adjacent manifest.

The liquid layouts currently supported are the retail DB2 layout hashes
`LiquidType D1ECEEC9` and `LiquidMaterial 98E5D7AA`. Recheck the schemas when
the retail install changes. The field layouts are documented in
[WoWDBDefs](https://github.com/wowdev/WoWDBDefs/tree/master/definitions), and
the MH2O structures in [TrinityCore's ADT reader](https://github.com/TrinityCore/TrinityCore/blob/master/src/tools/map_extractor/adt.h).

## Use locally

With the game closed, copy the validated archive into this checkout's data folder:

```powershell
Copy-Item out/retail-elwynn/patch-Q.MPQ wow-data/Data/patch-Q.MPQ
cargo run -p benilla-formats --example verify_retail_elwynn -- out/retail-elwynn/patch-Q.MPQ wow-data/Data
./play.cmd
```

The second validation checks the installed patch chain, including terrain and
minimap priority. A running client must restart to open the new archive.
To revert, move `wow-data/Data/patch-Q.MPQ` outside `Data` while the game is closed.
The original archives are retained. Assets and derived archives remain ignored
and must not be committed or redistributed.

For a scenery-only inspection without combat:

```powershell
$env:WOW_DATA = "$PWD/wow-data/Data"
$env:WOW_UNATTENDED = '1'
$env:WOW_NOSOUND = '1'
$env:WOW_WORLDVIEW_AT = '-9396,-633,68'
$env:WOW_WORLDVIEW_SHOT = "$PWD/out/retail-elwynn/retail-elwynn.png"
$env:WOW_WORLDVIEW_SHOT_AT = '30'
cargo run --release -p benilla-world --example retail_elwynn_viewer
```

## Rendering limits

This imports retail assets into the existing renderer. Lighting, fog, materials
and water shading still use the client's engine. Retail normal/specular material
passes, terrain vertex tint, and M2 particle/ribbon
effects are not converted. Terrain holes are reduced conservatively from the
retail 8x8 mask to the engine's 4x4 mask. Retail additive ground weights are
converted to sequential blend weights without reducing them to 4-bit alpha.
Exterior WMO vertex colors use retail additive-light semantics; the converter
leaves these surfaces to the engine's outdoor lighting instead of treating zero
additive light as a black diffuse multiplier. Interior baked colors are retained.
