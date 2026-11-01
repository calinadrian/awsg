"""Restore inherited retail skeleton animations in the local converted models.

python scripts/repair_race_animations.py D:/Projects/ASR "D:/Personal/Battle.net/World of Warcraft"
Only updates the gitignored local overlay patch-Y.MPQ. Source archives remain read-only.
"""
import copy
import json
from pathlib import Path
import struct
import sys


def inherited_converter(base, parent_bytes):
    """Keep child pivots/rig and animation overrides; inherit missing parent clips."""
    import retail_m2 as m2

    class Inherited(base.__class__):
        def pick_sequences(self):
            m2.Converter.pick_sequences(self)
            child = self.seqs
            child_globals = self.gloops
            child_buf = self.seq_buf
            self.seq_buf = m2.chunks(parent_bytes)['SKS1']
            m2.Converter.pick_sequences(self)
            parent = self.seqs
            self.seq_buf = child_buf
            self.gloops = child_globals
            parents = {(s.id, s.variation): s for s in parent}
            for s in child:
                s.inherited = False
                s.parent_data = parents.get((s.id, s.variation))
            overrides = {(s.id, s.variation) for s in child}
            for s in parent:
                s.inherited = True
                s.parent_data = s
            self.seqs = sorted(child + [s for s in parent if (s.id, s.variation) not in overrides],
                               key=lambda s: (s.id, s.variation))
            cursor = 0
            for s in self.seqs:
                s.start, s.end = cursor, cursor + s.duration
                cursor = s.end + 1
            self.report['sequences'] = len(self.seqs)

        def track(self, buf, off, size, value=bytes, identity=None):
            if struct.unpack_from('<h', buf, off + 2)[0] >= 0:
                return m2.Converter.track(self, buf, off, size, value, identity)
            all_sequences = self.seqs
            times, values, ranges = [], [], []
            interp = struct.unpack_from('<H', buf, off)[0]
            try:
                for s in all_sequences:
                    selected = s
                    use_parent = s.inherited
                    while True:
                        source, at = buf, off
                        if use_parent:
                            _, child_at = m2.arr(self.bones_buf, 0)
                            _, parent_at = m2.arr(self.parent_bones, 0)
                            source, at = self.parent_bones, parent_at + off - child_at
                        self.seqs = [selected]
                        track = m2.Converter.track(self, source, at, size, value, identity)
                        if track[3] or use_parent or s.parent_data is None:
                            break
                        selected = copy.copy(s)
                        selected.data = s.parent_data.data
                        use_parent = True
                    ts, vs = track[3:]
                    if track[1] >= 0:
                        globals_at = m2.arr(m2.chunks(parent_bytes)['SKS1'], 0)[1]
                        period = struct.unpack_from('<I', m2.chunks(parent_bytes)['SKS1'], globals_at+track[1]*4)[0]
                        if period <= 0:
                            raise ValueError('Invalid inherited global sequence period')
                        keys = [(s.start+cycle+t, v) for cycle in range(0, s.duration+1, period)
                                for t, v in zip(ts, vs) if cycle+t <= s.duration]
                        ts, vs = ([k[0] for k in keys], [k[1] for k in keys])
                    interp = max(interp, track[0])
                    if not ts:
                        ts = [s.start]
                        vs = [identity if identity is not None else m2.IDENTITY.get(size, bytes(size))]
                    lo = len(times)
                    times.extend(ts)
                    values.extend(vs)
                    ranges.append((lo, len(times) - 1))
                return interp, -1, ranges, times, values
            finally:
                self.seqs = all_sequences

    base.__class__ = Inherited
    base.parent_bones = m2.chunks(parent_bytes)['SKB1']
    cn, co = m2.arr(base.bones_buf, 0)
    pn, po = m2.arr(base.parent_bones, 0)
    if cn != pn or any(base.bones_buf[co+i*88+8:co+i*88+10] !=
                       base.parent_bones[po+i*88+8:po+i*88+10] for i in range(cn)):
        raise ValueError('Parent skeleton hierarchy differs from the child')
    return base


