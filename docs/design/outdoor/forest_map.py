from outdoor import *
random.seed(23)
G = [
 "sssssssssssssssssssbwwbsssssss",
 "ssskkkkkkkssssssssbwwwbsssssss",
 "ssSSSkkSSSssssssssbwwbssssssss",
 "ssSkkkkkkSsmmmsssbwwwbssssssss",
 "ssSkkkkkkkssmmmssbwwbsssssssss",
 "ssSSkkkSSSsssmmsbwwwbsssssssss",
 "ssssskssssssssssbwwbssssssssss",
 "sssssppsssssssssbwwbssssssssss",
 "ssssssppssssssssbwwbssssssssss",
 "pppppppppppppppppwwppppppppppp",
 "ssssssssssssssssbwwbsssppsssss",
 "ssssmmsssssssssbwwbsssspppssss",
 "sssmmmmssssssssbwwbssssppppsss",
 "ssssmmsssssssssbwwwbssspppssss",
 "sssssssssssssssbwwwbssssssssss",
 "ssssssssssssssbwwwbsssssssssss",
 "ssssssssssssssbwwbssssssssssss",
 "sssssssssssssbwwwbssssssssssss",
]
R, C = len(G), len(G[0])
PAD = 3 * T
img = Img(C * T, R * T + PAD, PAD)
def grass(x, y):
    t = fbm(x / 8, y / 8, 1)
    c = mix((56, 94, 42), (96, 132, 58), t)
    r = h2(x, y, 2)
    if r > .84: c = add(c, 22)
    elif r < .1: c = mul(c, .72)
    if r > .996: c = random.choice([(236, 232, 210), (240, 206, 90), (170, 140, 220)])
    return c
def moss(x, y): return mul(grass(x, y), .74)
def path(x, y):
    n = fbm(x / 5, y / 5, 3)
    c = (110 + n * 26, 88 + n * 20, 60 + n * 14)
    if h2(x, y, 4) > .95: c = add(c, 34)
    if fbm(x / 3, y / 12, 5) > .66: c = mul(c, .82)
    return c
def bank(x, y):
    n = fbm(x / 4, y / 4, 6)
    c = (78 + n * 20, 68 + n * 16, 50 + n * 12)
    if h2(x, y, 7) > .9: c = (120, 116, 106)
    return c
def water(x, y):
    n = fbm(x / 10, y / 6, 8)
    c = mix((30, 66, 90), (54, 108, 132), n)
    if math.sin(y * .9 + x * .2 + n * 8) > .93: c = add(c, 40)
    return c
