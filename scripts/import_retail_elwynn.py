"""Import the retail Elwynn arena and its 3x3 surrounding tiles into a local MPQ.

Reads the player's CASC install without changing it. Requires the ASR asset tools
(casc.py, retail_m2.py, photo_archive.py); no game assets belong in git.
Run with --help for paths. Output is staged until every dependency converts.
Retail shaders, terrain vertex tint and particle/ribbon effects are not carried;
the client renders the retail geometry and diffuse textures with its own lighting.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys

PREFIX = 'World\\ASRRetailElwynn\\'
TILES = [(x, y) for y in range(48, 51) for x in range(32, 35)]


def chunk(tag, data):
    return tag[::-1] + struct.pack('<I', len(data)) + data


def chunks(data):
    pos = 0
    while pos < len(data):
        if pos + 8 > len(data):
            raise ValueError('Truncated chunk header')
        tag, size = struct.unpack_from('<4sI', data, pos)
        pos += 8
        if pos + size > len(data):
            raise ValueError(f'Truncated {tag!r}')
        yield tag[::-1], data[pos:pos + size]
        pos += size


def uints(data):
    return list(struct.unpack(f'<{len(data) // 4}I', data))


def strings(names):
    blob, offsets = bytearray(), []
    for name in names:
        offsets.append(len(blob))
        blob += name.encode('ascii') + b'\0'
    return bytes(blob), struct.pack(f'<{len(offsets)}I', *offsets)


def decode_alpha(data, pos, compressed):
    if not compressed:
        result = data[pos:pos + 4096]
    else:
        result = bytearray()
        while len(result) < 4096:
            code = data[pos]
            pos += 1
            count = code & 127
            if not count:
                raise ValueError('Zero-length alpha run')
            if code & 128:
                result += bytes([data[pos]]) * count
                pos += 1
            else:
                result += data[pos:pos + count]
                pos += count
    if len(result) != 4096:
        raise ValueError('Invalid 8-bit alpha layer')
    return result


def encode_alpha(data):
    # Literal RLE records keep 8-bit precision through the vanilla reader.
    return b''.join(bytes([len(data[i:i + 127])]) + data[i:i + 127]
                    for i in range(0, len(data), 127))


def convert_layers(parts, effects=None):
    layers = [list(row) for row in struct.iter_unpack('<4I', parts[b'MCLY'])]
    if not 1 <= len(layers) <= 4:
        raise ValueError('Unsupported terrain layer count')
    weights = [decode_alpha(parts.get(b'MCAL', b''), row[2], bool(row[1] & 512))
               for row in layers[1:]]
    alpha = bytearray()
    # Retail weights are additive; the client's splats use sequential lerps.
    for i, row in enumerate(layers):
        row[3] = effects(row[3]) if effects else 0
        if i == 0:
            row[1], row[2] = row[1] & ~768, 0
        else:
            values = bytearray(4096)
            for p in range(4096):
                remaining = 255 - sum(w[p] for w in weights[i:])
                values[p] = min(255, round(weights[i - 1][p] * 255 / remaining)) if remaining > 0 else 0
            row[1], row[2] = (row[1] | 768), len(alpha)
            alpha += encode_alpha(values)
    return b''.join(struct.pack('<4I', *r) for r in layers), bytes(alpha)


def liquid_block(mh2o, index, formats):
    if not mh2o:
        return b''
    pos, count, _ = struct.unpack_from('<3I', mh2o, index * 12)
    if count == 0:
        return b''
    if count != 1:
        raise ValueError('Overlapping retail water layers need explicit conversion')
    kind, fmt, low, high, ox, oy, width, height, maskpos, vertices = struct.unpack_from(
        '<HHff4BII', mh2o, pos)
    if fmt >= 42:
        ox, oy, width, height = 0, 0, 8, 8
        fmt = formats[kind]
    if fmt not in (0, 1, 2, 3, 4, 5) or ox + width > 8 or oy + height > 8:
        raise ValueError(f'Unsupported retail liquid layout: {kind}, {fmt}')
    mask = int.from_bytes(mh2o[maskpos:maskpos + (width * height + 7) // 8], 'little') if maskpos else (1 << (width * height)) - 1
    n = (width + 1) * (height + 1)
    out = bytearray(struct.pack('<ff', low, high) + bytes(81 * 8) + bytes([15]) * 64 + bytes(84))
    for y in range(9):
        for x in range(9):
            k = min(height, max(0, y - oy)) * (width + 1) + min(width, max(0, x - ox))
            z, depth = low, 255
            if vertices:
                if fmt in (0, 1, 3):
                    z = struct.unpack_from('<f', mh2o, vertices + k * 4)[0]
                elif fmt in (4, 5):
                    z = struct.unpack_from('<f', mh2o, vertices + k * 8 + 4)[0]
                if fmt in (0, 2, 3, 4):
                    offset = {0: n * 4 + k, 2: k, 3: n * 8 + k, 4: k * 8}[fmt]
                    depth = mh2o[vertices + offset]
            struct.pack_into('<If', out, 8 + 8 * (y * 9 + x), min(255, depth), z)
    # Elwynn's imported surfaces are fresh water, not ocean or magma.
    if kind not in (1, 5, 41):
        raise ValueError(f'Unexpected Elwynn liquid type {kind}')
    for y in range(height):
        for x in range(width):
            if mask & (1 << (y * width + x)):
                out[8 + 81 * 8 + (y + oy) * 8 + x + ox] = 0
    return bytes(out)


class Importer:
    def __init__(self, source, listfile):
        from casc import Casc
        self.storage = Casc(source)
        self.ids = {}
        for line in listfile.read_text(encoding='utf-8').splitlines():
            ident, name = line.split(';', 1)
            self.ids[name.lower()] = int(ident)
        self.entries, self.models, self.wmos = {}, {}, {}
        self.provenance, self.model_reports = {}, {}
        self.effects, self.detail_models, self.absent_effects = {}, {}, set()
        from db2 import Table
        types, materials = self.read(1371380), self.read(1132538)
        if struct.unpack_from('<I', types, 156)[0] != 0xD1ECEEC9 or struct.unpack_from('<I', materials, 156)[0] != 0x98E5D7AA:
            raise ValueError('Unsupported retail liquid DB2 schema')
        tr, mr = Table(types).rows, Table(materials).rows
        self.liquid_formats = {kind: mr[row[14]][1] for kind, row in tr.items() if row[14] in mr}
        effect_data, doodad_data = self.read(1308499), self.read(1308057)
        if struct.unpack_from('<I', effect_data, 156)[0] != 0xD93D5678 or struct.unpack_from('<I', doodad_data, 156)[0] != 0xCFD94A21:
            raise ValueError('Unsupported retail ground-effect DB2 schema')
        self.effect_rows, self.doodad_rows = Table(effect_data).rows, Table(doodad_data).rows

    def effect(self, ident):
        if ident in (0, 0xffffffff):
            return 0xffffffff
        if ident not in self.effect_rows:
            self.absent_effects.add(ident)
            return 0xffffffff  # A retired DB2 ID has no retail ground-cover definition.
        if ident not in self.effects:
            row = self.effect_rows[ident]
            slots = []
            for doodad in row[2]:
                if not doodad:
                    slots.append(0xffffffff)  # Vanilla ID zero names a real flower.
                    continue
                if doodad not in self.detail_models:
                    fd = self.doodad_rows[doodad][0]
                    model = self.model(fd)
                    name = f'RetailElwynnDetail{fd}'
                    self.entries[f'World\\NoDXT\\Detail\\{name}.m2'] = self.entries[model]
                    self.detail_models[doodad] = name
                slots.append(1000000 + doodad)
            self.effects[ident] = [1000000 + ident, *slots, row[0], row[1]]
        return 1000000 + ident

    def ground_tables(self, data_dir):
        from durotar_assets import Chain
        game = Chain(data_dir, exclude=('patch-Q.MPQ',))
        try:
            for table, extra in (
                ('GroundEffectTexture', list(self.effects.values())),
                ('GroundEffectDoodad', [[1000001 + ident, 1000000 + ident, name + '.mdx']
                                       for ident, name in self.detail_models.items()]),
            ):
                path = f'DBFilesClient\\{table}.dbc'
                raw = game.read(path)
                magic, count, fields, stride, strings_size = struct.unpack_from('<4s4I', raw)
                if magic != b'WDBC' or fields != (7 if table == 'GroundEffectTexture' else 3):
                    raise ValueError(f'Unsupported vanilla {table} schema')
                records = bytearray(raw[20:20 + count * stride])
                names = bytearray(raw[20 + count * stride:])
                existing = {struct.unpack_from('<I', records, i * stride)[0] for i in range(count)}
                for row in extra:
                    if row[0] in existing:
                        raise ValueError('Retail ground effect ID collision')
                    row = list(row)
                    if isinstance(row[-1], str):
                        name = row[-1]
                        row[-1] = len(names)
                        names += name.encode('ascii') + b'\0'
                    records += struct.pack(f'<{fields}I', *row)
                self.entries[path] = struct.pack('<4s4I', magic, count + len(extra), fields, stride, len(names)) + records + names
        finally:
            game.close()

    def read(self, ident):
        if isinstance(ident, str):
            ident = self.ids[ident.lower().replace('\\', '/')]
        data = self.storage.read(ident)
        if data is None:
            raise FileNotFoundError(f'Retail FileDataID {ident} is not installed')
        self.provenance[str(ident)] = hashlib.sha256(data).hexdigest()
        return data

    def texture(self, ident):
        name = PREFIX + f'{ident}.blp'
        if name not in self.entries:
            data = self.read(ident)
            if data[:4] != b'BLP2':
                raise ValueError(f'Texture {ident} is not BLP2')
            self.entries[name] = data
        return name

    def model(self, ident):
        from retail_m2 import Converter, chunks as m2chunks, arr
        if ident in self.models:
            return self.models[ident]
        data = self.read(ident)
        parts = m2chunks(data)
        model = parts['MD21']
        skin = self.read(uints(parts['SFID'])[0])
        skeleton = self.read(uints(parts['SKID'])[0]) if 'SKID' in parts else None
        sn, so = arr(skin, 28)
        geosets = {struct.unpack_from('<H', skin, so + i * 48)[0]: 0 for i in range(sn)}
        texture_names = {i: self.texture(fd) for i, fd in enumerate(uints(parts.get('TXID', b''))) if fd}
        # Legacy retail M2s can still store texture filenames instead of TXID.
        tn, to = arr(model, 0x50)
        for i in range(tn):
            kind, _, length, offset = struct.unpack_from('<4I', model, to + i * 16)
            if kind == 0 and i not in texture_names and length:
                path = model[offset:offset + length].rstrip(b'\0').decode('ascii')
                texture_names[i] = self.texture(self.ids[path.lower().replace('\\', '/')])
        c = Converter(data, skeleton, skin, f'Retail Elwynn {ident}', set(range(1024)),
                      geosets, [], remap_body=False, texture_names=texture_names)
        converted = c.convert()
        if c.report['dropped'] or (not c.report['indices'] and arr(skin, 12)[0]):
            raise ValueError(f'Model {ident} lost geometry: {c.report}')
        name = PREFIX + f'{ident}.m2'
        self.entries[name] = converted
        self.models[ident] = name
        self.model_reports[str(ident)] = c.report
        return name

    def wmo(self, ident):
        if ident in self.wmos:
            return self.wmos[ident]
        root = dict(chunks(self.read(ident)))
        name = PREFIX + f'{ident}.wmo'
        mats, texnames = bytearray(root[b'MOMT']), []
        for i in range(0, len(mats), 64):
            fd = struct.unpack_from('<I', mats, i + 12)[0]
            texnames.append(self.texture(fd))
        tx, offsets = strings(texnames)
        for i, offset in enumerate(uints(offsets)):
            struct.pack_into('<I', mats, i * 64 + 12, offset)
            # A fixed-function diffuse pass; normal/specular maps are not MOTX offsets.
            struct.pack_into('<I', mats, i * 64 + 4, 0)
            struct.pack_into('<I', mats, i * 64 + 24, 0)
            struct.pack_into('<I', mats, i * 64 + 36, 0)
        root[b'MOTX'], root[b'MOMT'] = tx, bytes(mats)
        doodad_ids = uints(root.get(b'MODI', b''))
        doodads = bytearray(root.get(b'MODD', b''))
        used = {struct.unpack_from('<I', doodads, i)[0] & 0xffffff
                for i in range(0, len(doodads), 40)}
        dn, do = strings([self.model(fd) if i in used else ''
                          for i, fd in enumerate(doodad_ids)])
        for i in range(0, len(doodads), 40):
            value = struct.unpack_from('<I', doodads, i)[0]
            offset = struct.unpack_from('<I', do, 4 * (value & 0xffffff))[0]
            struct.pack_into('<I', doodads, i, (value & 0xff000000) | offset)
        root[b'MODN'], root[b'MODD'] = dn, bytes(doodads)
        groups = uints(root[b'GFID'])
        count = struct.unpack_from('<I', root[b'MOHD'], 4)[0]
        for i, fd in enumerate(groups[:count]):
            group = dict(chunks(self.read(fd)))
            header = bytearray(group[b'MOGP'][:68])
            exterior = bool(struct.unpack_from('<I', header, 8)[0] & 0x48)
            if exterior:
                # Retail exterior MOCV is additive light, often zero. Vanilla multiplies
                # it into the diffuse texture: zero would turn a sunlit facade black.
                flags = struct.unpack_from('<I', header, 8)[0]
                struct.pack_into('<I', header, 8, flags & ~4)
            first, seen = [], set()
            for tag, payload in chunks(group[b'MOGP'][68:]):
                if exterior and tag == b'MOCV':
                    continue
                if tag in (b'MOTV', b'MOCV') and tag in seen:
                    continue
                seen.add(tag)
                first.append(chunk(tag, payload))
            group[b'MOGP'] = header + b''.join(first)
            self.entries[name[:-4] + f'_{i:03}.wmo'] = b''.join(chunk(t, b) for t, b in group.items())
        for tag in (b'GFID', b'MODI', b'MOSB'):
            root.pop(tag, None)
        self.entries[name] = b''.join(chunk(t, b) for t, b in root.items())
        self.wmos[ident] = name
        return name

    def tile(self, x, y):
        path = f'world/maps/azeroth/azeroth_{x}_{y}'
        parts = list(chunks(self.read(path + '.adt')))
        root = dict(parts)
        texparts = list(chunks(self.read(path + '_tex0.adt')))
        tex = dict(texparts)
        obj = dict(chunks(self.read(path + '_obj0.adt')))
        textures = [self.texture(fd) for fd in uints(tex[b'MDID'])]
        mddf, modf = bytearray(obj.get(b'MDDF', b'')), bytearray(obj.get(b'MODF', b''))
        models, wmos = [], []
        for i in range(0, len(mddf), 36):
            fd = struct.unpack_from('<I', mddf, i)[0]
            flags = struct.unpack_from('<H', mddf, i + 34)[0]
            if not flags & 64:
                raise ValueError('Expected MDDF FileDataID')
            models.append(self.model(fd))
            struct.pack_into('<I', mddf, i, len(models) - 1)
            struct.pack_into('<H', mddf, i + 34, 0)
        for i in range(0, len(modf), 64):
            fd = struct.unpack_from('<I', modf, i)[0]
            flags, _, _, scale = struct.unpack_from('<4H', modf, i + 56)
            if not flags & 8 or scale not in (0, 1024):
                raise ValueError('Unsupported WMO placement')
            wmos.append(self.wmo(fd))
            struct.pack_into('<I', modf, i, len(wmos) - 1)
            struct.pack_into('<H', modf, i + 56, flags & ~12)
        md, mi = strings(models)
        wd, wi = strings(wmos)
        out = chunk(b'MVER', struct.pack('<I', 18))
        out += chunk(b'MTEX', strings(textures)[0])
        out += chunk(b'MMDX', md) + chunk(b'MMID', mi) + chunk(b'MDDF', mddf)
        out += chunk(b'MWMO', wd) + chunk(b'MWID', wi) + chunk(b'MODF', modf)
        cells = [b for t, b in parts if t == b'MCNK']
        texcells = [dict(chunks(b)) for t, b in texparts if t == b'MCNK']
        if len(cells) != 256 or len(texcells) != 256:
            raise ValueError('Incomplete tile')
        for index, (cell, texture) in enumerate(zip(cells, texcells, strict=True)):
            h = bytearray(cell[:128])
            flags = struct.unpack_from('<I', h)[0]
            # Retain only vanilla flags. Compress 8x8 holes conservatively to 4x4.
            if flags & 0x10000:
                highholes = int.from_bytes(h[20:28], 'little')
                holes = sum(1 << (yy * 4 + xx) for yy in range(4) for xx in range(4)
                            if any(highholes & (1 << ((yy * 2 + dy) * 8 + xx * 2 + dx))
                                   for dy in range(2) for dx in range(2)))
                struct.pack_into('<I', h, 60, holes)
            struct.pack_into('<I', h, 0, flags & 3)
            for off in (12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 56, 88, 92, 96, 100, 116, 120, 124):
                struct.pack_into('<I', h, off, 0)
            # Retail effect/area fields otherwise retain vanilla-compatible coordinates.
            components = dict(chunks(cell[128:]))
            layers, alpha = convert_layers(texture, self.effect)
            struct.pack_into('<I', h, 12, len(layers) // 16)
            body = bytearray()
            def add(tag, payload, offset, size=None):
                struct.pack_into('<I', h, offset, 128 + len(body) + 8)
                if size is not None:
                    struct.pack_into('<I', h, size, len(payload))
                body.extend(chunk(tag, payload))
            add(b'MCVT', components[b'MCVT'], 20)
            add(b'MCNR', components[b'MCNR'], 24)
            add(b'MCLY', layers, 28)
            if alpha:
                add(b'MCAL', alpha, 36, 40)
            if texture.get(b'MCSH'):
                add(b'MCSH', texture[b'MCSH'], 44, 48)
            water = liquid_block(root.get(b'MH2O'), index, self.liquid_formats)
            if water:
                struct.pack_into('<I', h, 0, (flags & 3) | 4)
                add(b'MCLQ', water, 96, 100)
            out += chunk(b'MCNK', h + body)
        name = path.replace('/', '\\') + '.adt'
        self.entries[name] = out
        print(f'Converted Elwynn tile {x},{y}: {len(models)} props, {len(wmos)} buildings', flush=True)

    def minimaps(self, data_dir):
        from durotar_assets import Chain
        game = Chain(data_dir, exclude=('patch-Q.MPQ',))
        try:
            original = game.read('Textures\\Minimap\\md5translate.trs').decode('ascii')
        finally:
            game.close()
        wanted = {f'azeroth\\map{x}_{y}.blp': (x, y) for x, y in TILES}
        lines, replaced = [], set()
        for line in original.splitlines():
            key = line.split('\t', 1)[0].lower()
            if key in wanted:
                x, y = wanted[key]
                image = self.read(f'world/minimaps/azeroth/map{x}_{y}.blp')
                name = f'asr-retail-elwynn-{x}-{y}.blp'
                self.entries['Textures\\Minimap\\' + name] = image
                line = line.split('\t', 1)[0] + '\t' + name
                replaced.add(key)
            lines.append(line)
        if replaced != wanted.keys():
            raise ValueError('Elwynn minimap index is incomplete')
        self.entries['Textures\\Minimap\\md5translate.trs'] = ('\r\n'.join(lines) + '\r\n').encode('ascii')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True, help='Retail folder containing .build.info')
    parser.add_argument('--tools', type=Path, required=True, help='ASR launcher asset tools folder')
    parser.add_argument('--listfile', type=Path, required=True)
    parser.add_argument('--data', type=Path, default=Path('wow-data/Data'), help='Local vanilla Data for the minimap index')
    parser.add_argument('--out', type=Path, default=Path('out/retail-elwynn/patch-Q.MPQ'))
    args = parser.parse_args()
    if args.out.resolve().is_relative_to(args.source.resolve()):
        parser.error('--out must be outside the read-only retail install')
    sys.path.insert(0, str(args.tools.resolve()))
    from photo_archive import build, read_stored
    importer = Importer(args.source, args.listfile)
    for x, y in TILES:
        importer.tile(x, y)
    importer.minimaps(args.data)
    importer.ground_tables(args.data)
    archive = build(importer.entries)
    if len(read_stored(archive)) != len(importer.entries) + 1:
        raise ValueError('Archive read-back failed')
    args.out.parent.mkdir(parents=True, exist_ok=True)
    staged = args.out.with_name(args.out.stem + '-staging.MPQ')
    temporary = staged.with_suffix('.MPQ.tmp')
    temporary.write_bytes(archive)
    temporary.replace(staged)
    report = dict(source=str(args.source.resolve()), version=importer.storage.version,
                  tiles=TILES, entries=len(importer.entries), models=len(importer.models),
                  wmos=len(importer.wmos), ground_effects=len(importer.effects),
                  ground_models=len(importer.detail_models), sha256=hashlib.sha256(archive).hexdigest(),
                  absent_ground_effect_ids=sorted(importer.absent_effects),
                  source_sha256=importer.provenance, model_reports=importer.model_reports,
                  limitations=['client lighting and shaders', 'no retail terrain vertex tint',
                               'no M2 particles/ribbons', 'terrain holes conservatively reduced to 4x4'])
    staged.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n')
    from bake_elwynn_shadows import bake_archive
    size = bake_archive(staged, args.tools)
    staged.replace(args.out)
    staged.with_suffix('.json').replace(args.out.with_suffix('.json'))
    print(f'Built {args.out}: {len(importer.entries)} assets, {size:,} bytes')


if __name__ == '__main__':
    main()
