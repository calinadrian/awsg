"""Build a local player folder from the user's existing MPQ chain.

It carries every arena defined in the survivors' ALL_MAPS (playable or locked),
each as the terrain tiles around its centre, not whole continents.
Python 3.10+ and Rust are required. No game files are downloaded or committed.
The full source Data directory is opened read-only. Output stays under out/.
"""
import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import zipfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
TILE_YARDS = 1600 / 3
# Detailed tiles around each arena's centre, and coarse horizon tiles around those.
TILE_RADIUS, HORIZON_RADIUS = 1, 5
MASK = 0xFFFFFFFF


def canonical(name):
    name = name.strip().replace('/', '\\').lower()
    if name.endswith(('.mdx', '.mdl')):
        name = name[:-4] + '.m2'
    if not name or ':' in name or '..' in name.split('\\'):
        raise ValueError(f'Unsafe asset path: {name!r}')
    return name


class Reader:
    def __init__(self, source):
        executable = ROOT / 'target/release/examples' / ('elwynn_assets.exe' if os.name == 'nt' else 'elwynn_assets')
        self.process = subprocess.Popen([str(executable), str(source)], stdin=subprocess.PIPE, stdout=subprocess.PIPE)

    def read(self, name, optional=False):
        self.process.stdin.write(name.encode() + b'\n')
        self.process.stdin.flush()
        header = self.process.stdout.read(8)
        if len(header) != 8:
            raise RuntimeError('Asset reader stopped unexpectedly')
        length, = struct.unpack('<q', header)
        data = self.process.stdout.read(abs(length))
        if len(data) != abs(length):
            raise RuntimeError('Truncated asset-reader response')
        if length < 0:
            if optional and ('not in patch chain' in data.decode() or 'deleted from patch chain' in data.decode()):
                return None
            raise RuntimeError(data.decode())
        return data

    def close(self):
        self.process.stdin.close()
        self.process.stdout.close()
        if self.process.wait() != 0:
            raise RuntimeError('Asset reader failed')


def runtime_equipment():
    """The same catalog the game loads, including definitions added by Rust at runtime."""
    result = subprocess.run(
        ['cargo', 'run', '--quiet', '--release', '-p', 'asr-coop', '--example', 'catalog_table'],
        cwd=ROOT, check=True, capture_output=True, text=True, encoding='utf-8')
    return {int(line.split('\t')[3]) for line in result.stdout.splitlines()
            if line and int(line.split('\t')[3])}


def equipment_paths(table, item_ids, names):
    """All available race/sex variants of the selected ItemDisplayInfo components."""
    stems = set()
    for item in item_ids - {0}:
        row = table.by_id.get(item)
        if row is None:
            raise ValueError(f'Required equipment display {item} is missing')
        for column in (1, 2, 3, 4, *range(14, 22)):
            value = table.text(row[column]).lower()
            if value:
                stems.add(re.sub(r'\.(mdx|mdl|m2|blp)$', '', value))
    for name in sorted(names):
        if name.startswith('item\\'):
            stem = re.sub(r'\.(m2|mdx|blp)$', '', name.rsplit('\\', 1)[-1])
            if stem in stems or any(stem.startswith(base + '_') for base in stems):
                yield name


class Dbc:
    def __init__(self, raw):
        if raw[:4] != b'WDBC':
            raise ValueError('Expected vanilla WDBC table')
        count, fields, size, strings = struct.unpack_from('<4I', raw, 4)
        if size != fields * 4 or len(raw) != 20 + count * size + strings:
            raise ValueError('Malformed WDBC table')
        self.rows = list(struct.iter_unpack('<' + 'I' * fields, raw[20:20 + count * size]))
        self.strings = raw[20 + count * size:]
        self.by_id = {row[0]: row for row in self.rows}

    def text(self, offset):
        if offset >= len(self.strings):
            raise ValueError('DBC string offset outside table')
        return self.strings[offset:].split(b'\0', 1)[0].decode('utf-8')


def chunks(raw):
    at = 0
    while at + 8 <= len(raw):
        magic, size = struct.unpack_from('<4sI', raw, at)
        if at + 8 + size > len(raw):
            # Some shipped files declare a final chunk a byte or so past their end
            # (undercity_144.wmo's MOGP); the bytes are packed unchanged either way.
            if at + 8 + size - len(raw) > 16:
                raise ValueError('Truncated world chunk')
            size = len(raw) - at - 8
        yield magic[::-1], raw[at + 8:at + 8 + size]
        at += 8 + size
    if at != len(raw):
        raise ValueError('Trailing partial world chunk')