def flag(x, y):
    c = (124, 122, 112)
    if (x + (y // 8) * 5) % 11 == 0 or y % 8 == 0: c = (78, 76, 70)
    c = mul(c, .9 + .2 * fbm(x / 4, y / 4, 9))
    if fbm(x / 7, y / 7, 10) > .64: c = mix(c, (70, 104, 52), .6)
    return c
mats = {'s': grass, 'm': moss, 'p': path, 'b': bank, 'w': water, 'k': flag, 'S': flag}
ground(img, G, mats, set('smpbw'), 13)
for py in range(PAD):
    for px in range(img.w): img.p[py][px] = mats[G[0][px // T]](px, py - PAD)

def cap(x, y, ch):
    c = mul((132, 128, 118), .85 + .3 * fbm(x / 4, y / 4, 20))
    if fbm(x / 6, y / 6, 21) > .55: c = mix(c, (74, 112, 54), .7)
    return c
def face(x, j, H, ch):
    i = x % 16
    c = (112, 108, 98) if (j // 5) % 2 == 0 else (98, 94, 86)
    if j % 5 == 0 or (i + (j // 5) * 6) % 12 == 0: c = (60, 58, 54)
    if fbm(x / 5, j / 5, 22) > .62: c = mix(c, (66, 100, 50), .6)
    return mul(c, 1 - .35 * j / H)
cliff_and_walls(img, G, set('S'), cap, face, 20, 30)

# altar with a glowing rune inside the ruin
ax, ay = 6 * T, 3 * T + PAD + 4
img.shadow_ell(ax + 14, ay + 18, 18, 4, .55)
img.rect(ax, ay, 26, 12, (150, 146, 136)); img.rect(ax, ay + 12, 26, 6, (96, 92, 86)); img.rect(ax, ay, 26, 1, (190, 186, 176))
for k in range(8): img.put(ax + 9 + k, ay + 5 + (k % 3 == 0), (150, 230, 255))
light(img, ax + 13, ay + 6, 40, (120, 200, 255), .45)

# wooden bridge over the stream on the path
bx0, by0 = 16 * T + 8, 9 * T + PAD - 4
[img.shade(bx0 + 4 + xx, by0 + 22 + yy, .5) for xx in range(44) for yy in range(4)]
for x in range(44):
    for y in range(22):
        c = (128, 92, 56) if x % 5 else (72, 50, 30)
        if y in (0, 21): c = (88, 60, 36)
        img.put(bx0 + x, by0 + y, mul(c, .9 + .2 * h2(x, y, 30)))
# stepping stones and rocks in the water
for (cx, cy) in [(18 * T + 6, 14 * T + 4), (17 * T + 2, 15 * T + 10), (19 * T + 10, 2 * T + 6)]:
    cy += PAD; img.shadow_ell(cx + 3, cy + 3, 7, 3, .6)
    img.ell(cx, cy, 6, 4, lambda x, y, d: img.put(x, y, (20, 20, 20) if d > .8 else mul((140, 138, 130), 1.2 - .5 * d)))

def bush(cx, cy, r):
    img.shadow_ell(cx + 4, cy + r * .6, r + 2, r * .4, .55)
    for k in range(5):
        ox, oy = random.randint(-r // 2, r // 2), random.randint(-r // 3, r // 4)
        img.ell(cx + ox, cy + oy, r * .7, r * .55, lambda x, y, d: img.put(x, y, (16, 30, 14) if d > .86 else mix((40, 78, 34), (88, 132, 56), (1 - d) * .7 + .3 * fbm(x / 2, y / 2, 40))))
for b in [(13 * T + 8, 13 * T + 4, 14), (12 * T, 14 * T, 10), (4 * T, 8 * T, 11), (22 * T, 6 * T, 12), (28 * T, 15 * T, 13), (10 * T, 1 * T, 10)]:
    bush(b[0], b[1] + PAD, b[2])
# fallen log
lx, ly = 7 * T, 12 * T + PAD
img.shadow_ell(lx + 26, ly + 10, 30, 4, .55)
for x in range(52):
    for y in range(10):
        c = (110, 76, 46) if y < 3 else (84, 58, 34) if y < 8 else (52, 36, 22)
        if x < 3: c = (170, 140, 96) if (x + y) % 3 else (120, 96, 64)
        img.put(lx + x, ly + y, c)
# mushroom ring
for k in range(9):
    a = k / 9 * 6.28; mx = int(4 * T + 8 + math.cos(a) * 18); my = int(15 * T + PAD + math.sin(a) * 9)
    img.rect(mx, my, 2, 3, (230, 222, 200)); img.rect(mx - 1, my - 2, 4, 2, (196, 60, 48)); img.put(mx, my - 2, (250, 240, 230))
# goblin camp: two tents and a fire
def tent(x0, y0):
    [img.shade(x0 + 6 + xx, y0 + 22 + yy, .5) for xx in range(30) for yy in range(4)]
    for y in range(24):
        hw = int(y * .62) + 1
        for x in range(-hw, hw + 1):
            c = (124, 104, 70) if x < 0 else (96, 80, 52)
            if abs(x) == hw: c = (30, 24, 16)
            if x == 0 and y > 12: c = (20, 16, 12)
            img.put(x0 + 15 + x, y0 + y, c)
tent(23 * T, 10 * T + PAD - 8); tent(27 * T, 12 * T + PAD - 8)
fx, fy = 25 * T + 8, 13 * T + PAD + 4
for (dx, dy) in [(-6, 2), (6, 2), (0, 4)]: img.rect(fx + dx - 4, fy + dy, 9, 3, (70, 46, 26))
light(img, fx, fy, 64, (255, 150, 60), .55)
for k in range(30): img.put(fx + random.randint(-4, 4), fy - random.randint(0, 9), random.choice([(255, 210, 100), (255, 130, 40), (220, 70, 20)]))

# trees: trunks and shadows on the ground layer, canopies on a transparent layer drawn above the tokens
TREES = [(12, 3, 26), (15, 6, 22), (25, 3, 28), (2, 12, 26), (8, 16, 24), (21, 14, 26), (28, 8, 22), (1, 5, 20), (23, 17, 22)]
for (tx, ty, r) in TREES:
    cx, cy = tx * T + 8, ty * T + PAD + 12
    img.shadow_ell(cx + r * .5, cy + 2, r * 1.1, r * .45, .55)
    img.rect(cx - 3, cy - 22, 7, 24, (78, 56, 38)); img.rect(cx - 3, cy - 22, 2, 24, (110, 82, 56)); img.rect(cx + 2, cy - 22, 2, 24, (52, 36, 24))
    img.rect(cx - 6, cy, 13, 2, (60, 44, 30))
grade(img, (1.05, 1.0, .88), .95, .6)
# dappled light under the canopy
for y in range(img.h):
    for x in range(img.w):
        img.p[y][x] = mul(img.p[y][x], .86 + .26 * fbm(x / 13, y / 13, 70))
img.save('forest_map.png')

# canopy layer (RGBA)
canopy = [[(0, 0, 0, 0)] * img.w for _ in range(img.h)]
for (tx, ty, r) in TREES:
    cx, cy = tx * T + 8, ty * T + PAD - 18
    blobs = [(0, 0, 1)] + [(math.cos(a) * r * .5, math.sin(a) * r * .38, .62) for a in [k * 1.1 for k in range(6)]]
    for (ox, oy, s) in blobs:
        rr = r * s
        for y in range(int(cy + oy - rr) - 1, int(cy + oy + rr) + 2):
            for x in range(int(cx + ox - rr) - 1, int(cx + ox + rr) + 2):
                if not (0 <= x < img.w and 0 <= y < img.h): continue
                d = ((x - cx - ox) / rr) ** 2 + ((y - cy - oy) / (rr * .85)) ** 2
                if d > 1: continue
                lit = 1 - ((x - cx) / r * .5 + (y - cy) / r * .7)
                c = mix((30, 62, 30), (110, 156, 66), max(0, min(1, .35 * lit + .4 * fbm(x / 3, y / 3, 80))))
                canopy[y][x] = (*mul(c, 1.02), 255)
# outline only the outer edge of each canopy, plus a darker rim below for depth
mask = [[canopy[y][x][3] > 0 for x in range(img.w)] for y in range(img.h)]
for y in range(img.h):
    for x in range(img.w):
        if not mask[y][x]: continue
        edge = any(not (0 <= x + dx < img.w and 0 <= y + dy < img.h and mask[y + dy][x + dx]) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        below = not (y + 3 < img.h and mask[y + 3][x])
        if edge: canopy[y][x] = (14, 28, 14, 255)
        elif below: canopy[y][x] = (*mul(canopy[y][x][:3], .7), 255)
rows = [b''.join(bytes(p) for p in r) for r in canopy]
raw = b''.join(b'\x00' + r for r in rows)
def ch(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
open('forest_canopy.png', 'wb').write(b'\x89PNG\r\n\x1a\n' + ch(b'IHDR', struct.pack('>IIBBBBB', img.w, img.h, 8, 6, 0, 0, 0)) + ch(b'IDAT', zlib.compress(raw, 9)) + ch(b'IEND', b''))
# flattened preview
for y in range(img.h):
    for x in range(img.w):
        if canopy[y][x][3]: img.p[y][x] = mix(img.p[y][x], canopy[y][x][:3], .9)
img.save('forest_flat_x2.png', 2)
print(img.w, img.h)
