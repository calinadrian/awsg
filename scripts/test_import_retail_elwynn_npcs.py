"""Dependency and roster checks for the retail NPC staging tool."""

from pathlib import Path
import re
import struct
import tempfile
import unittest

from import_retail_elwynn_npcs import ROSTER, extract, references


def chunk(tag, body):
    return tag + struct.pack('<I', len(body)) + body


class NpcExtractionTests(unittest.TestCase):
    def test_roster_matches_game_including_shared_displays(self):
        source = (Path(__file__).resolve().parents[1] /
                  'crates/benilla-app/src/survivors/elwynn.rs').read_text(encoding='utf-8')
        actual = re.findall(r'\(\s*"([^"]+)"\s*,\s*(\d+)\s*,', source)
        self.assertCountEqual([(name, int(display)) for name, display in actual],
                              [(name, display) for name, display, _ in ROSTER])

    def test_model_and_skeleton_dependency_formats(self):
        data = chunk(b'MD21', b'')
        data += chunk(b'SFID', struct.pack('<3I', 10, 20, 0))
        data += chunk(b'TXID', struct.pack('<I', 30))
        data += chunk(b'SKID', struct.pack('<I', 40))
        data += chunk(b'AFID', struct.pack('<HHI', 1, 2, 50))
        self.assertEqual(references(data), {10, 20, 30, 40, 50})
        skeleton = chunk(b'SKL1', b'') + chunk(b'SKPD', struct.pack('<4I', 0, 0, 60, 0))
        skeleton += chunk(b'BFID', struct.pack('<I', 70))
        self.assertEqual(references(skeleton), {60, 70})

    def test_truncated_and_invalid_references_fail(self):
        for data in (b'MD21', b'MD21' + struct.pack('<I', 8),
                     chunk(b'MD21', b'') + chunk(b'AFID', bytes(7)),
                     chunk(b'SKL1', b'') + chunk(b'SKPD', bytes(4))):
            with self.subTest(data=data), self.assertRaises(ValueError):
                references(data)

    def test_recursive_dependencies_and_replaceable_skins_are_staged_once(self):
        model_ids = {fd for _, _, ids in ROSTER for fd in ids}
        names = {fd: f'creature/family/{fd}.m2' for fd in model_ids}
        names[999] = 'creature/family/skin.blp'
        names[998] = 'creature/other/unrelated.blp'
        blobs = {fd: chunk(b'MD21', b'') + chunk(b'SKID', struct.pack('<I', 997))
                 for fd in model_ids}
        blobs[997] = chunk(b'SKL1', b'') + chunk(b'AFID', struct.pack('<HHI', 0, 0, 996))
        blobs[996], blobs[999] = b'animation', b'BLP2skin'

        class Storage:
            def read(self, fd):
                return blobs.get(fd)

        with tempfile.TemporaryDirectory() as directory:
            files = extract(Storage(), names, Path(directory))
            self.assertEqual(set(files), model_ids | {997, 996, 999})
            self.assertEqual((Path(directory) / 'files/996').read_bytes(), b'animation')
            del blobs[996]
            with self.assertRaises(FileNotFoundError):
                extract(Storage(), names, Path(directory))


if __name__ == '__main__':
    unittest.main()
