"""Bake scenery shadows into the local retail Elwynn overlay; source installs stay read-only."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys

from import_retail_elwynn import TILES, chunk, chunks


def add_shadows(adt, masks):
    if len(masks) != 256 * 512:
        raise ValueError('Expected 256 packed 64x64 shadow maps')
    result, index = bytearray(), 0
    for tag, payload in chunks(adt):
        if tag == b'MCIN':
            raise ValueError('Only converted retail ADTs without MCIN are supported')
        if tag == b'MCNK':
            if index >= 256 or len(payload) < 128:
                raise ValueError('Invalid converted terrain cell')
            payload = bytearray(payload)
            mask = masks[index * 512:(index + 1) * 512]
            offset, size = struct.unpack_from('<2I', payload, 44)
            if offset:
                if size != 512 or payload[offset - 8:offset - 4] != b'HSCM':
                    raise ValueError('Invalid existing shadow map')
                payload[offset:offset + 512] = mask
            else:
                struct.pack_into('<2I', payload, 44, len(payload) + 8, 512)
                payload += chunk(b'MCSH', mask)
            flags = struct.unpack_from('<I', payload)[0]
            struct.pack_into('<I', payload, 0, flags | 1)
            index += 1
        result += chunk(tag, payload)
    if index != 256:
        raise ValueError('Expected 256 converted terrain cells')
    return bytes(result)


def bake_archive(path, tools):
    sys.path.insert(0, str(Path(tools).resolve()))
    from photo_archive import build, read_stored
    path = Path(path).resolve()
    root = Path(__file__).resolve().parent.parent
    masks_dir = path.parent / 'shadows'
    subprocess.run(['cargo', 'run', '--release', '--offline', '-p', 'benilla-formats',
                    '--example', 'bake_elwynn_shadows', '--', str(path), str(masks_dir)],
                   cwd=root, check=True)
    before = path.read_bytes()
    files = read_stored(before)
    # photo_archive.build writes sorted names, with (listfile) first.
    names = files[0].decode('ascii').splitlines()
    keys = sorted(['(listfile)', *names])
    if len(keys) != len(files) or keys[0] != '(listfile)':
        raise ValueError('Unexpected stored archive directory')
    entries = dict(zip(keys, files, strict=True))
    entries.pop('(listfile)')
    counts = {}
    for x, y in TILES:
        key = next(n for n in entries if n.lower().replace('\\', '/') ==
                   f'world/maps/azeroth/azeroth_{x}_{y}.adt')
        masks = (masks_dir / f'azeroth_{x}_{y}.mcsh').read_bytes()
        entries[key] = add_shadows(entries[key], masks)
        counts[f'{x},{y}'] = sum(b.bit_count() for b in masks)
    archive = build(entries)
    # Verify all entries, including unchanged models and textures, before replacement.
    if read_stored(archive) != [archive_entry for _, archive_entry in sorted({
            **entries, '(listfile)': ('\r\n'.join(sorted(entries)) + '\r\n').encode('ascii')
    }.items())]:
        raise ValueError('Shadow archive read-back failed')
    backup = masks_dir / 'patch-Q.before-shadows.MPQ'
    if not backup.exists():
        backup.write_bytes(before)
    report_path = path.with_suffix('.json')
    report = json.loads(report_path.read_text()) if report_path.exists() else {}
    report['sha256'] = hashlib.sha256(archive).hexdigest()
    report['scenery_shadows'] = {
        'method': 'geometry rays with alpha-tested foliage into 64x64 MCSH',
        'daylight_minute': 780, 'shadowed_texels': counts,
        'limitations': ['static terrain receivers', 'no shadows beyond the imported tile boundary'],
    }
    temporary = path.with_suffix('.MPQ.tmp')
    temporary.write_bytes(archive)
    temporary.replace(path)
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(f'Baked scenery shadows into {path}: {sum(counts.values()):,} shadowed texels')
    return len(archive)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, default=Path('out/retail-elwynn/patch-Q.MPQ'))
    parser.add_argument('--tools', type=Path, required=True)
    args = parser.parse_args()
    sys.path.insert(0, str(args.tools.resolve()))
    bake_archive(args.archive, args.tools)


if __name__ == '__main__':
    main()
