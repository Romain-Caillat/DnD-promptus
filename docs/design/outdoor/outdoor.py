# Outdoor pixel map renderer prototype: terrain blending, 3/4 buildings and cliffs, props, light.
import math, random, zlib, struct, sys
T = 16

def h2(x, y, s):
    n = (x * 374761393 + y * 668265263 + s * 2147483647) & 0xffffffff
    n = (n ^ (n >> 13)) * 1274126177 & 0xffffffff
    return ((n ^ (n >> 16)) & 0xffff) / 65535.0
def vnoise(x, y, s):
    xi, yi = math.floor(x), math.floor(y); xf, yf = x - xi, y - yi
    u, v = xf * xf * (3 - 2 * xf), yf * yf * (3 - 2 * yf)
    a, b = h2(xi, yi, s), h2(xi + 1, yi, s); c, d = h2(xi, yi + 1, s), h2(xi + 1, yi + 1, s)
    return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
def fbm(x, y, s, o=4):
    t, a, f, n = 0, .5, 1, 0
    for i in range(o): t += a * vnoise(x * f, y * f, s + i * 17); n += a; a *= .5; f *= 2
    return t / n
def clamp(v): return max(0, min(255, int(v)))
def mul(c, k): return tuple(clamp(v * k) for v in c)
def add(c, k): return tuple(clamp(v + k) for v in c)
def mix(a, b, t): return tuple(clamp(a[i] + (b[i] - a[i]) * t) for i in range(3))

class Img:
    def __init__(s, w, h, pad):
        s.w, s.h, s.pad = w, h, pad
        s.p = [[(0, 0, 0)] * w for _ in range(h)]
    def get(s, x, y): return s.p[y][x] if 0 <= x < s.w and 0 <= y < s.h else None
    def put(s, x, y, c):
        if 0 <= x < s.w and 0 <= y < s.h: s.p[y][x] = c
    def blend(s, x, y, c, a):
        if 0 <= x < s.w and 0 <= y < s.h: s.p[y][x] = mix(s.p[y][x], c, a)
    def shade(s, x, y, k):
        if 0 <= x < s.w and 0 <= y < s.h: s.p[y][x] = mul(s.p[y][x], k)
    def rect(s, x0, y0, w, h, c):
        for y in range(y0, y0 + h):
            for x in range(x0, x0 + w): s.put(x, y, c)
    def ell(s, cx, cy, rx, ry, fn):
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                d = ((x + .5 - cx) / rx) ** 2 + ((y + .5 - cy) / ry) ** 2
                if d <= 1: fn(x, y, d)
    def shadow_ell(s, cx, cy, rx, ry, k=.55):
        s.ell(cx, cy, rx, ry, lambda x, y, d: s.shade(x, y, k + (1 - k) * d * d))
    def save(s, path, scale=1):
        rows = []
        for r in s.p:
            row = b''.join(bytes(c) * scale for c in r)
            rows += [row] * scale
        raw = b''.join(b'\x00' + r for r in rows)
        def ch(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
        open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + ch(b'IHDR', struct.pack('>IIBBBBB', s.w * scale, s.h * scale, 8, 2, 0, 0, 0)) + ch(b'IDAT', zlib.compress(raw, 9)) + ch(b'IEND', b''))

def sprite(img, x0, y0, rows, pal, outline=(14, 12, 12)):
    """Paint a small pixel prop with an automatic dark outline."""
    H, W = len(rows), len(rows[0])
    at = lambda x, y: 0 <= y < H and 0 <= x < W and rows[y][x] != '.'
    for y in range(-1, H + 1):
        for x in range(-1, W + 1):
            if at(x, y): img.put(x0 + x, y0 + y, pal[rows[y][x]])
            elif at(x + 1, y) or at(x - 1, y) or at(x, y + 1) or at(x, y - 1): img.put(x0 + x, y0 + y, outline)

def ground(img, grid, mats, warp_set, seed):
    R, C = len(grid), len(grid[0])
    for py in range(img.pad, img.h):
        for px in range(img.w):
            wx, wy = px, py - img.pad
            cx, cy = wx / T, wy / T
            ch = grid[min(R - 1, int(cy))][min(C - 1, int(cx))]
            if ch in warp_set:
                ox = (fbm(wx / 9, wy / 9, seed) - .5) * 26
                oy = (fbm(wx / 9, wy / 9, seed + 9) - .5) * 26
                cx2, cy2 = (wx + ox) / T, (wy + oy) / T
                ch2 = grid[max(0, min(R - 1, int(cy2)))][max(0, min(C - 1, int(cx2)))]
                if ch2 in warp_set: ch = ch2
            img.p[py][px] = mats[ch](wx, wy)

def cliff_and_walls(img, grid, solid, cap_fn, face_fn, face_h, seed, beyond=""):
    """Three-quarter view: cap drawn face_h px higher, face fills down to the cell's bottom edge.
    Cells outside the map count as solid when their edge is listed in `beyond` (n, s)."""
    R, C = len(grid), len(grid[0])
    def sol(x, y):
        if y < 0: return 'n' in beyond and 0 <= x < C and grid[0][x] in solid
        if y >= R: return 's' in beyond and 0 <= x < C and grid[R - 1][x] in solid
        return 0 <= x < C and grid[y][x] in solid
    for y in range(-3, 0):
        for x in range(C):
            if not sol(x, y): continue
            top = img.pad + y * T - face_h
            for j in range(T):
                for i in range(T): img.put(x * T + i, top + j, cap_fn(x * T + i, (y + 3) * T + j, grid[0][x]))
    for y in range(R):
        for x in range(C):
            if not sol(x, y): continue
            top = img.pad + y * T - face_h
            for j in range(T):
                for i in range(T):
                    c = cap_fn(x * T + i, y * T + j, grid[y][x])
                    if j < 2 and not sol(x, y - 1): c = add(c, 34)
                    if i < 1 and not sol(x - 1, y): c = add(c, 18)
                    if i >= T - 1 and not sol(x + 1, y): c = mul(c, .8)
                    img.put(x * T + i, top + j, c)
            if not sol(x, y + 1):
                fy = top + T
                for j in range(face_h):
                    for i in range(T):
                        img.put(x * T + i, fy + j, face_fn(x * T + i, j, face_h, grid[y][x]))
                for j in range(5):
                    for i in range(T): img.shade(x * T + i, fy + face_h + j, .55 + .09 * j)

def light(img, cx, cy, r, col, a):
    for y in range(int(cy - r), int(cy + r)):
        for x in range(int(cx - r), int(cx + r)):
            d = math.hypot(x - cx, y - cy) / r
            if d < 1:
                c = img.get(x, y)
                if c: img.p[y][x] = tuple(clamp(c[i] + col[i] * a * (1 - d) ** 2) for i in range(3))

def grade(img, tint, sat, vig):
    cx, cy = img.w / 2, img.h / 2
    for y in range(img.h):
        for x in range(img.w):
            r, g, b = img.p[y][x]
            l = .3 * r + .59 * g + .11 * b
            r, g, b = l + (r - l) * sat, l + (g - l) * sat, l + (b - l) * sat
            d = math.hypot((x - cx) / cx, (y - cy) / cy)
            k = 1 - vig * max(0, d - .55) ** 1.6
            img.p[y][x] = (clamp(r * tint[0] * k), clamp(g * tint[1] * k), clamp(b * tint[2] * k))
