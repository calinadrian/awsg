"""Conversion math tests; no installed game data required."""
import struct
import unittest

from import_retail_elwynn import Importer, convert_layers, decode_alpha, encode_alpha, liquid_block


class ConversionTests(unittest.TestCase):
    def test_empty_retail_ground_slots_do_not_resolve_to_vanilla_flower_zero(self):
        importer = Importer.__new__(Importer)
        importer.effects, importer.detail_models, importer.absent_effects = {}, {}, set()
        importer.effect_rows = {1: [0, 0, [0, 0, 0, 0]]}
        self.assertEqual(importer.effect(999), 0xffffffff)
        self.assertEqual(importer.effect(0), 0xffffffff)
        importer.effect(1)
        self.assertEqual(importer.effects[1][1:5], [0xffffffff] * 4)

    def test_retail_ground_effect_ids_are_remapped_without_discarding_them(self):
        parts = {b'MCLY': struct.pack('<4I', 0, 0, 0, 507)}
        layers, _ = convert_layers(parts, lambda effect: 1000000 + effect)
        self.assertEqual(struct.unpack('<4I', layers)[3], 1000507)

    def test_alpha_preserves_retail_weights_in_sequential_blender(self):
        # 10% base and 20/30/40% upper layers must survive sequential splatting.
        weights = [51, 76, 102]
        rows = [(0, 0, 0, 9)] + [(i + 1, 256, i * 4096, 9) for i in range(3)]
        parts = {b'MCLY': b''.join(struct.pack('<4I', *r) for r in rows),
                 b'MCAL': b''.join(bytes([w]) * 4096 for w in weights)}
        layers, alpha = convert_layers(parts)
        decoded = []
        for _, flags, pos, effect in list(struct.iter_unpack('<4I', layers))[1:]:
            self.assertEqual(effect, 0)
            decoded.append(decode_alpha(alpha, pos, bool(flags & 512))[0] / 255)
        a, b, c = decoded
        actual = [(1 - a) * (1 - b) * (1 - c), a * (1 - b) * (1 - c), b * (1 - c), c]
        for got, want in zip(actual, [26 / 255] + [w / 255 for w in weights], strict=True):
            self.assertAlmostEqual(got, want, delta=0.005)

    def test_opaque_top_layer_does_not_divide_by_zero(self):
        parts = {b'MCLY': b''.join(struct.pack('<4I', i, 256, max(0, i - 1) * 4096, 0)
                                  for i in range(4)),
                 b'MCAL': bytes(8192) + bytes([255]) * 4096}
        layers, alpha = convert_layers(parts)
        row = list(struct.iter_unpack('<4I', layers))[-1]
        self.assertEqual(decode_alpha(alpha, row[2], True), bytes([255]) * 4096)

    def test_alpha_literal_rle_round_trip(self):
        data = bytes(range(256)) * 16
        self.assertEqual(decode_alpha(encode_alpha(data), 0, True), data)

    def test_partial_water_preserves_mask_and_vertex_heights(self):
        # A 2x1 surface with only its first square wet, inset at cell (3,4).
        instance, mask, vertices = 3072, 3096, 3097
        blob = bytearray(vertices + 6 * 5)
        struct.pack_into('<3I', blob, 0, instance, 1, 0)
        struct.pack_into('<HHff4BII', blob, instance, 5, 0, 12, 17, 3, 4, 2, 1, mask, vertices)
        blob[mask] = 1
        struct.pack_into('<6f', blob, vertices, *range(12, 18))
        blob[vertices + 24:] = bytes([42]) * 6
        out = liquid_block(blob, 0, {})
        self.assertEqual(len(out), 804)
        self.assertEqual(out[656 + 4 * 8 + 3], 0)
        self.assertEqual(out[656 + 4 * 8 + 4], 15)
        self.assertEqual(struct.unpack_from('<If', out, 8 + 8 * (4 * 9 + 3)), (42, 12))
        self.assertEqual(struct.unpack_from('<f', out, 12 + 8 * (5 * 9 + 5))[0], 17)

    def test_missing_water_is_empty(self):
        self.assertEqual(liquid_block(None, 0, {}), b'')


if __name__ == '__main__':
    unittest.main()
