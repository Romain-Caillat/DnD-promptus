from outdoor import *
random.seed(11)
G = [
 "PPPPPPPPPPPPssssssssxxxxxxxxxx",
 "PPPPPPPPPPPPssssssxxxxxxxxssss",
 "PPPPPPPPPPPsssssxxxxxxxsssssss",
 "PPPPPPPPPPssssssxxxxxsssrrssss",
 "sssssssssssssssxxxxxxssrrrssss",
 "sssrrsssssssssssxxxssssrrsssss",
 "ssrrrkkssssssssssssssssssssss s"[:30],
 "sssrkkksssssssssssssssssssssss",
 "ssssskssssssssssssssssskkkssss",
 "ssssssssssssssssssssssskkkkkss",
 "ssssssssssssrrrsssssssssssssss",
 "sssssssssssrrrrrssssssssHHHHHH",
 "ssssssssssssrrrsssssssssHHHHHH",
 "sssskksssssssssssssssssHHHHHHH",
 "ssskkkksssssssssssssssssssssss",
 "sssskkssssssssssssssrrssssssss",
 "ssssssssssssssssssssrrrsssssss",
 "ssssssssssssssssssssssssssssss",
]
G = [r.replace(' ', 's') for r in G]
R, C = len(G), len(G[0])
PAD = 3 * T
img = Img(C * T, R * T + PAD, PAD)
def sand(x, y):
    n = fbm(x / 9, y / 9, 1)
    rip = math.sin(x * .32 + y * .12 + fbm(x / 20, y / 20, 2) * 9)
    c = mix((160, 102, 62), (196, 136, 86), n)
    c = mul(c, 1 + .06 * rip)
    if h2(x, y, 3) > .97: c = add(c, 24)
    return c
def regolith(x, y):
    n = fbm(x / 4, y / 4, 4)
    c = mix((104, 70, 52), (140, 96, 66), n)
    if h2(x, y, 5) > .9: c = mul(c, .7)
    if h2(x + 1, y, 5) > .93: c = add(c, 30)
    return c
crack = set()
for k in range(30):
    x, y = random.randint(0, C * T), random.randint(0, R * T)
    for s in range(random.randint(10, 30)):
        crack.add((x, y)); x += random.choice((-1, 0, 1)); y += random.choice((-1, 0, 1, 1))
def rock(x, y):
    n = fbm(x / 5, y / 5, 6)
    c = mix((84, 62, 54), (118, 88, 70), n)
    if (x, y) in crack: c = mul(c, .5)
    return c
def scorched(x, y):
    n = fbm(x / 6, y / 6, 7)
    c = mix((40, 32, 30), (82, 62, 48), n * .8)
    if h2(x, y, 8) > .97: c = (230, 120, 50)
    return c
