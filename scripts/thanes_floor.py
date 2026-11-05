"""Derive a connected arena floor and safe spawn cells from the imported WMO.

World placement is (-local_x, -local_y, local_z + 500). Follow a continuous
walkable layer near the first burial hall, rejecting roofs, walls and prop bounds.
Store the floor in the asset archive rather than embedding derived data in Git."""
import math
import struct
from collections import deque
X0, Y0, CELL, W, H = (0.0, -60.0, 1.0, 110, 120)
START = (54.0, 0.0, 500.0)
RADIUS = 32.0

def build_floor(importer):
    import deadmines_ground as ground
    from import_retail_elwynn import chunks
    ground.X0, ground.Y0, ground.CELL, ground.W, ground.H = (X0, Y0, CELL, W, H)
    ground.START, ground.ARENA_RADIUS = (START, RADIUS)
    up, every, obstacles = ({}, {}, [])
    root = importer.entries['World\\ASRForeverThanes\\7732775_002.wmo']
    parts = dict(chunks(dict(chunks(root))[b'MOGP'][68:]))
    vertices = [(-x, -y, z + 500) for x, y, z in struct.iter_unpack('<3f', parts[b'MOVT'])]
    for indices in struct.iter_unpack('<3H', parts[b'MOVI']):
        a, b, c = (vertices[i] for i in indices)
        u = [b[k] - a[k] for k in range(3)]
        v = [c[k] - a[k] for k in range(3)]
        nx, ny, nz = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        floor = nz / (math.sqrt(nx * nx + ny * ny + nz * nz) or 1) > 0.7
        if not floor:
            obstacles.append((a, b, c))
        if abs(nz) < 1e-08:
            continue
        lo = [int((min((q[k] for q in (a, b, c))) - (X0, Y0)[k]) // CELL) for k in range(2)]
        hi = [int((max((q[k] for q in (a, b, c))) - (X0, Y0)[k]) // CELL) for k in range(2)]
        for gx in range(max(0, lo[0]), min(W - 1, hi[0]) + 1):
            for gy in range(max(0, lo[1]), min(H - 1, hi[1]) + 1):
                x, y = (X0 + (gx + 0.5) * CELL, Y0 + (gy + 0.5) * CELL)
                p = ((x - a[0]) * v[1] - (y - a[1]) * v[0]) / nz
                q = (u[0] * (y - a[1]) - u[1] * (x - a[0])) / nz
                if p < -1e-06 or q < -1e-06 or p + q > 1.000001:
                    continue
                z = a[2] + p * u[2] + q * v[2]
                i = gx * H + gy
                every.setdefault(i, []).append(z)
                if floor and 495 <= z <= 505:
                    up.setdefault(i, []).append(z)
    candidates = {}
    for i, layers in up.items():
        for z in sorted(set((round(z, 2) for z in layers))):
            head = min((a - z for a in every[i] if a > z + 2), default=999)
            if head >= 3:
                candidates.setdefault(i, []).append((z, head))
    heights, headroom = ground.connected_floors(candidates, start=START)
    blocked = ground.blocked_spawns(heights, obstacles)
    converted = dict(chunks(importer.entries['World\\ASRForeverThanes\\7732775.wmo']))
    model_names = converted[b'MODN']
    bounds = {}
    for offset in range(0, len(converted[b'MODD']), 40):
        row = converted[b'MODD'][offset:offset + 40]
        packed, px, py, pz, qx, qy, qz, qw, scale = struct.unpack_from('<I8f', row)
        name = model_names[packed & 16777215:].split(b'\x00')[0].decode('ascii')
        if name not in bounds:
            model = importer.entries[name]
            count, at = struct.unpack_from('<II', model, 68)
            pts = [struct.unpack_from('<3f', model, at + 48 * i) for i in range(count)]
            bounds[name] = [(min((p[k] for p in pts)), max((p[k] for p in pts))) for k in range(3)] if pts else None
        box = bounds[name]
        if box is None:
            continue
        pts = []
        for x in box[0]:
            for y in box[1]:
                for z in box[2]:
                    x, y, z = (x * scale, y * scale, z * scale)
                    tx, ty, tz = (2 * (qy * z - qz * y), 2 * (qz * x - qx * z), 2 * (qx * y - qy * x))
                    rx = x + qw * tx + qy * tz - qz * ty
                    ry = y + qw * ty + qz * tx - qx * tz
                    rz = z + qw * tz + qx * ty - qy * tx
                    pts.append((-px - rx, -py - ry, pz + rz + 500))
        low = [min((p[k] for p in pts)) for k in range(3)]
        high = [max((p[k] for p in pts)) for k in range(3)]
        if high[2] < 500.5 or low[2] > 508:
            continue
        for gx in range(max(0, int(low[0] - 2)), min(W, int(high[0] + 3))):
            for gy in range(max(0, int(low[1] - Y0 - 2)), min(H, int(high[1] - Y0 + 3))):
                blocked.add(gx * H + gy)
    safe = []
    for gx in range(1, W - 1):
        for gy in range(1, H - 1):
            i = gx * H + gy
            z = heights[i]
            if i in blocked or not math.isfinite(z) or headroom[i] < 8:
                continue
            if math.hypot(X0 + gx + 0.5 - START[0], Y0 + gy + 0.5 - START[1]) > RADIUS - 3:
                continue
            if all((math.isfinite(heights[(gx + dx) * H + gy + dy]) and abs(heights[(gx + dx) * H + gy + dy] - z) <= 1 for dx in (-1, 0, 1) for dy in (-1, 0, 1))):
                safe.append(i)
    if not safe:
        raise ValueError('Hall of Thanes has no safe floor cells')
    targets = [(65535, 65535)] * (W * H)
    queue = deque(safe)
    for i in safe:
        targets[i] = divmod(i, H)
    while queue:
        i = queue.popleft()
        x, y = divmod(i, H)
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            xx, yy = (x + dx, y + dy)
            if 0 <= xx < W and 0 <= yy < H and (targets[xx * H + yy][0] == 65535):
                targets[xx * H + yy] = targets[i]
                queue.append(xx * H + yy)
    seed = int((START[0] - X0) // CELL) * H + int((START[1] - Y0) // CELL)
    tx, ty = targets[seed]
    spawn = [X0 + tx + 0.5, Y0 + ty + 0.5, heights[tx * H + ty]]
    raw = struct.pack(f'<{W * H}f', *heights) + struct.pack(f'<{W * H * 2}H', *(v for t in targets for v in t))
    return (raw, dict(safe_cells=len(safe), floor_cells=sum((math.isfinite(z) for z in heights)), start=spawn))
