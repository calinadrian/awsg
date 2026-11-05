# WoWForever Durotar terrain

The Survivors map picker includes Durotar after Elwynn Forest and Hall of Thanes. Its nine
playable-area tiles are converted from the active `wowf` CASC product into a local vanilla MPQ
overlay. Retail textures, terrain layers, props, buildings, water, ground cover, and minimap tiles
are remapped to private names so they do not replace shared assets. The map's existing creature
roster and encounter tuning remain in the game data.

## Import

From the repository root, with the WoWForever install and ASR asset tools present:

```powershell
python scripts/import_forever_durotar.py `
  --source 'D:/Personal/Battle.net/World of Warcraft/Data' `
  --tools D:/Projects/ASR/launcher `
  --listfile D:/Projects/ASR/downloads/wow-listfile/community-listfile.csv
Copy-Item out/forever-durotar/patch-X.MPQ wow-data/Data/patch-X.MPQ
```

The CASC install is read-only. The importer writes `out/forever-durotar/patch-X.MPQ` and an
adjacent manifest. It converts Kalimdor tiles X 39–41, Y 29–31, validates MPQ read-back, preserves
the installed Elwynn minimap and ground-cover records, and writes terrain assets under private
`ASRForeverDurotar` paths. Local game data and generated archives are not committed or
redistributed.

The existing renderer handles lighting and shaders. Retail terrain vertex tint, multiple
overlapping liquid surfaces within one chunk, and M2 particles/ribbons are not represented; when
liquid layers overlap, the converter keeps the uppermost surface.
