"""Import the local Forever Hall of Thanes environment into a private vanilla MPQ.

The source install is read-only. Existing retail conversion handles geometry and
props; the generated floor and Map.dbc row stay in ignored local asset archives.
Locally available HD/SD content variants are preferred over absent alternatives."""
import sys, json, hashlib, struct
from pathlib import Path
sys.dont_write_bytecode = True
import argparse
parser = argparse.ArgumentParser(description='Import Hall of Thanes from a local Forever install, without modifying it.')
parser.add_argument('--source', type=Path, required=True)
parser.add_argument('--tools', type=Path, required=True)
parser.add_argument('--listfile', type=Path, required=True)
parser.add_argument('--data', type=Path, default=Path('wow-data/Data'))
parser.add_argument('--out', type=Path, default=Path('out/forever-thanes/patch-R.MPQ'))
args = parser.parse_args()
if args.out.resolve().is_relative_to(args.source.resolve()):
    parser.error('Output must be outside the source install')
sys.path.insert(0, str(args.tools.resolve()))
import import_retail_elwynn as retail
from casc import Casc

class ForeverCasc(Casc):

    def _read_root(self, ekey):
        data = self.read_ekey(ekey)
        header, version = struct.unpack_from('<II', data, 4)
        if data[:4] != b'TSFM' or version not in (1, 2):
            raise ValueError('Unsupported root')
        pos = header
        self.files = {}
        while pos < len(data):
            if version == 2:
                count, locale, f1, f2, f3 = struct.unpack_from('<IIIIB', data, pos)
                content = f1 | f2 | f3 << 17
                pos += 17
            else:
                count, content, locale = struct.unpack_from('<III', data, pos)
                pos += 12
            deltas = struct.unpack_from(f'<{count}i', data, pos)
            pos += count * 4
            keys = pos
            pos += count * 16
            if not content & 268435456:
                pos += count * 8
            if not (locale & 2 or locale in (0, 4294967295, 131062)) or content & 128:
                continue
            ident = -1
            for i, delta in enumerate(deltas):
                ident += delta + 1
                ck = data[keys + i * 16:keys + i * 16 + 16]
                old = self.files.get(ident)
                available = lambda k: k in self.encoding and self.encoding[k][:9] in self.index
                if old is None or (not available(old) and available(ck)):
                    self.files[ident] = ck
from photo_archive import build
retail.PREFIX = 'World\\ASRForeverThanes\\'

class ThanesImporter(retail.Importer):

    def __init__(self, source, listfile):
        c = ForeverCasc.__new__(ForeverCasc)
        c.root_dir = Path(source)
        c.data_dir = c.root_dir / 'Data' / 'data'
        c.archives = {}
        rows = c.root_dir.joinpath('.build.info').read_text().splitlines()
        fields = [v.split('!')[0] for v in rows[0].split('|')]
        row = next((r for line in rows[1:] if (r := dict(zip(fields, line.split('|')))).get('Product') == 'wowf' and r.get('Active') == '1'))
        c.version = row['Version']
        c.build = c._config(row['Build Key'])
        c._read_indexes()
        c._read_encoding(bytes.fromhex(c.build['encoding'].split()[1]))
        c._read_root(c.ekey(bytes.fromhex(c.build['root'].split()[0])))
        self.storage = c
        self.ids = {}
        for line in Path(listfile).open(encoding='utf-8'):
            i, n = line.rstrip().split(';', 1)
            self.ids[n.lower()] = int(i)
        self.entries = {}
        self.models = {}
        self.wmos = {}
        self.provenance = {}
        self.model_reports = {}

    def wmo(self, ident):
        name = super().wmo(ident)
        root = dict(retail.chunks(self.entries[name]))
        count = struct.unpack_from('<I', root[b'MOHD'], 4)[0]
        liquid_types = None
        self.liquid_reports = []
        for index in range(count):
            group_name = name[:-4] + f'_{index:03}.wmo'
            data = self.entries[group_name]
            group = dict(retail.chunks(data))[b'MOGP']
            parts = dict(retail.chunks(group[68:]))
            kind = None
            liquid_id = struct.unpack_from('<I', group, 0x34)[0]
            if parts.get(b'MLIQ'):
                if liquid_types is None:
                    from db2 import Table
                    raw = self.read(1371380)
                    if struct.unpack_from('<I', raw, 156)[0] != 0xD1ECEEC9:
                        raise ValueError('Unsupported Forever LiquidType schema')
                    liquid_types = Table(raw)
                row = liquid_types.rows.get(liquid_id)
                # D1ECEEC9: SoundBank (field 3) is the basic liquid class.
                # https://github.com/wowdev/WoWDBDefs/blob/master/definitions/LiquidType.dbd
                if row is None or row[3] not in (0, 1, 2, 3):
                    raise ValueError(f'Unsupported liquid class for LiquidType {liquid_id}')
                kind = row[3]
            self.entries[group_name] = legacy_group(data, kind)
            self.liquid_reports.append(dict(group=index, source_type=liquid_id,
                                            legacy_kind=kind, has_pool=kind is not None))
        return name

    def texture(self, ident):
        if ident == 0:
            name = retail.PREFIX + 'white.blp'
            if name not in self.entries:
                self.entries[name] = struct.pack('<4sI4BII16I16I', b'BLP2', 1, 3, 8, 0, 0, 1, 1, 1172, *[0] * 15, 4, *[0] * 15) + bytes(1024) + bytes([255] * 4)
            return name
        return super().texture(ident)

    def model(self, ident):
        result = super().model(ident)
        if len(self.models) % 32 == 0:
            print('Converted models', len(self.models), flush=True)
        return result