def strings(raw):
    return [part.decode('ascii') for part in raw.split(b'\0') if part]


# MPQ's hash/crypt tables, matching crates/benilla-mpq/src/crypto.rs.
def crypt_table():
    table, seed = [0] * 0x500, 0x100001
    for low in range(256):
        for high in range(5):
            seed = (seed * 125 + 3) % 0x2AAAAB
            first = (seed & 0xFFFF) << 16
            seed = (seed * 125 + 3) % 0x2AAAAB
            table[low + high * 256] = first | (seed & 0xFFFF)
    return table


CRYPT = crypt_table()


def name_hash(name, kind):
    first, second = 0x7FED7FED, 0xEEEEEEEE
    for byte in name.replace('/', '\\').upper().encode('ascii'):
        first = CRYPT[kind + byte] ^ ((first + second) & MASK)
        second = (byte + first + second + (second << 5) + 3) & MASK
    return first


def encrypt(words, key):
    seed, out = 0xEEEEEEEE, []
    for plain in words:
        seed = (seed + CRYPT[0x400 + (key & 255)]) & MASK
        out.append(plain ^ ((key + seed) & MASK))
        key = ((((~key & MASK) << 21) + 0x11111111) | (key >> 11)) & MASK
        seed = (plain + seed + (seed << 5) + 3) & MASK
    return struct.pack('<' + 'I' * len(out), *out)


class Archive:
    """Stream zlib-compressed MPQ sectors; no single-unit or encrypted file flags."""
    def __init__(self, path):
        self.file = path.open('wb')
        self.file.write(b'\0' * 32)
        self.entries = []

    def add(self, name, raw):
        position = self.file.tell()
        sectors = [raw[start:start + 4096] for start in range(0, len(raw), 4096)]
        encoded = []
        offset = (len(sectors) + 1) * 4
        offsets = [offset]
        for sector in sectors:
            compressed = b'\x02' + zlib.compress(sector, 6)
            block = compressed if len(compressed) < len(sector) else sector
            encoded.append(block)
            offset += len(block)
            offsets.append(offset)
        self.file.write(struct.pack('<' + 'I' * len(offsets), *offsets))
        for block in encoded:
            self.file.write(block)
        self.entries.append((name, position, offset, len(raw)))

    def finish(self):
        self.add('(listfile)', ('\r\n'.join(name for name, *_ in self.entries) + '\r\n').encode('ascii'))
        slots = 1
        while slots < len(self.entries) * 2:
            slots *= 2
        hashes = [[MASK] * 4 for _ in range(slots)]
        blocks = []
        for index, (name, position, compressed, size) in enumerate(self.entries):
            slot = name_hash(name, 0) & (slots - 1)
            while hashes[slot][3] != MASK:
                slot = (slot + 1) & (slots - 1)
            hashes[slot] = [name_hash(name, 0x100), name_hash(name, 0x200), 0, index]
            blocks.extend((position, compressed, size, 0x80000200))
        hash_at = self.file.tell()
        self.file.write(encrypt([word for row in hashes for word in row], name_hash('(hash table)', 0x300)))
        block_at = self.file.tell()
        self.file.write(encrypt(blocks, name_hash('(block table)', 0x300)))
        size = self.file.tell()
        if size >= 2**32:
            raise ValueError('Arena archive exceeds the MPQ v1 size limit')
        self.file.seek(0)
        self.file.write(struct.pack('<4sIIHHIIII', b'MPQ\x1a', 32, size, 0, 3, hash_at, block_at, slots, len(self.entries)))
        self.file.close()


