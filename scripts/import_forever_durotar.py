"""Convert the WoWForever CASC Durotar tiles into a private vanilla MPQ overlay."""

import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys

TILES = [(x, y) for y in range(29, 32) for x in range(39, 42)]


class ForeverCasc:
    """Read the active wowf product using the local CASC reader's block decoder."""

    def __init__(self, source, Casc):
        self.root_dir = source
        self.data_dir = source / 'Data' / 'data'
        self.archives = {}
        self._reader = Casc.__new__(Casc)
        self._reader.root_dir = self.root_dir
        self._reader.data_dir = self.data_dir
        self._reader.archives = self.archives
        info = (source / '.build.info').read_text().splitlines()
        fields = [value.split('!')[0] for value in info[0].split('|')]
        rows = [dict(zip(fields, line.split('|'))) for line in info[1:] if line]
        row = next(r for r in rows if r.get('Active') == '1' and r.get('Product') == 'wowf')
        self.version = row['Version']
        self.build = self._reader._config(row['Build Key'])
        self._reader._read_indexes()
        self._reader._read_encoding(bytes.fromhex(self.build['encoding'].split()[1]))
        self.index = self._reader.index
        self.encoding = self._reader.encoding
        self._read_root(self.ekey(bytes.fromhex(self.build['root'].split()[0])))
        self._reader.files = self.files

    def __getattr__(self, name):
        return getattr(self._reader, name)

    def _read_root(self, ekey):
        data = self.read_ekey(ekey)
        header, version = struct.unpack_from('<II', data, 4)
        if data[:4] != b'TSFM' or version not in (1, 2):
            raise ValueError('Unsupported WoWForever root format')
        pos = header
        self.files = {}
        while pos < len(data):
            if version == 2:
                count, locale, f1, f2, f3 = struct.unpack_from('<IIIIB', data, pos)
                content = f1 | f2 | (f3 << 17)
                pos += 17
            else:
                count, content, locale = struct.unpack_from('<III', data, pos)
                pos += 12
            deltas = struct.unpack_from(f'<{count}i', data, pos)
            pos += count * 4
            keys = pos
            pos += count * 16
            if not content & 0x10000000:
                pos += count * 8
            if not (locale & 2 or locale in (0, 0xffffffff, 131062)) or content & 128:
                continue
            ident = -1
            for index, delta in enumerate(deltas):
                ident += delta + 1
                key = data[keys + index * 16:keys + index * 16 + 16]
                old = self.files.get(ident)
                stored = lambda item: item in self.encoding and self.encoding[item][:9] in self.index
                if old is None or (not stored(old) and stored(key)):
                    self.files[ident] = key

    def close(self):
        self._reader.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path,
                        default=Path(r'D:\Personal\Battle.net\World of Warcraft\Data'),
                        help='WoWForever install root or its Data directory')
    parser.add_argument('--tools', type=Path, default=Path(r'D:\Projects\ASR\launcher'))
    parser.add_argument('--listfile', type=Path,
                        default=Path(r'D:\Projects\ASR\downloads\wow-listfile\community-listfile.csv'))
    parser.add_argument('--data', type=Path, default=Path('wow-data/Data'))
    parser.add_argument('--out', type=Path, default=Path('out/forever-durotar/patch-X.MPQ'))
    args = parser.parse_args()
    source = args.source.resolve()
    if source.name.lower() == 'data' and not (source / '.build.info').exists():
        source = source.parent
    if args.out.resolve().is_relative_to(source):
        parser.error('--out must be outside the read-only WoWForever install')
    sys.path.insert(0, str(args.tools.resolve()))
    from casc import Casc
    from photo_archive import build, read_stored
    import import_retail_elwynn as retail

    retail.PREFIX = 'World\\ASRForeverDurotar\\'
    storage = ForeverCasc(source, Casc)
    importer = retail.Importer(source, args.listfile, storage=storage)
    importer.effect_base = 2000000
    importer.detail_prefix = 'ForeverDurotar'
    for x, y in TILES:
        importer.tile(x, y, map_name='kalimdor', label='Durotar')
    importer.minimaps(args.data, map_name='kalimdor', private_name='forever-durotar',
                      tiles=TILES, exclude=('patch-X.MPQ',))
    importer.ground_tables(args.data, exclude=('patch-X.MPQ',))
    archive = build(importer.entries)
    if len(read_stored(archive)) != len(importer.entries) + 1:
        raise ValueError('Archive read-back failed')
    args.out.parent.mkdir(parents=True, exist_ok=True)
    staged = args.out.with_name(args.out.stem + '-staging.MPQ')
    temporary = staged.with_suffix('.MPQ.tmp')
    temporary.write_bytes(archive)
    temporary.replace(staged)
    report = dict(source=str(source), product='wowf', version=storage.version, tiles=TILES,
                  entries=len(importer.entries), models=len(importer.models), wmos=len(importer.wmos),
                  ground_effects=len(importer.effects), ground_models=len(importer.detail_models),
                  sha256=hashlib.sha256(archive).hexdigest(),
                  source_sha256=importer.provenance, model_reports=importer.model_reports,
                  limitations=['client lighting and shaders', 'no retail terrain vertex tint',
                               'no M2 particles/ribbons', 'terrain holes reduced to 4x4'])
    staged.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n')
    staged.replace(args.out)
    staged.with_suffix('.json').replace(args.out.with_suffix('.json'))
    print(f'Built {args.out}: {len(importer.entries)} assets, {len(importer.models)} models, {len(importer.wmos)} WMOs')


if __name__ == '__main__':
    main()
