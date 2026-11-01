"""Shadow packing tests, without installed game assets."""
import struct
import unittest

from bake_elwynn_shadows import add_shadows
from import_retail_elwynn import chunk, chunks


class ShadowTests(unittest.TestCase):
    def test_maps_are_addressable_and_rebaking_does_not_grow_cells(self):
        header = bytearray(128)
        body = chunk(b'MCVT', b'heights unchanged')
        adt = chunk(b'MVER', struct.pack('<I', 18)) + chunk(b'MCNK', header + body) * 256
        masks = bytes([0x81]) * (256 * 512)
        baked = add_shadows(adt, masks)
        cells = [data for tag, data in chunks(baked) if tag == b'MCNK']
        for cell in cells:
            offset, size = struct.unpack_from('<2I', cell, 44)
            self.assertEqual(size, 512)
            self.assertEqual(cell[offset:offset + size], bytes([0x81]) * 512)
            self.assertEqual(cell[128:128 + len(body)], body)
            self.assertEqual(struct.unpack_from('<I', cell)[0] & 1, 1)
        self.assertEqual(add_shadows(baked, masks), baked)

    def test_truncated_masks_and_incomplete_tiles_are_rejected(self):
        with self.assertRaises(ValueError):
            add_shadows(b'', b'')
        with self.assertRaises(ValueError):
            add_shadows(chunk(b'MCNK', bytes(128)), bytes(256 * 512))


if __name__ == '__main__':
    unittest.main()
