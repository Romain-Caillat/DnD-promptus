"""Layered pixel avatar for the character creator: a dwarf built from swappable pieces.

Each piece is its own SVG stacked over the others, so the board swaps one with an `sc-if`.
Recolourable parts use classes (`k-skin-m`…) whose fill comes from CSS variables set on the
wrapper, so a colour swatch only rewrites one style attribute. Same pixel grid (20 x 26),
outline and three-tone shading as `sprite-prototype.py`.
"""
import colorsys

W, H = 20, 26
OUT = '#1B1426'
FIXED = {'metal': '#C9CFD9', 'horn': '#EDE3C8', 'wood': '#8A5A33', 'leather': '#7A5233', 'gold': '#F2C14E',
         'pants': '#5A4636', 'boot': '#3B2A20', 'eyew': '#FFFFFF', 'eye': '#1B1426'}
NOSHADE = {'eyew', 'eye', 'gold'}
VARS = ('skin', 'hair', 'outfit')

PALETTES = {
    'skin': {'clair': '#F0C09A', 'hale': '#E8A982', 'brun': '#A86B4A'},
    'hair': {'roux': '#D9762B', 'brun': '#7A4A2A', 'noir': '#2E2A33', 'blanc': '#E6E0D0'},
    'outfit': {'bleu': '#4F6D9A', 'vert': '#3F7D4E', 'rouge': '#A3362E', 'violet': '#4B3A6B'},
}


