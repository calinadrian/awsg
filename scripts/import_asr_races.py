"""Import locally converted ASR races; never modify the source installation.

Usage: python scripts/import_asr_races.py D:/Projects/ASR
Uses that project's read-only MPQ tooling and reviewed race-expansion archives.
Blizzard assets are written only under gitignored wow-data/Data.
"""
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys


def main():
    source = Path(sys.argv[1]).resolve()
    root = Path(__file__).resolve().parents[1]
    sys.path.insert(0, str(source / 'launcher'))
    from bloodelf_assets import Dbc, dbc_name, RACE_ATLAS
    from durotar_assets import Chain
    from photo_archive import build, read_stored
    from race_catalog import PORTRAITS
    from repair_goblin_torso import repair_model

    candidate = source / 'downloads/race-expansion'
    dest = root / 'wow-data/Data'
    metadata = json.loads((source / 'launcher/native-races.json').read_text())
    classes_source = (root / 'crates/benilla-app/src/survivors/data.rs').read_text()
    classes = {}
    for block in classes_source.split('ClassDef {')[2:]:
        cid = int(re.search(r'id: (\d+)', block)[1])
        looks = re.search(r'looks: &\[(.*?)\]', block, re.S)[1]
        classes[cid] = [int(x) for x in re.findall(r'\d+', looks)]

    # Refuse to replace unrelated local archives. Repeated imports are idempotent.
    for name in ('patch-V.MPQ', 'patch-W.MPQ'):
        src, dst = candidate / name, dest / name
        digest = hashlib.sha256(src.read_bytes()).hexdigest()
        if dst.exists() and hashlib.sha256(dst.read_bytes()).hexdigest() != digest:
            raise ValueError(f'Refusing to replace different local archive: {dst}')
        if not dst.exists():
            shutil.copyfile(src, dst)

    chain = Chain(dest, exclude=('patch-Y.MPQ',))
    try:
        displays = Dbc(chain.read(dbc_name('CreatureDisplayInfo')))
        extras = Dbc(chain.read(dbc_name('CreatureDisplayInfoExtra')))
        races = {r[0]: r for r in Dbc(chain.read(dbc_name('ChrRaces'))).rows()}
        display_rows = {r[0]: r for r in displays.rows()}
        extra_rows = {r[0]: r for r in extras.rows()}
        roster = []
        for rid, name, faction in (
            (1, 'Human', 'alliance'), (2, 'Orc', 'horde'),
            (3, 'Dwarf', 'alliance'), (4, 'Night Elf', 'alliance'),
            (5, 'Undead', 'horde'), (6, 'Tauren', 'horde'),
            (7, 'Gnome', 'alliance'), (8, 'Troll', 'horde'),
            (10, 'Blood Elf', 'horde'),
            *((int(k), v['name'], v['faction']) for k, v in metadata.items()),
        ):
            allowed = metadata[str(rid)]['classes'] if str(rid) in metadata else None
            if rid == 10:
                allowed = [2, 3, 4, 5, 8, 9]
            if allowed is None:
                allowed = [cid for cid, looks in classes.items() if any(
                    extra_rows.get(display_rows[d][3], [0, 0])[1] == rid
                    for d in looks if d in display_rows)]
            roster.append(dict(id=rid, name=name, faction=faction,
                               classes=allowed, portrait=PORTRAITS[rid]))
            for cid in allowed:
                for sex in (0, 1):
                    did = 300000 + cid * 200 + rid * 2 + sex
                    if did in display_rows or did in extra_rows:
                        raise ValueError(f'Display collision: {did}')
                    # Preserve class clothing, but composite the new race's own skin.
                    template = next(extra_rows[display_rows[d][3]] for d in classes[cid]
                                    if d in display_rows and display_rows[d][3] in extra_rows)
                    extra = list(template)
                    extra[:8] = [did, rid, sex, 0, 0, 0, 0, 0]
                    extra[18] = 0  # no baked texture from another race
                    if rid == 52:  # dragon bodies do not wear humanoid armor
                        extra[8:18] = [0] * 10
                    row = list(display_rows[races[rid][4 + sex]])
                    row[0], row[3] = did, did
                    displays.add(row)
                    extras.add(extra)
        entries = {dbc_name('CreatureDisplayInfo'): displays.bytes(),
                   dbc_name('CreatureDisplayInfoExtra'): extras.bytes()}
        output = dest / 'patch-Y.MPQ'
        if output.exists():
            files = read_stored(output.read_bytes())
            existing = dict(zip(files[0].decode('ascii').splitlines(), files[1:], strict=True))
            for name, data in entries.items():
                if name in existing and existing[name] != data:
                    raise ValueError(f'Refusing to replace different local entry: {name}')
            existing.update(entries)
            entries = existing
        for sex in ('Male', 'Female'):
            path = f'Character\\Goblin\\{sex}\\Goblin{sex}.m2'
            # Preserve animation repairs already carried by the local overlay.
            key = next((name for name in entries if name.lower() == path.lower()), path)
            raw = entries[key] if key in entries else chain.read(path)
            repaired, changed = repair_model(raw)
            if changed:
                entries[key] = repaired
        output.write_bytes(build(entries))
        (dest.parent / 'asr-races.json').write_text(json.dumps(roster, indent=2))
        # Code metadata contains only names, IDs, class policy and atlas coordinates.
        lines = ['pub(super) const RACES: &[Race] = &[']
        for r in roster:
            faction = {'alliance': 0, 'horde': 1, 'neutral': 2}[r['faction']]
            lines.append(f'Race {{ id: {r["id"]}, name: {json.dumps(r["name"])}, faction: {faction}, classes: &{r["classes"]}, portrait: ({r["portrait"][0]}, {r["portrait"][1]}) }},')
        lines.append('];')
        (root / 'crates/benilla-app/src/survivors/races_data.rs').write_text('\n'.join(lines) + '\n')
        print(f'Imported {len(roster)} races, 52 body models and class appearances into {dest}')
    finally:
        chain.close()


if __name__ == '__main__':
    main()