mats = {'s': sand, 'r': regolith, 'k': rock, 'x': scorched, 'P': sand, 'H': sand}
ground(img, G, mats, set('srkx'), 5)
for py in range(PAD):
    for px in range(img.w): img.p[py][px] = mats[G[0][px // T]](px, py - PAD)

def cap(x, y, ch):
    if ch == 'H':
        c = (150, 156, 162) if (x // 8 + y // 8) % 2 else (140, 146, 152)
        if x % 8 == 0 or y % 8 == 0: c = (110, 114, 120)
        return c
    n = fbm(x / 7, y / 7, 20)
    c = mix((176, 118, 74), (206, 150, 98), n)
    if h2(x, y, 21) > .95: c = mul(c, .7)
    return c
def face(x, j, H, ch):
    if ch == 'H':
        tx = x // 16; i = x % 16
        c = (188, 192, 196) if x % 16 else (130, 134, 140)
        if 8 <= j <= 12: c = (205, 232, 250) if i % 8 else (90, 100, 110)
        if tx == 26 and j >= 12 and 3 <= i <= 12: c = (232, 232, 228) if ((i + j) // 3) % 2 else (16, 16, 16)
        if j < 2: c = add(c, 20)
        return mul(c, 1 - .3 * (j / H))
    band = int(j / 5 + fbm(x / 14, j / 6, 22) * 2) % 3
    c = [(152, 98, 64), (124, 78, 52), (170, 114, 74)][band]
    if h2(x // 2, j // 6, 23) > .86: c = mul(c, .62)
    if j < 2: c = add(c, 26)
    return mul(c, 1 - .4 * (j / H))
cliff_and_walls(img, G, set('PH'), cap, face, 28, 30, beyond='n')

def crater(cx, cy, r):
    img.ell(cx, cy, r, r * .7, lambda x, y, d: img.put(x, y, mul(img.p[y][x], .6 + .25 * d) if d < .75 else add(img.p[y][x], 22)))
crater(6 * T, 9 * T + PAD, 26); crater(14 * T + 8, 15 * T + PAD, 18); crater(9 * T, 13 * T + PAD, 10)

def boulder(cx, cy, rx, ry):
    img.shadow_ell(cx + 5, cy + ry * .5, rx + 2, ry * .5, .5)
    img.ell(cx, cy, rx, ry, lambda x, y, d: img.put(x, y, (24, 18, 16) if d > .82 else mul((118, 88, 72), 1.25 - .5 * d - .25 * ((x - cx) / rx + (y - cy) / ry))))
for b in [(3 * T, 6 * T, 10, 7), (12 * T, 7 * T + 4, 12, 8), (21 * T, 13 * T, 9, 6), (2 * T, 15 * T, 12, 8), (27 * T, 7 * T, 8, 6), (8 * T, 4 * T + 10, 7, 5)]:
    boulder(b[0], b[1] + PAD, b[2], b[3])

def crystals(cx, cy):
    light(img, cx, cy - 6, 34, (60, 230, 210), .45)
    for (dx, hgt, w) in [(-6, 16, 4), (-1, 24, 5), (5, 14, 4), (9, 9, 3)]:
        img.shadow_ell(cx + dx + 6, cy + 2, w + 3, 2, .55)
        for y in range(hgt):
            ww = max(1, int(w * (1 - y / hgt) + .5))
            for x in range(-ww, ww + 1):
                c = (60, 200, 186) if x < 0 else (140, 250, 236)
                if abs(x) == ww: c = (16, 40, 40)
                img.put(cx + dx + x, cy - y, c)
crystals(4 * T, 12 * T + PAD); crystals(26 * T, 4 * T + PAD); crystals(10 * T, 1 * T + PAD - 24)

# crashed dropship
cx, cy = 18 * T, 4 * T + PAD - 4
img.shadow_ell(cx + 10, cy + 18, 58, 12, .45)
for y in range(-18, 19):
    for x in range(-56, 57):
        d = (x / 56) ** 2 + (y / 18) ** 2
        if d > 1: continue
        c = (160, 164, 170) if y < 4 else (110, 114, 120)
        if x % 9 == 0 or y == -6: c = mul(c, .75)
        if y < -12: c = add(c, 30)
        if x > 34 and -12 < y < 0: c = (40, 60, 80) if (x - y) % 7 else (150, 180, 200)
        if -20 < x < -4 and -6 < y < 8 and fbm(x / 4, y / 4, 40) > .45: c = (24, 20, 20)
        if fbm(x / 6, y / 6, 41) > .66: c = mix(c, (40, 34, 30), .6)
        if d > .9: c = (20, 20, 22)
        img.put(cx + x, cy + y, c)
for y in range(0, 22):
    for x in range(0, 30 - y):
        img.put(cx - 30 + x - y, cy + 10 + y, (20, 20, 22) if x == 0 or x == 29 - y or y == 21 else (128, 132, 138))
for e in (-52, -46):
    img.ell(cx + e, cy, 7, 9, lambda x, y, d: img.put(x, y, (18, 16, 16) if d > .7 else (70, 70, 74)))
light(img, cx - 12, cy, 46, (255, 120, 40), .6)
for k in range(40):
    img.put(cx - 12 + random.randint(-8, 8), cy + random.randint(-5, 6), random.choice([(255, 200, 90), (255, 120, 40), (220, 70, 20)]))
for k in range(30):
    x = cx + random.randint(-90, 60); y = cy + random.randint(14, 60)
    s = random.randint(2, 5); img.rect(x, y, s, s - 1, random.choice([(150, 154, 160), (96, 100, 106)])); img.put(x, y + s - 1, (20, 20, 20))

# cover: concrete barriers and military crates in front of the outpost
def barrier(x0, y0, w):
    [img.shade(x0 + 3 + xx, y0 + 10 + yy, .5) for xx in range(w) for yy in range(4)]
    for x in range(w):
        for y in range(12):
            c = (150, 148, 142) if y < 4 else (120, 118, 112) if y < 10 else (84, 82, 78)
            if x in (0, w - 1) or y == 11: c = (24, 22, 22)
            if x % 16 == 0: c = mul(c, .7)
            img.put(x0 + x, y0 + y, mul(c, .92 + .16 * h2(x, y, 50)))
barrier(16 * T, 9 * T + PAD + 2, 3 * T); barrier(20 * T, 10 * T + PAD + 4, 2 * T); barrier(11 * T, 9 * T + PAD - 2, 2 * T)
def crate(x0, y0, s=14):
    [img.shade(x0 + 3 + xx, y0 + s - 3 + yy, .5) for xx in range(s) for yy in range(4)]
    for x in range(s):
        for y in range(s):
            c = (78, 88, 64) if y < s - 4 else (54, 62, 44)
            if x in (0, s - 1) or y in (0, s - 1): c = (20, 22, 18)
            if y == s // 3: c = (40, 46, 34)
            img.put(x0 + x, y0 + y, c)
for (x, y) in [(22 * T + 6, 9 * T + 4), (23 * T + 6, 9 * T + 8), (14 * T + 4, 11 * T + 2)]:
    crate(x, y + PAD)
# solar panels and an antenna by the outpost
for k in range(3):
    x0, y0 = 24 * T + k * 22, 16 * T + PAD - 4
    [img.shade(x0 + 4 + xx, y0 + 12 + yy, .55) for xx in range(18) for yy in range(3)]
    for x in range(18):
        for y in range(12):
            img.put(x0 + x, y0 + y, (30, 44, 70) if x % 6 and y % 4 else (150, 160, 170))
ax, ay = 28 * T + 8, 11 * T + PAD - 28
img.rect(ax, ay - 30, 2, 30, (170, 174, 180)); img.rect(ax - 6, ay - 30, 14, 2, (170, 174, 180)); img.put(ax, ay - 33, (255, 80, 60))
grade(img, (1.04, .98, .9), .88, .65)
img.save('space_map.png')
print(img.w, img.h)
