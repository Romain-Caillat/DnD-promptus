from outdoor import *
random.seed(7)
G = [
 "BBBBBBBBBBBBBdddBBBBBBBBBBBBBB",
 "BBBBBBBBBBBBBdddBBBBBBBBBBBBBB",
 "BBBBBBBBBBBBBdddBBBBBBBBBBBBBB",
 "wwwwwwwwwwwwwwwwwwwwwwwwwwwwww",
 "wwwwwwwwwwwwwwwwwwwwwwwwwwwwww",
 "cccccccccccccccccccccccccccccc",
 "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
 "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
 "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
 "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
 "cccccccccccccccccccccccccccccc",
 "wwwwwwwwwwwwwwwwwwwwwwwwwwwwww",
 "ggggggggdgggggggggggggggggggggg"[:30],
 "gggddggggggggggggddgggggRRRRRR",
 "ggdddggggggggggggdddggggRRRRRR",
 "gggdggggggggggggggggggggRRRRRR",
 "ggggggggggggggddggggggggRRRRRR",
 "ggggggggggggggggggggggggRRRRRR",
]
R, C = len(G), len(G[0])
PAD = 3 * T
img = Img(C * T, R * T + PAD, PAD)

# cracks on the asphalt: a few random walks
crack = set()
for k in range(14):
    x, y = random.randint(0, C * T), random.randint(6 * T, 10 * T)
    for s in range(random.randint(20, 60)):
        crack.add((x, y)); x += random.choice((-1, 0, 1, 1)); y += random.choice((-1, 0, 0, 1))