def legacy_group(data, liquid_kind=None):
    """Translate modern triangle flags and wide material IDs to vanilla records.

    MPY2 has two uint16 fields; MOPY has two uint8 fields. Modern MOBA flag
    0x2 selects a uint16 material at +10 instead of the byte at +23.
    https://skarndev.github.io/wowlib/python/wmo/group-chunks/
    """
    outer = dict(retail.chunks(data))
    header = bytearray(outer[b'MOGP'][:68])
    parts = list(retail.chunks(outer[b'MOGP'][68:]))
    # Modern groupLiquid is a LiquidType ID (0 means no liquid). Vanilla
    # instead treats every value except 0xf as an unbounded flooded room.
    # Always use the vanilla sentinel; real pools get their class per MLIQ tile.
    struct.pack_into('<I', header, 0x34, 0xf)
    if not any(tag == b'MLIQ' and payload for tag, payload in parts):
        flags = struct.unpack_from('<I', header, 8)[0]
        struct.pack_into('<I', header, 8, flags & ~0x1000)
    converted = []
    for tag, payload in parts:
        if tag == b'MLIQ' and payload:
            if liquid_kind not in (0, 1, 2, 3):
                raise ValueError('An actual liquid pool needs a translated liquid class')
            if len(payload) < 30:
                raise ValueError('Truncated MLIQ header')
            xv, yv, xt, yt = struct.unpack_from('<4I', payload)
            tile_at = 30 + xv * yv * 8
            if xv != xt + 1 or yv != yt + 1 or tile_at + xt * yt != len(payload):
                raise ValueError('Unsupported MLIQ vertex/tile layout')
            pool = bytearray(payload)
            for at in range(tile_at, len(pool)):
                if pool[at] & 0xf != 0xf:
                    pool[at] = (pool[at] & 0xf0) | liquid_kind
            payload = bytes(pool)
        elif tag == b'MPY2':
            if len(payload) % 4:
                raise ValueError('Truncated MPY2 triangle table')
            triangles = bytearray()
            for flags, material in struct.iter_unpack('<HH', payload):
                if material == 0xffff:
                    material = 0xff
                elif material >= 0xff:
                    raise ValueError('WMO triangle material exceeds vanilla limits')
                # The high bit 0x100 selects a retail-only ground-type query.
                if flags & ~0x1ff:
                    raise ValueError(f'Unknown WMO triangle flags {flags:#x}')
                triangles.extend((flags & 0xff, material))
            tag, payload = b'MOPY', bytes(triangles)
        elif tag == b'MOBA':
            if len(payload) % 24:
                raise ValueError('Truncated MOBA render batch table')
            batches = bytearray(payload)
            for at in range(0, len(batches), 24):
                if batches[at + 22] & 2:
                    material = struct.unpack_from('<H', batches, at + 10)[0]
                    if material >= 0xff:
                        raise ValueError('WMO batch material exceeds vanilla limits')
                    batches[at + 23] = material
                    batches[at + 22] &= ~2
            payload = bytes(batches)
        converted.append((tag, payload))
    table = dict(converted)
    triangle_count = len(table.get(b'MOVI', b'')) // 6
    if len(table.get(b'MOPY', b'')) != triangle_count * 2:
        raise ValueError('WMO triangle collision table does not match geometry')
    outer[b'MOGP'] = header + b''.join(retail.chunk(tag, payload) for tag, payload in converted)
    return b''.join(retail.chunk(tag, payload) for tag, payload in outer.items())

