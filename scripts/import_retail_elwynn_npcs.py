"""Stage retail visual-upgrade candidates for the ASR Elwynn roster.

Reads the player's CASC install only. Raw models, skin variants and referenced
animation/skeleton files stay under ignored out/, with hashes and explicit
appearance limitations. Candidate family matches are not retail NPC display IDs.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys


# Current survivors/elwynn.rs roster, including guardians and named bosses.
# Multiple candidates retain the original species as well as a newer alternative.
ROSTER = [
    ('Kobold Vermin', 26, (1139464,)),
    ('Young Wolf', 165, (926251,)),
    ('Murloc Streamrunner', 31, (1113034,)),
    ('Kobold Geomancer', 163, (1139464,)),
    ('Defias Thug', 2323, (1011653,)),
    ('Riverpaw Brute', 175, (3886641,)),
    ('Prowler', 644, (926251,)),
    ('Defias Rogue Wizard', 2315, (1000764,)),
    ('Defias Enforcer', 2324, (1011653,)),
    ('Mother Fang', 7536, (124904, 3095173)),
    ('Fedfennel', 204, (4061820,)),
    ('Princess', 703, (3978441,)),
    ('Morgan the Collector', 2322, (1011653,)),
    ('Kobold Taskmaster', 163, (1139464,)),
    ('Gruff Swiftbite', 246, (3886641,)),
    ('Bellygrub', 377, (3978441,)),
    ('Hogger', 384, (4665405,)),
]


def references(data):
    """FileDataID references in modern M2/skeleton chunks, with strict bounds."""
    if data[:4] not in (b'MD21', b'SKL1'):
        return set()
    result, pos = set(), 0
    while pos < len(data):
        if pos + 8 > len(data):
            raise ValueError('Truncated model chunk header')
        tag, size = struct.unpack_from('<4sI', data, pos)
        pos += 8
        body = data[pos:pos + size]
        if len(body) != size:
            raise ValueError(f'Truncated {tag!r} chunk')
        pos += size
        if tag in (b'SFID', b'SKID', b'TXID', b'BFID', b'PFID'):
            if size % 4:
                raise ValueError(f'Invalid {tag!r} reference array')
            result.update(v[0] for v in struct.iter_unpack('<I', body))
        elif tag == b'AFID':
            if size % 8:
                raise ValueError('Invalid animation reference array')
            result.update(v[2] for v in struct.iter_unpack('<HHI', body))
        elif tag == b'SKPD':
            if size != 16:
                raise ValueError('Unknown skeleton parent layout')
            result.add(struct.unpack_from('<I', body, 8)[0])
    return result - {0}


def extract(storage, names, destination):
    models = sorted({fd for _, _, ids in ROSTER for fd in ids})
    # Replaceable creature skins are selected by display tables rather than TXID.
    # Stage each creature family's named BLPs so later fitting can choose its skin.
    folders = {names[fd].rsplit('/', 1)[0] for fd in models
               if names[fd].startswith('creature/')}
    pending = set(models) | {fd for fd, name in names.items()
                            if name.endswith('.blp') and name.rsplit('/', 1)[0] in folders}
    files = {}
    while pending:
        fd = min(pending)
        pending.remove(fd)
        if fd in files:
            continue
        data = storage.read(fd)
        if data is None:
            raise FileNotFoundError(f'Retail dependency {fd} is not installed')
        # Numeric filenames also safely handle unnamed files and untrusted listfiles.
        path = destination / 'files' / str(fd)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        files[fd] = dict(name=names.get(fd), path=f'files/{fd}', bytes=len(data),
                         sha256=hashlib.sha256(data).hexdigest())
        pending.update(references(data) - files.keys())
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--tools', type=Path, required=True)
    parser.add_argument('--listfile', type=Path, required=True)
    parser.add_argument('--out', type=Path, default=Path('out/retail-elwynn-npcs'))
    args = parser.parse_args()
    source, output = args.source.resolve(), args.out.resolve()
    allowed = Path(__file__).resolve().parents[1] / 'out'
    if not output.is_relative_to(allowed) or output.is_relative_to(source):
        parser.error('--out must be under this checkout\'s ignored out/ directory and outside retail')
    sys.path.insert(0, str(args.tools.resolve()))
    from casc import Casc
    names = {}
    for line in args.listfile.read_text(encoding='utf-8').splitlines():
        ident, name = line.split(';', 1)
        names[int(ident)] = name.lower().replace('\\', '/')
    output.mkdir(parents=True, exist_ok=True)
    manifest = output / 'manifest.json'
    # Never leave a previous success manifest covering an interrupted new extraction.
    manifest.unlink(missing_ok=True)
    storage = Casc(source)
    try:
        files = extract(storage, names, output)
        tables = {}
        for name, fd in [('CreatureDisplayInfo', 1108759), ('CreatureModelData', 1365368)]:
            try:
                data = storage.read(fd)
            except PermissionError:
                tables[name] = dict(file_data_id=fd, status='encrypted in local install')
                continue
            if data is None:
                tables[name] = dict(file_data_id=fd, status='not installed')
                continue
            (output / f'{name}.db2').write_bytes(data)
            tables[name] = dict(file_data_id=fd, status='extracted', sha256=hashlib.sha256(data).hexdigest())
        report = dict(source=str(source), version=storage.version, status='raw candidates extracted',
                      roster=[dict(name=name, vanilla_display=display, candidate_models=list(ids))
                              for name, display, ids in ROSTER],
                      files={str(fd): files[fd] for fd in sorted(files)}, tables=tables,
                      limitations=[
                          'Family candidates; exact retail NPC display/skin matches are unverified.',
                          'Human HD models need NPC customization, clothing and texture composition.',
                          'Raw retail files need conversion and visual/animation validation before installation.',
                          'No game data, runtime roster, or installed patch archive was changed.',
                      ])
        temporary = manifest.with_suffix('.json.tmp')
        temporary.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
        temporary.replace(manifest)
        print(f'Extracted {len(files)} files, {sum(f["bytes"] for f in files.values()):,} bytes; {len(ROSTER)} roster entries')
        print(manifest)
    finally:
        storage.close()


if __name__ == '__main__':
    main()
