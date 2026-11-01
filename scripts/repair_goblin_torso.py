"""Remove the retail goblin torso hide track from local converted models.

Usage: python scripts/repair_goblin_torso.py D:/Projects/ASR
Only edits the local patch-Y overlay; the reference installation stays read-only.
"""
import hashlib
from pathlib import Path
import struct
import sys


def repair_model(raw):
    """The selected base torso is geoset 0 with retail color controller 1.

    Retail's character customization overrides this controller. A legacy model
    has no such override, so its zero alpha hides the torso even in idle/run.
    Leave the other controller (the deliberately hidden auxiliary mesh) intact.
    """
    if raw[:4] != b'MD20' or struct.unpack_from('<I', raw, 4)[0] != 256:
        raise ValueError('Expected the converted v256 goblin model')
    data = bytearray(raw)
    count, views = struct.unpack_from('<II', data, 0x4c)
    changed = 0
    for view in range(count):
        sections, section_at = struct.unpack_from('<II', data, views + view * 44 + 24)
        batches, batch_at = struct.unpack_from('<II', data, views + view * 44 + 32)
        for i in range(batches):
            at = batch_at + i * 24
            section, _, color = struct.unpack_from('<3H', data, at + 4)
            if section >= sections:
                raise ValueError('Goblin batch refers to an absent mesh section')
            geoset = struct.unpack_from('<H', data, section_at + section * 32)[0]
            if geoset == 0 and color == 1:
                struct.pack_into('<H', data, at + 8, 0xffff)
                changed += 1
    return bytes(data), changed


def main():
    source = Path(sys.argv[1]).resolve()
    root = Path(__file__).resolve().parents[1]
    sys.path.insert(0, str(source / 'launcher'))
    from durotar_assets import Chain
    from photo_archive import build, read_stored

    overlay = root / 'wow-data/Data/patch-Y.MPQ'
    original = overlay.read_bytes()
    files = read_stored(original)
    entries = dict(zip(files[0].decode('ascii').splitlines(), files[1:], strict=True))
    chain = Chain(overlay.parent)
    total = 0
    try:
        for sex in ('Male', 'Female'):
            path = f'Character\\Goblin\\{sex}\\Goblin{sex}.m2'
            repaired, changed = repair_model(chain.read(path))
            if changed:
                entries[path] = repaired
                total += changed
            print(f'Goblin {sex}: corrected {changed} torso material batch(es)')
    finally:
        chain.close()
    if total:
        digest = hashlib.sha256(original).hexdigest()[:12]
        backup = overlay.with_name(f'patch-Y.before-goblin-torso-{digest}.bak')
        if not backup.exists():
            backup.write_bytes(original)
        temporary = overlay.with_suffix('.MPQ.tmp')
        temporary.write_bytes(build(entries))
        temporary.replace(overlay)
        print(f'Updated {overlay}; backup: {backup.name}')


if __name__ == '__main__':
    main()