def asphalt(x, y):
    n = fbm(x / 4, y / 4, 1)
    c = (52 + (n - .5) * 22, 54 + (n - .5) * 22, 57 + (n - .5) * 22)
    if h2(x, y, 3) > .96: c = add(c, 22)
    if (x, y) in crack: c = mul(c, .45)
    if fbm(x / 20, y / 20, 5) > .68: c = mul(c, .85)
    # centre line and faded edge lines
    if 8 * T - 1 <= y <= 8 * T and (x // 10) % 2 == 0 and h2(x, y, 9) > .25: c = (178, 168, 130)
    if (y == 6 * T + 2 or y == 10 * T - 3) and h2(x // 3, y, 4) > .4: c = mix(c, (170, 170, 162), .6)
    # zebra crossing in front of the alley
    if 13 * T + 2 <= x < 16 * T - 2 and 6 * T + 4 <= y < 10 * T - 4 and (y // 5) % 2 == 0 and h2(x // 4, y // 4, 8) > .18: c = mix(c, (172, 172, 166), .62)
    return c
def sidewalk(x, y):
    n = fbm(x / 6, y / 6, 2)
    c = (112 + (n - .5) * 16, 110 + (n - .5) * 16, 103 + (n - .5) * 16)
    if x % 16 == 0 or (y % 16 == 0 and (x // 16 + y // 16) % 1 == 0): c = mul(c, .72)
    if h2(x, y, 6) > .985: c = mul(c, .6)
    if fbm(x / 9, y / 9, 7) > .72: c = mix(c, (74, 84, 50), .5)  # weeds in the joints
    return c
def curb(x, y):
    j = y % 16
    if j < 4: c = (146, 144, 136)
    elif j < 12: c = (124, 122, 116)
    else: c = (84, 82, 78)
    return add(c, (fbm(x / 3, y, 4) - .5) * 14)
def grass(x, y):
    t = fbm(x / 7, y / 7, 11)
    c = mix((58, 74, 40), (96, 98, 54), t)
    r = h2(x, y, 12)
    if r > .82: c = add(c, 26)
    elif r < .12: c = mul(c, .7)
    return c
def dirt(x, y):
    n = fbm(x / 5, y / 5, 13)
    c = (88 + n * 26, 72 + n * 20, 54 + n * 14)
    if h2(x, y, 14) > .95: c = add(c, 30)
    return c
mats = {'a': asphalt, 'w': sidewalk, 'c': curb, 'g': grass, 'd': dirt, 'B': dirt, 'R': grass}
ground(img, G, mats, set('gd'), 21)

def zone(tx):
    if tx <= 6: return 'pharma'
    if tx <= 12: return 'maison'
    if tx <= 22: return 'garage'
    return 'shop'
def cap(x, y, ch):
    tx = x // 16
    if ch == 'R':
        # south house: tiled roof seen from above
        c = (96, 58, 46) if (y // 4) % 2 else (110, 66, 52)
        if (x + (y // 4) * 4) % 8 == 0: c = mul(c, .7)
        return mul(c, .85 + .3 * fbm(x / 9, y / 9, 31))
    z = zone(tx)
    if z == 'garage':
        c = (112, 114, 116) if x % 4 < 2 else (88, 90, 92)
        if fbm(x / 8, y / 8, 32) > .7: c = mix(c, (128, 78, 48), .5)
        return c
    n = fbm(x / 3, y / 3, 33)
    c = (60 + n * 18, 58 + n * 18, 60 + n * 18)
    if h2(x, y, 34) > .9: c = add(c, 18)
    return c
def face(x, j, H, ch):
    tx = x // 16; i = x % 16; z = zone(tx)
    g = fbm(x / 6, j / 6, 40)
    if z == 'pharma':
        c = (170, 164, 150)
        if 6 <= i <= 11 and 6 <= j <= 17 and tx != 3:
            c = (36, 46, 54) if (tx % 2) else (120, 92, 60) if (i + j) % 5 else (90, 68, 44)
            if i in (6, 11) or j in (6, 17): c = (210, 206, 196)
        if tx == 3 and 4 <= i <= 12 and j >= 9:
            c = (24, 30, 34) if h2(x, j, 41) > .2 else (200, 220, 225)
            if i in (4, 12): c = (90, 90, 90)
    elif z == 'maison':
        c = (124, 70, 56)
        if j % 5 == 0 or (i + (j // 5) * 4) % 8 == 0: c = (92, 84, 78)
        if 4 <= i <= 11 and 7 <= j <= 18 and tx in (8, 10, 12):
            c = (30, 36, 40) if tx != 10 else (126, 96, 62)
            if tx == 10 and (j - i) % 4 == 0: c = (90, 68, 44)
            if i in (4, 11) or j in (7, 18): c = (196, 190, 180)
    elif z == 'garage':
        c = (118, 120, 122) if x % 4 < 2 else (96, 98, 100)
        if 17 <= tx <= 21 and j >= 8:
            c = (74, 76, 78) if j % 3 else (58, 60, 62)
            if j >= 25: c = (12, 12, 12)
        if fbm(x / 7, j / 7, 42) > .66: c = mix(c, (130, 80, 48), .6)
    else:
        c = (116, 124, 130)
        if 24 <= tx <= 28 and 9 <= j <= 26:
            c = (32, 42, 50)
            if (i + j) % 23 == 0 or (i - j) % 31 == 0: c = (150, 165, 175)
            if tx in (25, 26) and (i + j) % 6 < 2: c = (120, 92, 60)
    if j < 2: c = add(c, 30)
    if j >= H - 2: c = mul(c, .55)
    return mul(c, .82 + .3 * g - .25 * (j / H))
# the alley keeps going north beyond the map
for py in range(0, PAD):
    for px in range(13 * T, 16 * T): img.p[py][px] = dirt(px, py + 400)
cliff_and_walls(img, G, set('BR'), cap, face, 2 * T, 30, beyond='ns')

# roof details: AC units, vents, a skylight, parapet line
for (x, y, w, h) in [(30, 8, 18, 12), (150, 14, 14, 10), (390, 6, 22, 14), (440, 16, 12, 10)]:
    img.shadow_ell(x + w / 2 + 3, y + h + 1, w / 2 + 2, 3, .5)
    img.rect(x, y, w, h, (120, 122, 124)); img.rect(x, y, w, 2, (160, 162, 164)); img.rect(x + 3, y + 4, w - 6, h - 7, (70, 72, 74))
    for k in range(x + 4, x + w - 4, 3): img.rect(k, y + 4, 1, h - 7, (40, 42, 44))
img.rect(108, 10, 26, 16, (150, 170, 180)); img.rect(110, 12, 22, 12, (60, 80, 92)); img.rect(110, 12, 6, 12, (110, 130, 140))
for x in range(0, 13 * T): img.put(x, PAD + T - 1, (24, 24, 26)); img.put(x, PAD + T - 2, (90, 90, 92))
for x in range(16 * T, 30 * T): img.put(x, PAD + T - 1, (24, 24, 26)); img.put(x, PAD + T - 2, (90, 90, 92))
# rubble spilling from the garage
for k in range(28):
    x = 19 * T + random.randint(-20, 24); y = PAD + 3 * T + random.randint(0, 10)
    s = random.randint(2, 4); img.rect(x, y, s, s - 1, random.choice([(110, 108, 104), (86, 84, 80), (130, 126, 118)])); img.put(x, y + s - 1, (20, 20, 20))
# pharmacy green cross sign
for dy in range(-3, 4):
    for dx in range(-1, 2):
        img.put(2 * T + 8 + dx, PAD - T + 2 + dy + 10 - 16, (70, 168, 96)); img.put(2 * T + 8 + dy, PAD - T + 2 + dx + 10 - 16, (70, 168, 96))

def car(x0, y0, body, burned=False, door=False):
    w, h = 46, 28
    img.shadow_ell(x0 + w / 2 + 3, y0 + h - 2, w / 2 + 3, 6, .45)
    glass = (14, 12, 12) if burned else (44, 58, 68)
    for y in range(h):
        for x in range(w):
            rx = min(x, w - 1 - x); ry = min(y, h - 1 - y)
            if rx + ry < 4: continue
            side = y >= 19
            if side:
                c = mul(body, .6)
                if x in (16, 30) : c = mul(body, .42)
                if y == 19: c = mul(body, .78)
            else:
                c = add(body, 18) if (x < 10 or x > 37) else body
                if 10 <= x <= 14 and 3 <= y <= 16: c = glass if (x - 10) + abs(y - 9.5) * .2 < 5 else body
                if 15 <= x <= 31 and 3 <= y <= 16: c = mul(body, .82)
                if 32 <= x <= 35 and 4 <= y <= 15: c = glass
                if y < 2: c = add(body, 40)
                if not burned and c == glass and h2(x, y, 51) > .9: c = (170, 186, 196)
            if burned:
                c = mix(c, (26, 22, 20), .55 + .3 * fbm(x / 4, y / 4, 52))
                if fbm(x / 5, y / 5, 53) > .58: c = mix(c, (122, 64, 34), .6)
            elif fbm(x / 6, y / 6, 54) > .72: c = mix(c, (118, 70, 40), .5)
            if rx + ry == 4 or x in (0, w - 1) or y in (0, h - 1): c = (16, 14, 14)
            img.put(x0 + x, y0 + y, c)
    if not burned:
        img.rect(x0 + 1, y0 + 5, 2, 3, (230, 220, 170)); img.rect(x0 + 1, y0 + 12, 2, 3, (230, 220, 170))
        img.rect(x0 + w - 3, y0 + 5, 2, 3, (150, 40, 34)); img.rect(x0 + w - 3, y0 + 12, 2, 3, (150, 40, 34))
    for wx in (5, 34):
        img.rect(x0 + wx, y0 + h - 4, 8, 5, (14, 14, 14)); img.rect(x0 + wx + 2, y0 + h - 3, 4, 2, (60, 60, 62))
    if door:
        for y in range(9):
            for x in range(14): img.put(x0 + 16 + x, y0 + h + y, mul(body, .7) if y < 7 else (16, 14, 14))
        img.rect(x0 + 18, y0 + h + 1, 9, 4, glass)
car(4 * T + 1, 6 * T + 2 + PAD, (78, 100, 124), door=True)
car(17 * T + 1, 7 * T + 6 + PAD, (60, 54, 50), burned=True)
car(24 * T + 4, 8 * T + 8 + PAD, (132, 52, 44))

# blood on the road, drag marks toward the alley
for k in range(260):
    a = random.random() * 6.28; r = random.random() ** 2 * 12
    img.blend(int(14 * T + 4 + math.cos(a) * r * 1.4), int(8 * T + 8 + PAD + math.sin(a) * r), (84, 14, 16), .75)
for s in range(70):
    img.blend(14 * T + 4 + s // 6, 8 * T + 6 + PAD - s, (84, 14, 16), .45)
for s in range(120):
    x = 5 * T + s * 2; y = int(9 * T + 4 + PAD + math.sin(s / 9) * 6)
    img.shade(x, y, .7); img.shade(x, y + 5, .7)
# manhole
img.ell(10 * T + 8, 7 * T + 8 + PAD, 6, 5, lambda x, y, d: img.put(x, y, (40, 40, 42) if d > .55 or (x + y) % 3 else (64, 64, 66)))

# sandbag barricade across the alley mouth and a pallet wall on the south sidewalk
def sandbags(x0, y0, n, rows=2):
    for r in range(rows):
        for k in range(n):
            cx = x0 + k * 9 + (r % 2) * 4; cy = y0 - r * 5
            img.shadow_ell(cx + 2, cy + 3, 6, 3, .6)
            img.ell(cx, cy, 5.5, 3.6, lambda x, y, d: img.put(x, y, (16, 14, 12) if d > .78 else mul((150, 134, 100), 1.1 - .4 * d + .1 * (y < cy - 1))))
sandbags(13 * T - 2, 4 * T + PAD + 6, 6)
def pallets(x0, y0, w):
    [img.shade(x0 + 2 + xx, y0 + 12 + yy, .55) for xx in range(w) for yy in range(4)]
    for x in range(w):
        for y in range(12):
            c = (128, 98, 62) if y % 4 else (80, 60, 38)
            if x % 16 in (0, 15): c = (70, 52, 32)
            img.put(x0 + x, y0 + y, mul(c, .9 + .2 * h2(x, y, 60)))
pallets(8 * T, 11 * T + PAD - 8, 5 * T)

# trash bags by the shop
for (cx, cy) in [(26 * T + 4, 4 * T + 6), (27 * T, 4 * T + 2), (27 * T + 10, 4 * T + 8)]:
    cy += PAD
    img.shadow_ell(cx + 2, cy + 5, 7, 3, .55)
    img.ell(cx, cy, 6.5, 5.5, lambda x, y, d: img.put(x, y, (12, 12, 14) if d > .8 else (28 + 30 * (1 - d) * (y < cy), 28 + 30 * (1 - d) * (y < cy), 32 + 34 * (1 - d) * (y < cy))))

# dead tree and hedges in the yards
for (cx, cy, rx, ry) in [(1 * T, 12 * T + 8, 14, 7), (4 * T, 12 * T + 6, 16, 7), (19 * T, 12 * T + 6, 22, 7)]:
    cy += PAD
    img.shadow_ell(cx + 3, cy + 6, rx, 4, .55)
    img.ell(cx, cy, rx, ry, lambda x, y, d: img.put(x, y, (14, 18, 10) if d > .85 else mix((40, 58, 32), (70, 92, 48), fbm(x / 3, y / 3, 70) * (1 - d * .5))))
tx, ty = 6 * T + 8, 15 * T + 8 + PAD
img.shadow_ell(tx + 14, ty + 2, 22, 5, .55)
img.rect(tx - 2, ty - 26, 5, 28, (64, 50, 40))
for (a, l) in [(-2.3, 22), (-1.2, 20), (-.4, 18), (-2.8, 14), (-1.7, 26)]:
    for s in range(l):
        x = int(tx + math.cos(a) * s); y = int(ty - 22 + math.sin(a) * s)
        img.put(x, y, (58, 46, 36)); img.put(x + 1, y, (40, 32, 26))

# street lights: the pole rises over the facade
for lx in (8 * T + 8, 21 * T + 8):
    base = 4 * T + 12 + PAD
    img.shadow_ell(lx + 12, base + 1, 14, 2, .6)
    img.rect(lx - 1, base - 46, 3, 46, (54, 56, 58)); img.rect(lx, base - 46, 1, 46, (96, 98, 100))
    img.rect(lx - 1, base - 48, 10, 3, (54, 56, 58)); img.rect(lx + 6, base - 46, 4, 2, (210, 200, 160))

# shopping cart
cx, cy = 2 * T + 4, 11 * T + 2 + PAD
for x in range(14):
    for y in range(9):
        if x % 3 == 0 or y % 3 == 0: img.put(cx + x, cy + y, (150, 154, 158))
img.rect(cx + 1, cy + 9, 2, 2, (20, 20, 20)); img.rect(cx + 11, cy + 9, 2, 2, (20, 20, 20))

# paper scraps
for k in range(40):
    x, y = random.randint(0, C * T - 3), random.randint(PAD + 3 * T, PAD + 12 * T)
    img.rect(x, y, 2, 1, (180, 176, 164))

# light: dusk, desaturated, a fire glowing in the burned car
grade(img, (.95, .96, 1.02), .78, .7)
light(img, 18 * T + 8, 7 * T + 14 + PAD, 70, (255, 130, 50), .55)
for k in range(30):
    x = 18 * T + random.randint(-14, 14); y = 7 * T + 10 + PAD - random.randint(0, 9)
    img.put(x, y, random.choice([(255, 196, 90), (255, 120, 40), (210, 60, 20)]))
img.save('zombie_map.png')
print(img.w, img.h)