def graft(body, converter):
    import retail_m2 as m2
    converter.pick_sequences()
    w = m2.Writer()
    w.data = bytearray(body)
    w.array(0x14, len(converter.gloops)//4, converter.gloops)
    by_id = {}
    for i, s in enumerate(converter.seqs):
        by_id.setdefault(s.id, []).append(i)
    blob = bytearray()
    for i, s in enumerate(converter.seqs):
        chain = by_id[s.id]
        nxt = chain[chain.index(i)+1] if chain.index(i)+1 < len(chain) else -1
        blob += struct.pack('<HHIIfIhHIII', s.id, chain.index(i), s.start, s.end, s.speed,
                            s.flags & ~0x40 | 0x20, s.frequency, 0, s.replay_min, s.replay_max, s.blend)
        blob += s.bounds + struct.pack('<hH', nxt, i)
    w.array(0x1c, len(converter.seqs), blob)
    top = max(by_id)+1
    w.array(0x24, top, struct.pack(f'<{top}H', *(by_id[i][0] if i in by_id else 65535 for i in range(top))))
    play = [(f if f in by_id else 0, flags if f in by_id else 0) for f, flags in converter.playable]
    w.array(0x2c, len(play), b''.join(struct.pack('<hh', *p) for p in play))
    n, at = m2.arr(body, 0x34)
    count, src = m2.arr(converter.bones_buf, 0)
    if count != n:
        raise ValueError('Converted body lost skeleton bones')
    for i in range(n):
        rec, source = at + i*108, src + i*88
        for field, old_field, size, value, identity in (
            (12, 16, 12, bytes, None),
            (40, 36, 16, m2.comp_quat, struct.pack('<4f', 0, 0, 0, 1)),
            (68, 56, 12, bytes, struct.pack('<3f', 1, 1, 1)),
        ):
            converter.write_track(w, rec, field, converter.track(converter.bones_buf, source+old_field, size, value, identity))
    return bytes(w.data)


def main():
    source = Path(sys.argv[1]).resolve()
    root = Path(__file__).resolve().parents[1]
    sys.path.insert(0, str(source/'launcher'))
    import casc
    import retail_m2 as m2
    from bloodelf_assets import vanilla_animations
    from durotar_assets import Chain
    from photo_archive import build, read_stored
    metadata = json.loads((source/'launcher/native-races.json').read_text())
    manifest = json.loads((source/'downloads/race-expansion/patch-W.json').read_text())
    storage = casc.Casc(sys.argv[2])
    game = Chain(root/'wow-data/Data', exclude=('patch-Y.MPQ',))
    entries, report = {}, {}
    try:
        for race, info in metadata.items():
            for sex in (0, 1):
                label = ('Male', 'Female')[sex]
                model = storage.read(info['sexes'][str(sex)]['model'])
                chunks = m2.chunks(model) if model[:4] != b'MD20' else {}
                if 'SKID' not in chunks:
                    continue
                skeleton = storage.read(struct.unpack('<I', chunks['SKID'])[0])
                sk = m2.chunks(skeleton)
                if 'SKPD' not in sk:
                    continue
                parent_id = struct.unpack_from('<I', sk['SKPD'], 8)[0]
                if not parent_id:
                    continue
                name = info['folder']+label
                record = manifest['characters'][name]
                keep, playable = vanilla_animations(game, label)
                converter = m2.Converter(model, skeleton, storage.read(record['skin']), name, keep, {}, playable)
                converter = inherited_converter(converter, storage.read(parent_id))
                body = game.read(record['path'])
                fixed = graft(body, converter)
                for header, stride in ((0x44, 48), (0x4c, 44), (0x5c, 16), (0x104, 48), (0x10c, 2)):
                    count, offset = m2.arr(body, header)
                    if m2.arr(fixed, header) != (count, offset) or body[offset:offset+count*stride] != fixed[offset:offset+count*stride]:
                        raise ValueError(f'{name}: animation repair changed array {header:#x}')
                for aid in (0, 4, 5):
                    if not any(s.id == aid for s in converter.seqs):
                        raise ValueError(f'{name}: required animation {aid} absent')
                entries[record['path']] = fixed
                report[name] = dict(parent=parent_id, before=m2.arr(body, 0x1c)[0], after=len(converter.seqs))
                print(name, report[name])
        overlay = root/'wow-data/Data/patch-Y.MPQ'
        files = read_stored(overlay.read_bytes())
        existing = dict(zip(files[0].decode('ascii').splitlines(), files[1:], strict=True))
        existing.update(entries)
        temporary = overlay.with_suffix('.MPQ.tmp')
        temporary.write_bytes(build(existing))
        temporary.replace(overlay)
        (root/'wow-data/race-animation-repair.json').write_text(json.dumps(report, indent=2))
    finally:
        game.close()


if __name__ == '__main__':
    main()
