"""Build consistent race portraits from the local retail install, preserving custom heads.

python scripts/repair_race_portraits.py D:/Projects/ASR "D:/Personal/Battle.net/World of Warcraft"
"""
import json
from pathlib import Path
import sys


def main():
    source = Path(sys.argv[1]).resolve()
    root = Path(__file__).resolve().parents[1]
    sys.path.insert(0, str(source/'launcher'))
    import blp
    import casc
    from bloodelf_assets import RACE_ATLAS
    from character_upgrade import RACES, listfile
    from durotar_assets import Chain
    from photo_archive import build, read_stored
    from race_catalog import PORTRAITS
    metadata = json.loads((source/'launcher/native-races.json').read_text())
    names = listfile(source/'downloads/wow-listfile/community-listfile.csv')
    storage = casc.Casc(sys.argv[2])
    game = Chain(root/'wow-data/Data')
    try:
        atlas = (1024, 256, bytearray([18, 21, 27, 255] * 1024 * 256))
        for race, (x, y) in PORTRAITS.items():
            folder = metadata[str(race)]['folder'] if str(race) in metadata else RACES[race][0]
            for sex, label in enumerate(('male', 'female')):
                path = f'interface/characterframe/temporaryportrait-{label}-{folder}'.lower()+'.blp'
                if race in (84, 86):
                    portrait = blp.decode((source/'launcher/assets/race-portraits'/f'{race}-{label}.blp').read_bytes())
                elif path in names:
                    portrait = blp.decode(storage.read(names[path]))
                else:
                    raise ValueError(f'Missing race portrait: {path}')
                blp.paste(atlas, blp.resize(portrait, 64, 64), x, y+128*sex)
        overlay = root/'wow-data/Data/patch-Y.MPQ'
        files = read_stored(overlay.read_bytes())
        entries = dict(zip(files[0].decode('ascii').splitlines(), files[1:], strict=True))
        entries[RACE_ATLAS] = blp.encode(atlas)
        game.close()
        temporary = overlay.with_suffix('.MPQ.tmp')
        temporary.write_bytes(build(entries))
        temporary.replace(overlay)
        print('Rebuilt all 52 race portraits with matching backgrounds and source artwork.')
    finally:
        game.close()


if __name__ == '__main__':
    main()