def main():
    from durotar_assets import Chain
    from thanes_floor import build_floor
    from photo_archive import read_stored
    a = ThanesImporter(args.source, args.listfile)
    try:
        name = a.wmo(7732775)
        ground, stats = build_floor(a)
        a.entries[retail.PREFIX + 'arena.ground'] = ground
        placement = struct.pack('<II12f4H', 0, 69464566, 0, 0, 500, 0, 0, 0, *[0] * 6, 0, 0, 0, 1024)
        folder = 'ASRHallOfThanes'
        a.entries[f'World\\Maps\\{folder}\\{folder}.wdt'] = retail.chunk(b'MVER', struct.pack('<I', 18)) + retail.chunk(b'MPHD', struct.pack('<8I', 1, *[0] * 7)) + retail.chunk(b'MAIN', bytes(64 * 64 * 8)) + retail.chunk(b'MWMO', name.encode('ascii') + b'\x00') + retail.chunk(b'MODF', placement)
        chain = Chain(args.data, exclude=(args.out.name,))
        try:
            raw = chain.read('DBFilesClient\\Map.dbc')
        finally:
            chain.close()
        magic, count, fields, stride, size = struct.unpack_from('<4s4I', raw)
        if (magic, fields, stride) != (b'WDBC', 42, 168):
            raise ValueError('Unsupported vanilla Map.dbc')
        rows = [bytearray(raw[20 + i * stride:20 + (i + 1) * stride]) for i in range(count)]
        rows = [row for row in rows if struct.unpack_from('<I', row)[0] != 3065]
        names = bytearray(raw[20 + count * stride:])
        row = bytearray(stride)

        def text(field, value):
            struct.pack_into('<I', row, field * 4, len(names))
            names.extend(value.encode('ascii') + b'\x00')
        struct.pack_into('<I', row, 0, 3065)
        text(1, folder)
        struct.pack_into('<I', row, 8, 1)
        text(4, 'Hall of Thanes')
        rows.append(row)
        a.entries['DBFilesClient\\Map.dbc'] = struct.pack('<4s4I', magic, len(rows), fields, stride, len(names)) + b''.join(rows) + names
        archive = build(a.entries)
        if len(read_stored(archive)) != len(a.entries) + 1:
            raise ValueError('Incomplete archive')
        args.out.parent.mkdir(parents=True, exist_ok=True)
        staged = args.out.with_suffix('.MPQ.tmp')
        staged.write_bytes(archive)
        staged.replace(args.out)
        report = dict(source=str(args.source.resolve()), product='wowf', version=a.storage.version, map_id=3065, source_wmo=7732775, arena=stats, entries=len(a.entries), models=len(a.models), groups=20, sha256=hashlib.sha256(archive).hexdigest(), source_sha256=a.provenance, model_reports=a.model_reports, liquid_groups=a.liquid_reports, limitations=['ASR lighting and water shaders', 'No M2 particles or ribbons', 'Survivors encounters use classic model stand-ins'])
        args.out.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(dict(output=str(args.out), bytes=len(archive), entries=len(a.entries), models=len(a.models), arena=stats)), flush=True)
    finally:
        a.storage.close()
if __name__ == '__main__':
    main()
