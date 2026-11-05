# Hall of Thanes arena

The Survivors map picker offers Elwynn Forest (saved index 0) and Hall of Thanes
(index 1). Hall of Thanes uses the player's locally installed Forever environment:
source map 3065, WMO FileDataID 7732775, all 20 building groups and 31 distinct prop
models. The arena is a 32-yard circle in the first burial hall, not a traversal of
the entire dungeon. The complete building remains available to the renderer.

This is a Survivors adaptation. Its four named bosses and themed enemy ladder use
existing classic creature displays and Survivors rules. It does not recreate the
Forever dungeon's scripted encounters, quests, loot tables or multiplayer server.
Lighting and materials use ASR's renderer; M2 particles and ribbons are not imported.

## Import

From the repository root:

```powershell
python scripts/import_forever_thanes.py `
  --source 'D:/Personal/Battle.net/World of Warcraft' `
  --tools D:/Projects/ASR/launcher `
  --listfile D:/Projects/ASR/downloads/wow-listfile/community-listfile.csv
Copy-Item out/forever-thanes/patch-R.MPQ wow-data/Data/patch-R.MPQ
```

The importer selects active product `wowf` from `.build.info`, resolves locally
available HD/SD variants, converts the WMO and props using the existing retail
converter, translates MPY2 triangle collision records and wide MOBA material IDs
to the vanilla layouts, and writes a private global-WMO WDT and an additional vanilla Map.dbc
row. It derives a connected floor grid from the building geometry; safe spawn
cells exclude walls and prop bounds. The generated floor remains inside the MPQ.
The source install is read-only; assets and manifests are ignored local outputs.
No extracted assets belong in Git or in redistributed builds.

The importer requires the current source FileDataIDs and compatible retail model
formats. A future Forever build may require updating them. Missing or encrypted
dependencies fail the import. The adjacent JSON records build, source hashes,
conversion reports, floor statistics, limitations and archive hash.

## Play

Close any running client, run `play.cmd`, select **Hall of Thanes** under
**Your Adventure**, and start a run. Rebuild/restart the local co-op relay to accept
arena index 1; all participating clients need the same imported pack and new client.
The map stays disabled when its floor pack is absent. To remove it, move
`wow-data/Data/patch-R.MPQ` outside `Data` with the game closed.

Gameplay and visual validation are left to the user. Import-time archive assembly
checks are not a gameplay test.

## Liquid conversion

Forever's MOGP liquid field contains a LiquidType ID, with 0 marking a dry group.
The vanilla engine interprets 0 as an unbounded fully flooded room. The importer
sets the vanilla no-override sentinel (15) on every group. Actual MLIQ pools retain
their footprint and heights and receive the class from the source LiquidType
SoundBank field, so their volume is bounded by the pool rather than the room.
The manifest records the source type and translated liquid class for each group.