def adj(hexc, f):
    r, g, b = [int(hexc[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    l = max(0, min(1, l * f if f < 1 else l + (1 - l) * (f - 1)))
    r, g, b = colorsys.hls_to_rgb(h, l, s)
    return '#%02X%02X%02X' % (round(r * 255), round(g * 255), round(b * 255))


def css_vars(skin, hair, outfit):
    out = []
    for k, c in (('skin', PALETTES['skin'][skin]), ('hair', PALETTES['hair'][hair]), ('outfit', PALETTES['outfit'][outfit])):
        out += [f'--{k}-l: {adj(c, 1.28)}', f'--{k}-m: {c}', f'--{k}-d: {adj(c, .72)}']
    return '; '.join(out)


class L:
    def __init__(s):
        s.g = {}

    def px(s, x, y, c):
        x, y = int(round(x)), int(round(y))
        if 0 <= x < W and 0 <= y < H:
            s.g[(x, y)] = c

    def rect(s, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                s.px(x, y, c)

    def ell(s, cx, cy, rx, ry, c, ymin=-99, ymax=99):
        for y in range(H):
            for x in range(W):
                if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1 and ymin <= y <= ymax:
                    s.g[(x, y)] = c

    def line(s, x0, y0, x1, y1, c):
        n = max(abs(x1 - x0), abs(y1 - y0)) or 1
        for i in range(n + 1):
            s.px(x0 + (x1 - x0) * i / n, y0 + (y1 - y0) * i / n, c)

    def svg(s, px, cls='', mask=frozenset()):
        g = s.g
        rects = []
        outline = set()
        for (x, y) in g:
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                p = (x + dx, y + dy)
                if p not in g and p not in mask and 0 <= p[0] < W and 0 <= p[1] < H:
                    outline.add(p)
        cells = {p: ('fill', OUT) for p in outline}
        for (x, y), c in g.items():
            top = (x, y - 1) not in g or g[(x, y - 1)] != c
            back = (x - 1, y) not in g
            bot = (x, y + 1) not in g
            tone = 'l' if top and not bot else 'd' if back or bot else 'm'
            if c in VARS:
                cells[(x, y)] = ('class', f'k-{c}-{tone}')
            else:
                base = FIXED[c]
                cells[(x, y)] = ('fill', base if c in NOSHADE or tone == 'm' else adj(base, {'l': 1.28, 'd': .72}[tone]))
        # One rect per horizontal run of the same paint keeps the markup small.
        for y in range(H):
            x = 0
            while x < W:
                p = cells.get((x, y))
                if p is None:
                    x += 1
                    continue
                x1 = x
                while cells.get((x1 + 1, y)) == p:
                    x1 += 1
                rects.append(f'<rect x="{x}" y="{y}" width="{x1 - x + 1}" height="1" {p[0]}="{p[1]}"></rect>')
                x = x1 + 1
        return (f'<svg class="avl {cls}" viewBox="0 0 {W} {H}" width="{W * px}" height="{H * px}" shape-rendering="crispEdges" '
                f'xmlns="http://www.w3.org/2000/svg">' + ''.join(rects) + '</svg>')


def base():
    c = L()
    c.rect(3, 14, 4, 19, 'outfit')                  # back arm
    c.rect(6, 21, 8, 23, 'pants'); c.rect(10, 21, 12, 23, 'pants')
    c.rect(5, 24, 8, 25, 'boot'); c.rect(10, 24, 13, 25, 'boot')
    c.rect(4, 13, 13, 21, 'outfit')                 # body
    c.rect(4, 19, 13, 20, 'leather'); c.px(11, 19, 'gold'); c.px(11, 20, 'gold')
    c.ell(9, 9, 4.6, 4.6, 'skin')                   # head
    c.px(11, 9, 'eyew'); c.px(12, 9, 'eye')
    c.px(14, 9, 'skin'); c.px(15, 10, 'skin')       # nose
    return c


def beard(kind):
    c = L()
    if kind == 'longue':
        c.ell(10, 13, 5.2, 4.2, 'hair', ymin=10)
    elif kind == 'tressee':
        c.ell(10, 12, 4.6, 3.2, 'hair', ymin=10)
        c.rect(8, 14, 9, 17, 'hair'); c.rect(12, 14, 13, 17, 'hair')
        c.px(8, 16, 'gold'); c.px(13, 16, 'gold')
    else:  # courte
        c.ell(10, 11.5, 4.6, 2.2, 'hair', ymin=10)
    c.rect(11, 10, 14, 10, 'hair')                  # moustache
    return c


def headgear(kind):
    c = L()
    if kind == 'casque':
        c.ell(9, 6, 5.4, 3.6, 'metal', ymax=6)
        c.rect(3, 6, 14, 7, 'metal')
        for (x, y) in [(3, 4), (2, 3), (2, 2), (1, 1), (14, 4), (15, 3), (15, 2), (16, 1)]:
            c.px(x, y, 'horn')
    elif kind == 'capuche':
        c.ell(8.5, 7, 5.4, 4.6, 'outfit', ymax=8)
        c.rect(4, 8, 5, 12, 'outfit')
    else:  # nu: hair on top
        c.ell(9, 6, 4.8, 2.8, 'hair', ymax=6)
        c.rect(4, 6, 6, 9, 'hair')
    return c


def weapon(kind):
    c = L()
    if kind == 'hache':
        c.line(16, 6, 16, 22, 'wood')
        c.rect(17, 6, 19, 11, 'metal'); c.px(17, 5, 'metal'); c.px(17, 12, 'metal')
    elif kind == 'marteau':
        c.line(16, 8, 16, 22, 'wood')
        c.rect(14, 4, 19, 8, 'metal'); c.rect(14, 4, 14, 8, 'gold')
    else:  # epee
        c.line(16, 3, 16, 14, 'metal'); c.rect(14, 15, 18, 15, 'gold'); c.line(16, 16, 16, 18, 'leather')
    return c


def front():
    c = L()
    c.rect(12, 14, 14, 17, 'outfit')
    c.rect(15, 16, 16, 17, 'skin')
    return c


OPTIONS = {
    'beard': [('longue', 'Longue'), ('tressee', 'Tressée'), ('courte', 'Courte')],
    'head': [('casque', 'Casque à cornes'), ('capuche', 'Capuche'), ('nu', 'Tête nue')],
    'weapon': [('hache', 'Hache'), ('marteau', 'Marteau'), ('epee', 'Épée')],
}
LAYER_FN = {'beard': beard, 'head': headgear, 'weapon': weapon}
AV_CSS = ''.join(f'.av .k-{k}-{t}{{fill:var(--{k}-{t})}}' for k in VARS for t in 'lmd') + '.av{position:relative;flex:none}.av > .avl, .av > sc-if > .avl{position:absolute;left:0;top:0}'


def avatar(px, hole='av'):
    """The stacked layers; `{{<hole>Vars}}` sets the colours, `{{<hole>_<layer>_<kind>}}` shows a piece."""
    # A piece's outline is drawn only outside the body, so a beard or a helmet never hides the face.
    body = frozenset(base().g) | frozenset(front().g)
    parts = [base().svg(px)]
    for layer, opts in OPTIONS.items():
        if layer == 'weapon':
            continue
        for k, _ in opts:
            parts.append('<sc-if value="{{%s_%s_%s}}" hint-placeholder-val="{{ false }}">%s</sc-if>' % (hole, layer, k, LAYER_FN[layer](k).svg(px, mask=body)))
    parts.append(front().svg(px))
    for k, _ in OPTIONS['weapon']:
        parts.append('<sc-if value="{{%s_weapon_%s}}" hint-placeholder-val="{{ false }}">%s</sc-if>' % (hole, k, weapon(k).svg(px, mask=body)))
    return f'<div class="av" style="width: {W * px}px; height: {H * px}px; {{{{{hole}Vars}}}}">' + ''.join(parts) + '</div>'


def palettes_js():
    """{layer: {name: 'css vars'}}; the board joins one entry per layer into the wrapper's style."""
    import json
    return json.dumps({k: {n: '; '.join(f'--{k}-{t}: {adj(c, f)}' for t, f in (('l', 1.28), ('m', 1), ('d', .72))) for n, c in v.items()} for k, v in PALETTES.items()}, ensure_ascii=False)
