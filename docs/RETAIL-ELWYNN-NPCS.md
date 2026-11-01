# Retail Elwynn NPC source assets

The separate NPC staging tool extracts visual-upgrade candidates for all 17
entries in `crates/benilla-app/src/survivors/elwynn.rs`, including its guardians
and bosses. It reads the player's retail installation without changing it:

```powershell
python scripts/import_retail_elwynn_npcs.py `
  --source 'D:/Personal/Battle.net/World of Warcraft' `
  --tools D:/Projects/ASR/launcher `
  --listfile D:/Projects/ASR/downloads/wow-listfile/community-listfile.csv
python -m unittest discover -s scripts -p test_import_retail_elwynn_npcs.py
```

Output is restricted to this checkout's ignored `out/` directory. The default is
`out/retail-elwynn-npcs/manifest.json`, with original retail bytes stored by
FileDataID under `files/`. The manifest identifies their listfile names, sizes,
SHA-256 hashes, source build and proposed roster matches. No source assets or
derived game archives may be committed or redistributed.

The extraction follows model skin, texture, skeleton, bone and animation file
references recursively. It also collects each selected creature family's named
BLP files: replaceable skin textures are not necessarily named by model TXID.
Any missing required dependency fails extraction and leaves no success manifest.

| Roster members | Candidate source models |
|---|---|
| Kobold Vermin, Geomancer, Taskmaster | Kobold2 |
| Young Wolf, Prowler | WolfDraenor |
| Murloc Streamrunner | Murloc2 |
| Riverpaw Brute, Gruff Swiftbite | Gnoll2 |
| Fedfennel | Gnoll2Caster |
| Hogger | Dedicated Hogger model |
| Princess, Bellygrub | Boar2 |
| Mother Fang | Original retail MineSpiderBoss and newer Spider alternative |
| Defias Thug, Enforcer, Morgan the Collector | Human male HD |
| Defias Rogue Wizard | Human female HD |

These are proposed family matches, not verified retail NPC appearances. In
particular, a modern spider alternative changes the original mine-spider design,
and the original ASR Gruff display is a wolf despite its gnoll NPC name. Both
choices need visual review. No skin variant is silently selected.

The tool attempts to extract `CreatureDisplayInfo.db2` and
`CreatureModelData.db2`, recording missing or encrypted tables explicitly.
On an install with encrypted display data, exact retail display and skin matches
remain unresolved. Table field definitions are maintained in
[WoWDBDefs](https://github.com/wowdev/WoWDBDefs/tree/master/definitions).

The bundle is input to conversion and fitting. Raw retail models cannot be loaded
as 1.12 M2s. Human NPCs additionally need customization and clothing composition;
their HD meshes alone do not reproduce a dressed Defias NPC. Conversion must retain
the needed animation sequences and select geosets and skin variants before a
separate local patch can be validated. This tool does not install a patch or
change the runtime roster.