class Package:
    def __init__(self, reader, archive):
        self.reader, self.archive = reader, archive
        self.pending = collections.deque()
        self.seen, self.manifest = set(), []
        self.dbcs, self.area_ids, self.effect_ids = {}, {12}, set()
        self.wmo_ids, self.sound_ids = set(), set()
        self.source_missing, self.legacy_models = [], []

    def add(self, name, reason):
        if ':' in name or '..' in name.replace('/', '\\').split('\\'):
            self.source_missing.append(dict(path=name, reason=reason + ': nonportable path authored in source asset'))
            return
        self.pending.append((canonical(name), reason))

    def table(self, name):
        key = canonical('DBFilesClient/' + name + '.dbc')
        if key not in self.dbcs:
            self.dbcs[key] = Dbc(self.reader.read(key))
        return self.dbcs[key]

    def dependency(self, name, raw):
        if name.endswith(('.adt', '.wdt', '.wmo')):
            parts = list(chunks(raw))
            for tag, data in parts:
                if tag in (b'MTEX', b'MMDX', b'MWMO', b'MOTX', b'MODN', b'MOSB'):
                    for path in strings(data):
                        self.add(path, name)
                elif tag == b'MOHD':
                    count = struct.unpack_from('<I', data, 4)[0]
                    if count > 4096:
                        raise ValueError('Invalid WMO group count')
                    if identifier := struct.unpack_from('<I', data, 32)[0]:
                        self.wmo_ids.add(identifier)
                    for index in range(count):
                        self.add(name[:-4] + f'_{index:03}.wmo', name)
                elif tag == b'MCNK':
                    if name.endswith('.adt'):
                        self.area_ids.add(struct.unpack_from('<I', data, 52)[0])
                    # Sub-chunks are found through the header's offsets (from the MCNK chunk's own
                    # header), not walked: some tiles' sub-chunk sizes do not tile the body.
                    def sub(ofs_at, tag):
                        at = struct.unpack_from('<I', data, ofs_at)[0] - 8
                        if at < 128 or at + 8 > len(data) or data[at:at + 4][::-1] != tag:
                            return None
                        return data[at + 8:at + 8 + struct.unpack_from('<I', data, at + 4)[0]]
                    if layers := sub(0x1C, b'MCLY'):
                        for row in struct.iter_unpack('<4I', layers[:len(layers) // 16 * 16]):
                            if row[3]:
                                self.effect_ids.add(row[3] & 0xFFFF)
                    # SoundEmitter IDs are resolved through SoundEntries.
                    if (emitters := sub(0x5C, b'MCSE')) and len(emitters) % 28 == 0:
                        self.sound_ids.update(struct.unpack_from('<I', emitters, i)[0] for i in range(0, len(emitters), 28))
        elif name.endswith('.m2'):
            if raw[:8] != b'MD20\x00\x01\x00\x00':
                # Some unused vanilla spell effects contain older model formats.
                # Preserve their bytes, without pretending to parse that layout.
                self.legacy_models.append(name)
                for match in re.finditer(rb'[A-Za-z][A-Za-z0-9_ \\/.-]{2,240}\.blp\x00', raw, re.I):
                    self.add(match.group()[:-1].decode('ascii'), name)
                return
            count, offset = struct.unpack_from('<II', raw, 0x5C)
            for index in range(count):
                kind, _, size, at = struct.unpack_from('<4I', raw, offset + index * 16)
                if kind == 0 and size:
                    self.add(raw[at:at + size].rstrip(b'\0').decode('ascii'), name)

    def drain(self, overrides=None):
        overrides = overrides or {}
        while self.pending:
            name, reason = self.pending.popleft()
            if name in self.seen:
                continue
            raw = overrides[name] if name in overrides else self.reader.read(name, optional=True)
            self.seen.add(name)
            if raw is None:
                self.source_missing.append(dict(path=name, reason=reason))
                continue
            self.dependency(name, raw)
            self.archive.add(name, raw)
            self.manifest.append(dict(path=name, bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), reason=reason))
            if len(self.manifest) % 1000 == 0:
                print(f'Packed {len(self.manifest):,} assets', flush=True)

    def creatures(self, ids):
        displays, models, extras = (self.table(name) for name in ('CreatureDisplayInfo', 'CreatureModelData', 'CreatureDisplayInfoExtra'))
        item_ids, appearances = set(), set()
        voices = self.table('CreatureSoundData')
        for display in sorted(ids):
            row = displays.by_id.get(display)
            if row is None:
                raise ValueError(f'Required creature display {display} is missing from source CreatureDisplayInfo')
            model = models.by_id[row[1]]
            path = canonical(models.text(model[2]))
            self.add(path, f'creature display {display}')
            directory = path.rsplit('\\', 1)[0]
            for offset in row[6:9]:
                texture = displays.text(offset)
                if texture and not row[3]:
                    self.add(directory + '/' + texture + ('' if texture.lower().endswith('.blp') else '.blp'), f'creature skin {display}')
            sound = row[2] or model[13]
            if sound in voices.by_id:
                self.sound_ids.update(value for i, value in enumerate(voices.by_id[sound]) if i not in (0, 9, 22, 24))
            if row[3]:
                extra = extras.by_id[row[3]]
                appearances.add(tuple(extra[1:8]))
                item_ids.update(extra[8:18])
                bake = extras.text(extra[18])
                if bake:
                    self.add('Textures/BakedNpcTextures/' + bake + ('' if bake.lower().endswith('.blp') else '.blp'), f'appearance {display}')
        # The menu's FEATURES dials offer every skin, face and hair row of a hero's race and sex,
        # so every row of a carried body ships, not only the displays' own looks.
        sections = self.table('CharSections')
        bodies = {(race, sex) for race, sex, *_ in appearances}
        for row in sections.rows:
            if (row[1], row[2]) in bodies:
                for offset in row[6:9]:
                    if path := sections.text(offset):
                        self.add(path, 'playable/NPC character appearance')
        return item_ids

    def equipment(self, item_ids, names):
        table = self.table('ItemDisplayInfo')
        # Sex/race suffixes are selected at runtime. Match the selected display's
        # basenames against the complete listfile, rather than retain all equipment.
        for name in equipment_paths(table, item_ids, names):
            self.add(name, 'selected character equipment')

    def environment(self):
        textures, doodads = self.table('GroundEffectTexture'), self.table('GroundEffectDoodad')
        by_internal = {row[1]: row for row in doodads.rows}
        for effect in self.effect_ids:
            if row := textures.by_id.get(effect):
                for identifier in row[1:5]:
                    if identifier != MASK and identifier in by_internal:
                        path = doodads.text(by_internal[identifier][2]).replace('/', '\\').rsplit('\\', 1)[-1]
                        self.add('World/NoDXT/Detail/' + path, 'arena ground cover')
        areas = self.table('AreaTable')
        music, intros, beds = (self.table(name) for name in ('ZoneMusic', 'ZoneIntroMusicTable', 'SoundAmbience'))
        wmo_areas = self.table('WMOAreaTable')
        for row in wmo_areas.rows:
            if row[1] in self.wmo_ids:
                self.area_ids.add(row[10])
                if bed := beds.by_id.get(row[6]):
                    self.sound_ids.update(bed[1:3])
                if song := music.by_id.get(row[7]):
                    self.sound_ids.update(song[6:8])
                if intro := intros.by_id.get(row[8]):
                    self.sound_ids.add(intro[2])
        # Include parent rows: outdoor sound can be inherited by a subarea.
        for identifier in list(self.area_ids):
            while identifier in areas.by_id:
                self.area_ids.add(identifier)
                parent = areas.by_id[identifier][2]
                if not parent or parent == identifier:
                    break
                identifier = parent
        for identifier in self.area_ids:
            if row := areas.by_id.get(identifier):
                # AreaTable columns 7 ambience, 8 music, 9 intro in build 5875.
                if bed := beds.by_id.get(row[7]):
                    self.sound_ids.update(bed[1:3])
                if song := music.by_id.get(row[8]):
                    self.sound_ids.update(song[6:8])
                if intro := intros.by_id.get(row[9]):
                    self.sound_ids.add(intro[2])


def build():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, default=ROOT / 'wow-data/Data')
    parser.add_argument('--out', type=Path, default=ROOT / 'out/elwynn-playable')
    parser.add_argument('--data-only', action='store_true', help='Build the archive only; do not build/copy a player executable')
    parser.add_argument('--skip-reader-build', action='store_true', help='Use an already compiled elwynn_assets helper')
    parser.add_argument('--zip', action='store_true', help='Also create a ZIP beside the output folder')
    parser.add_argument('--include-coop-server', action='store_true', help='Include the operator relay executable and startup script')
    args = parser.parse_args()
    source, output = args.data.resolve(), args.out.resolve()
    if not output.is_relative_to(ROOT / 'out') or output == ROOT / 'out' or source == output or output.is_relative_to(source) or source.is_relative_to(output):
        parser.error('Output must be a separate child of this checkout\'s ignored out/ directory')
    output.mkdir(parents=True, exist_ok=True)
    (output / 'Data').mkdir(exist_ok=True)
    if not args.skip_reader_build:
        subprocess.run(['cargo', 'build', '--release', '-p', 'benilla-formats', '--example', 'elwynn_assets'], cwd=ROOT, check=True)
    reader = Reader(source)
    temporary = output / 'Data/patch.MPQ.tmp'
    archive = Archive(temporary)
    package = Package(reader, archive)
    try:
        listed = {canonical(line.split('\t', 1)[1]) for line in reader.read('@list').decode().splitlines()}
        for name in sorted(listed):
            # Shared assets stay available for every class, race, spell and reward.
            # Continent maps, unrelated creatures, world-map UI and cinematics do not.
            if (name.startswith(('dbfilesclient\\', 'fonts\\', 'spells\\', 'xtextures\\', 'environments\\stars\\')) or
                (name.startswith('interface\\') and not name.startswith(('interface\\worldmap\\', 'interface\\taxiframe\\', 'interface\\cinematics\\'))) or
                name.startswith(('sound\\ambience\\water\\', 'sound\\ambience\\weather\\'))):
                package.add(name, 'shared gameplay/UI assets')
            elif name.endswith('.ground'):
                # A custom arena's walk grid (Hall of Thanes' arena.ground); no asset references it.
                package.add(name, 'custom arena walk grid')

        gameplay = ROOT / 'crates/benilla-app/src/survivors'
        definitions = (gameplay / 'data.rs').read_text()
        definitions += '\n' + (gameplay / 'story_maps.rs').read_text()
        ids = {1825, 16587}  # XP pickup units.
        ids.update(int(n) for n in re.findall(r'\(\s*"[^"\n]+",\s*(\d+)', (gameplay / 'elwynn.rs').read_text()))
        for looks in re.findall(r'looks:\s*&\[([^]]*)\]', definitions):
            ids.update(map(int, re.findall(r'\d+', looks)))
        for race, classes in re.findall(r'Race \{ id: (\d+),.*?classes: &\[([^]]*)\]', (gameplay / 'races_data.rs').read_text()):
            for cls in map(int, re.findall(r'\d+', classes)):
                for sex in (0, 1):
                    identifier = 300000 + cls * 200 + int(race) * 2 + sex
                    if identifier in package.table('CreatureDisplayInfo').by_id:
                        ids.add(identifier)
        ids.update(int(n) for n in re.findall(r'totem:\s*\((\d+)', (gameplay / 'spell_data.rs').read_text()))
        # Every arena's foes and bosses: `foe(..)`/`giant(..)` rows and The Barrens' `en(..)`.
        ids.update(int(n) for n in re.findall(r'\b(?:foe|giant|en)\(\s*"[^"\n]+",\s*(\d+)', definitions))
        equipment = package.creatures(ids)
        # Owned drops can replace every starter slot, including converted retail gear.
        equipment.update(runtime_equipment())
        for weapons in re.findall(r'weapons:\s*&\[(.*?)\]', definitions, re.S):
            equipment.update(int(n) for n in re.findall(r'\(\s*\d+,\s*(\d+)', weapons))
        package.equipment(equipment, listed)
        objects = package.table('GameObjectDisplayInfo')
        for display in (114, 259, 2614):  # Healing food, caches, gold.
            package.add(objects.text(objects.by_id[display][1]), f'loot object {display}')
        # Morgan's landmine is spawned at runtime and has no arena placement to pull it in.
        package.add(objects.text(objects.by_id[5791][1]), 'Morgan landmine')

        for table_name in ('SpellVisualEffectName', 'SpellChainEffects', 'LightSkybox'):
            for name in package.table(table_name).strings.split(b'\0'):
                path = name.decode('utf-8')
                if path.lower().endswith(('.mdx', '.mdl', '.m2', '.blp')) and ('/' in path or '\\' in path):
                    package.add(path, table_name)

        # Every arena in ALL_MAPS, playable or locked, so an unlock never lands on an empty map.
        arenas = [(int(m), float(x), float(y)) for m, x, y in
                  re.findall(r'\bmap:\s*(\d+),.*?\bcenter:\s*\[\s*(-?[\d.]+),\s*(-?[\d.]+)', definitions, re.S)]
        if not arenas:
            raise ValueError('No arena definitions found in data.rs')
        map_table = package.table('Map')
        wanted = collections.defaultdict(set)  # map directory -> ADT (x, y) around its arenas
        for map_id, x, y in arenas:
            row = map_table.by_id.get(map_id)
            if row is None:
                package.source_missing.append(dict(map=map_id, reason='arena map absent from source Map.dbc'))
                continue
            # ADT names run x along -WoW y and y along -WoW x.
            tx, ty = int(32 - y / TILE_YARDS), int(32 - x / TILE_YARDS)
            wanted[map_table.text(row[1])].update(
                (tx + dx, ty + dy) for dx in range(-TILE_RADIUS, TILE_RADIUS + 1) for dy in range(-TILE_RADIUS, TILE_RADIUS + 1))

        overrides, tiles = {}, {}
        for directory, near in sorted(wanted.items()):
            base = f'World/Maps/{directory}/{directory}'
            wdt_path = canonical(base + '.wdt')
            original_wdt = reader.read(wdt_path, optional=True)
            if original_wdt is None:
                package.source_missing.append(dict(path=wdt_path, reason='arena map tile index'))
                continue
            # A tile index with no terrain tiles is a WMO-only map (a dungeon): its MODF/MWMO carries it whole.
            present = set()
            wdt = bytearray()
            for tag, raw in chunks(original_wdt):
                if tag == b'MAIN':
                    raw = bytearray(raw)
                    for ty in range(64):
                        for tx in range(64):
                            slot = (ty * 64 + tx) * 8
                            if (tx, ty) in near and struct.unpack_from('<I', raw, slot)[0]:
                                present.add((tx, ty))
                            else:
                                raw[slot:slot + 8] = b'\0' * 8
                wdt += struct.pack('<4sI', tag[::-1], len(raw)) + raw
            overrides[wdt_path] = bytes(wdt)
            package.add(wdt_path, f'{directory} arena tile index')
            tiles[directory] = sorted(present)
            for tx, ty in sorted(present):
                package.add(f'{base}_{tx}_{ty}.adt', f'{directory} arena and visible surroundings')
            if not present:
                continue

            # Coarse, untextured horizon heights for the renderer's five-tile radius around the
            # arenas; the rest of the continent stays out of the distant-terrain index too.
            wdl_path = canonical(base + '.wdl')
            original_wdl = reader.read(wdl_path, optional=True)
            if original_wdl is None:
                continue
            wdl_chunks = dict(chunks(original_wdl))
            horizon = {(tx + dx, ty + dy) for tx, ty in present
                       for dx in range(-HORIZON_RADIUS, HORIZON_RADIUS + 1) for dy in range(-HORIZON_RADIUS, HORIZON_RADIUS + 1)}
            maof = bytearray(64 * 64 * 4)
            wdl_header = struct.pack('<4sII', b'REVM', 4, 18)
            wdl_body = bytearray()
            for ty in range(64):
                for tx in range(64):
                    if (tx, ty) not in horizon:
                        continue
                    offset = struct.unpack_from('<I', wdl_chunks[b'MAOF'], (ty * 64 + tx) * 4)[0]
                    if offset:
                        tag, length = struct.unpack_from('<4sI', original_wdl, offset)
                        if tag != b'ERAM':
                            raise ValueError('Unexpected distant-terrain tile')
                        struct.pack_into('<I', maof, (ty * 64 + tx) * 4, len(wdl_header) + 8 + len(maof) + len(wdl_body))
                        wdl_body += original_wdl[offset:offset + 8 + length]
            overrides[wdl_path] = bytes(wdl_header + struct.pack('<4sI', b'FOAM', len(maof)) + maof + wdl_body)
            package.add(wdl_path, f'{directory} coarse horizon heights')

        # The minimap index, cut to the packed terrain tiles; entries stay under their map's header.
        translate_path = canonical('Textures/Minimap/md5translate.trs')
        translate = reader.read(translate_path).decode('ascii')
        by_dir = {directory.lower(): set(present) for directory, present in tiles.items()}
        sections = collections.defaultdict(list)
        for line in translate.splitlines():
            columns = line.split('\t')
            match = re.fullmatch(r'([^\\/]+)[\\/]map(\d+)_(\d+)\.blp', columns[0], re.I)
            if not match or (int(match[2]), int(match[3])) not in by_dir.get(match[1].lower(), ()):
                continue
            if len(columns) != 2:
                raise ValueError('Malformed outdoor minimap mapping')
            sections[match[1]].append(line)
            package.add('Textures/Minimap/' + columns[1], f'{match[1]} minimap')
        for directory, present in tiles.items():
            found = sum(len(lines) for name, lines in sections.items() if name.lower() == directory.lower())
            if found != len(present):
                print(f'Note: {directory} has minimap entries for {found} of {len(present)} packed tiles', flush=True)
        # The parser expects each continent's header before its entries.
        minimap = ''.join(f'dir: {directory}\r\n' + ''.join(line + '\r\n' for line in lines) for directory, lines in sections.items())
        overrides[translate_path] = minimap.encode('ascii')
        package.add(translate_path, 'arena-only minimap index')
        package.drain(overrides)
        package.environment()

        sounds = package.table('SoundEntries')
        for row in sounds.rows:
            directory = sounds.text(row[23]).replace('/', '\\')
            # Keep local ambience/music plus named forced/intro beds. Other zones'
            # music and speech are not part of this pack.
            label = sounds.text(row[2]).lower()
            shared = directory.lower().startswith(('sound\\spells', 'sound\\interface', 'sound\\item')) or (directory.lower().startswith('sound\\character') and row[1] not in (12, 29))
            if shared or row[0] in package.sound_ids or label in ('ghost (donotrename)', 'underwater (donotrename)') or directory.lower().startswith(('sound\\music\\zonemusic\\forest', 'sound\\music\\gluescreenmusic', 'sound\\doodad')):
                for offset in row[3:13]:
                    if file := sounds.text(offset):
                        package.add(directory.rstrip('\\') + '/' + file, 'arena/creature/environment audio')
        package.drain()
        archive.finish()
        temporary.replace(output / 'Data/patch.MPQ')
    finally:
        if not archive.file.closed:
            archive.file.close()
        reader.close()

    report = dict(source=str(source), tiles=tiles, assets=len(package.manifest),
                  raw_asset_bytes=sum(row['bytes'] for row in package.manifest),
                  archive_bytes=(output / 'Data/patch.MPQ').stat().st_size,
                  files=package.manifest, source_missing=package.source_missing,
                  legacy_models=package.legacy_models, gameplay_tested=False)
    (output / 'asset-manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    if not args.data_only:
        subprocess.run(['cargo', 'build', '--release', '-p', 'warcraft-survivors', '--no-default-features'], cwd=ROOT, check=True)
        executable = 'warcraft-survivors.exe' if os.name == 'nt' else 'warcraft-survivors'
        shutil.copy2(ROOT / 'target/release' / executable, output / executable)
        (output / 'Play.cmd').write_text('@echo off\nsetlocal\ncd /d "%~dp0"\nset "WOW_DATA=%~dp0Data"\n"%~dp0warcraft-survivors.exe"\n')
        for name in ('LICENSE-MIT', 'LICENSE-APACHE', 'CONTROLS.md'):
            shutil.copy2(ROOT / name, output / name)
        shutil.copy2(ROOT / 'docs/COOP.md', output / 'COOP.md')
        if args.include_coop_server:
            subprocess.run(['cargo', 'build', '--release', '-p', 'asr-coop'], cwd=ROOT, check=True)
            relay = 'asr-coop.exe' if os.name == 'nt' else 'asr-coop'
            shutil.copy2(ROOT / 'target/release' / relay, output / relay)
            (output / 'Host Server.cmd').write_text('@echo off\ncd /d "%~dp0"\n"%~dp0asr-coop.exe" --bind 0.0.0.0:47771\npause\n')
        (output / 'README.txt').write_text('ASR Game local player package\n\nRun Play.cmd. No Rust installation is needed to play.\nKeep Data beside the executable. This package contains assets from your own\nlocal game installation; keep game data out of the source repository.\nBuild and packaging do not establish gameplay correctness; gameplay is untested.\n')
    total = sum(path.stat().st_size for path in output.rglob('*') if path.is_file())
    print(f'Player package: {output}\n{len(package.manifest):,} assets; {report["archive_bytes"]:,} archive bytes; {total:,} total bytes', flush=True)
    if args.zip:
        zip_path = output.with_suffix('.zip')
        temporary_zip = zip_path.with_suffix('.zip.tmp')
        with zipfile.ZipFile(temporary_zip, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as zipped:
            for path in sorted(output.rglob('*')):
                if path.is_file():
                    zipped.write(path, path.relative_to(output.parent))
        temporary_zip.replace(zip_path)
        print(f'ZIP: {zip_path} ({zip_path.stat().st_size:,} bytes)', flush=True)


if __name__ == '__main__':
    build()
