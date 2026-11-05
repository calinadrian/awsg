# Local Forever Skyborne races

High Order Skyborne (95, Alliance) supports Warrior, Hunter, Rogue, Mage and Druid.
Windshaper Skyborne (96, Horde) supports Warrior, Hunter, Rogue, Shaman and Druid.
These combinations come from Forever CharBaseInfo, not retail fallback races.
Both body types use the actual Skyborne models, textures and portraits.

Source: the user's read-only local wowf 1.60.1.70707 installation. Character models
are resolved through ChrRaceXChrModel -> ChrModel -> CreatureDisplayInfo ->
CreatureModelData. Model files are 7478487 (male) and 7478494 (female).
The display table has unrelated encrypted BLTE blocks: the importer omits those
blocks while preserving offsets; it never decrypts them. Required Skyborne rows
are in readable sections. Customization collections include Skyborne feathers.

The importer uses D:/Projects/ASR/launcher for existing CASC, character conversion,
BLP and MPQ tooling. Run from the game root:

```powershell
python scripts/import_forever_skyborne.py --source 'D:/Personal/Battle.net/World of Warcraft' --tools D:/Projects/ASR/launcher --listfile D:/Projects/ASR/downloads/wow-listfile/community-listfile.csv --data wow-data/Data --out out/forever-skyborne/patch-Y.MPQ
```

It stages a merged patch-Y archive and report, preserving unrelated existing
entries. Install the staged archive only if the current overlay SHA256 still
matches `base_overlay_sha256` in the report. Retain the old overlay for rollback.
The importer refuses duplicate race/display identities; start from the retained
pre-import overlay to rebuild. Generated assets and reports remain gitignored.

Available appearance dials are the existing skin, face, hair style, hair color
and facial hair selectors. Other Skyborne options use their default selection.
Class abilities and progression use the existing Survivors systems; no Forever
racial abilities or special druid forms are implemented by this import.
The original ASR race importer rebuilds picker data, so preserve these two rows
if regenerating its roster. No assets are committed or published.
