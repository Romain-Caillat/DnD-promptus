"""Add the front (south) and back (north) frames to the starter sprite packs.

characters/walk-in-four-directions. Run once, from the repository root:

    uv run --with pyyaml python docs/design/sprite-turn.py

It was run when the ticket was built: the packs in content/sprites/ now
hold their four directions and are the source, edited by hand. Running
it again does nothing to a piece that already has `north` and `south`.

How a profile becomes a front and a back view
---------------------------------------------
The bodies, hair, beards and headwear are drawn by hand below (HAND):
a face or a hat cannot be guessed from a profile. Every other piece is
turned by rule, from the depth of each layer, onto the front body:

- legs and feet: the back leg goes left, the front leg right, the gap
  between them widens by one column;
- torso, armour, belt: each row is resampled across the front torso,
  the centre column taken from the profile's front edge (south) or its
  back edge (north) and the sides from its middle — a coat's lapels end
  up in the middle of the chest, its back seam in the middle of the back;
- front arm and hand: copied onto both arms, which hang beside the torso;
- weapon and front (what the hand holds): moved to the right hand
  (south), or mirrored into the left hand behind the body (north);
- back (a jetpack, a bag): centred behind the body (south) or over its
  back (north);
- face and head layers of other pieces (a scar): resampled across the
  face, south only.

`west` stays the mirror of `east`. Every layer keeps its pack colours.
"""

import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
PACKS = ["content/sprites/marins-1718/pack.yaml", "content/sprites/equipage-spatial/pack.yaml"]

# ---------------------------------------------------------------------------
# Front body geometry (shared by both packs: same bodies)
# ---------------------------------------------------------------------------

# Torso columns in profile and from the front, per body.
TORSO = {"svelte": ((6, 12), (7, 12)), "robuste": ((5, 13), (6, 13))}
HEAD = ((6, 13), (6, 13))
# Arms hang at x 4-5 and 14-15 on both bodies, hands one row below.
ARM_DX = (-7, 3)  # profile front arm at x 11
HAND_DX = (-9, 1)  # profile hand at x 13, y 16
HAND_DY = 1


def L(depth, at, *rows):
    return {"depth": depth, "at": list(at), "rows": list(rows)}


ARMS = "ss........ss"
FACE_SOUTH = ["..ssss..", ".ssssss.", "ssssssss", "ssessess", "ssssssss", "ssssssss", ".ssssss.", "..ssss.."]
FACE_NORTH = ["..ssss..", ".ssssss.", "ssssssss", "ssssssss", "ssssssss", "ssssssss", ".ssssss.", "..ssss.."]


def body(legs_at, legs, feet_at, feet, torso_at, torso, face):
    return [
        L("legs", legs_at, *[legs] * 5),
        L("feet", feet_at, *[feet] * 2),
        L("torso", torso_at, *[torso] * 8),
        L("head", (6, 5), *face),
        L("front_arm", (4, 13), *[ARMS] * 4),
        L("hand", (4, 17), *[ARMS] * 2),
    ]


def bodies():
    out = {}
    for d, face in (("south", FACE_SOUTH), ("north", FACE_NORTH)):
        out.setdefault("svelte", {})[d] = body((7, 19), "ss..ss", (6, 24), "sss..sss", (7, 12), "ssssss", face)
        out.setdefault("robuste", {})[d] = body(
            (6, 19), "sss..sss", (5, 24), "ssss..ssss", (6, 12), "ssssssss", face
        )
    return out


COURT_TOP = ["..hhhh..", ".hhhhhh.", "hhhhhhhh"]
HAIR = {
    "court": {
        "south": [L("hair", (6, 4), *COURT_TOP, "h......h")],
        "north": [L("hair", (6, 4), *COURT_TOP, "hhhhhhhh", "hhhhhhhh", "hhhhhhhh", ".hhhhhh.")],
    },
    "longue": {
        "south": [
            L("hair", (5, 4), "..hhhhhh..", ".hhhhhhhh.", "hhhhhhhhhh", "hhh....hhh",
              *["hh......hh"] * 6, "h........h")
        ],
        "north": [L("hair", (5, 4), "..hhhhhh..", ".hhhhhhhh.", *["hhhhhhhhhh"] * 8, ".hhhhhhhh.")],
    },
    "catogan": {
        "south": [L("hair", (6, 4), *COURT_TOP, "h......h")],
        "north": [
            L("hair", (6, 4), *COURT_TOP, "hhhhhhhh", "hhhhhhhh", "hhhhhhhh", ".hhhhhh.",
              "...BB...", "...hh...", "...hh...", "...hh...", "...hh...")
        ],
    },
    "tresses": {
        "south": [
            L("hair", (5, 4), "...hhhh...", "..hhhhhh..", ".hhhhhhhh.", ".h......h.",
              "h........h", "h........h", "h........h", "g........g", "h........h", "h........h", "g........g")
        ],
        "north": [
            L("hair", (6, 4), *COURT_TOP, "hhhhhhhh", "hhhhhhhh", "hhhhhhhh", ".hhhhhh.",
              "..h..h..", "..h..h..", "..g..g..", "..h..h..", "..h..h..", "..g..g..")
        ],
    },
    "rase": {
        "south": [L("hair", (6, 5), "..hhhh..", ".h....h.")],
        "north": [L("hair", (6, 5), "..hhhh..", ".hhhhhh.", "hhhhhhhh", "hhhhhhhh", ".hhhhhh.")],
    },
    "chignon": {
        "south": [L("hair", (6, 3), "...hh...", *COURT_TOP, "h......h")],
        "north": [
            L("hair", (6, 2), "...hh...", "..hhhh..", "..hhhh..", *COURT_TOP, "hhhhhhhh",
              "hhhhhhhh", "hhhhhhhh", ".hhhhhh.")
        ],
    },
}

BEARD = {
    "courte": {"south": [L("beard", (6, 10), "hh....hh", ".hhhhhh.", "..hhhh..")], "north": []},
    "longue": {
        "south": [
            L("beard", (6, 10), "hh....hh", "hhhhhhhh", ".hhhhhh.", ".hhhhhh.", "..hhhh..", "...hh...")
        ],
        "north": [],
    },
    "bouc": {"south": [L("beard", (6, 11), "...hh...", "...hh...")], "north": []},
    "moustache": {"south": [L("beard", (6, 10), "..hhhh..", ".h....h.")], "north": []},
}

BANDANA = {
    "south": [L("headwear", (6, 4), "..dddd..", ".ddaddd.", "dddddadd")],
    "north": [L("headwear", (6, 4), "..dddd..", ".dddddd.", "dddddddd", "...dd...", "..d..d..", "..d..d..")],
}

VORR_TOP = ["c........c", ".c......c.", "..c....c..", "..c....c..", "..dcddcd.."]
VORR_CHEF_TOP = ["c...aa...c", ".c..aa..c.", "..c.aa.c..", "..c.aa.c..", "..dcaacd.."]
VORR_FACE = [
    ".dddddddd.", ".dddddddd.", "dxxddddxxd", "ddxddddxdd", "daddddddad", "dddddddddd",
    ".ddaddadd.", "..dddddd..", ".o..dd..o.",
]
VORR_BACK = [
    ".dddddddd.", ".dddddddd.", "dddddddddd", "dddddddddd", "dadddddddd", "dddddddddd",
    ".ddaddadd.", "..dddddd..", "....dd....",
]

HEADWEAR = {
    "marins-1718": {
        "tricorne": {
            "south": [L("headwear", (3, 3), "....dddddd....", "...dddddddd...", "d.dddddddddd.d", ".aaaaddddaaaa.")],
            "north": [L("headwear", (3, 3), "....dddddd....", "...dddddddd...", "d.dddddddddd.d", ".aaaaaaaaaaaa.")],
        },
        "bandana": BANDANA,
        "chapeau": {
            d: [L("headwear", (3, 2), "....dddddd....", "....dddddd....", "....dddddd....", "....aaaaaa....",
                  "dddddddddddddd")]
            for d in ("south", "north")
        },
        "foulard": {
            "south": [L("headwear", (6, 4), "..dadd..", ".dddadd.", "dddddddd", "dd....dd", "d......d")],
            "north": [
                L("headwear", (6, 4), "..dadd..", ".dddadd.", "dddddddd", "dddddddd", "dddadddd",
                  "dddddddd", ".dddddd.", "..dddd..", "...dd...")
            ],
        },
        "bonnet": {
            "south": [L("headwear", (6, 4), "..dddd..", ".dddddd.", "aaaaaaaa")],
            "north": [L("headwear", (6, 3), "..d.....", "..dddd..", ".dddddd.", "aaaaaaaa")],
        },
    },
    "equipage-spatial": {
        "lunettes": {
            "south": [L("headwear", (6, 5), "mvvmmvvm", "lmmllmml")],
            "north": [L("headwear", (6, 6), "llllllll")],
        },
        "oreillette": {
            "south": [
                L("headwear", (4, 1), "v....", "m....", "m....", "m....", ".m...", ".m...", ".m...",
                  "uu...", "uu...", ".uu..", "..uuv")
            ],
            "north": [
                L("headwear", (14, 1), "...v", "...m", "...m", "...m", "..m.", "..m.", "..m.", "..uu",
                  "..uu", ".uu.")
            ],
        },
        "bandana": BANDANA,
        "casque-vorr": {
            "south": [L("headwear", (5, 0), *VORR_TOP, *VORR_FACE)],
            "north": [L("headwear", (5, 0), *VORR_TOP, *VORR_BACK)],
        },
        "casque-vorr-chef": {
            "south": [L("headwear", (5, 0), *VORR_CHEF_TOP, *VORR_FACE)],
            "north": [L("headwear", (5, 0), *VORR_CHEF_TOP, *VORR_BACK)],
        },
    },
}


def hand_drawn(pack, slot, piece):
    if slot == "body":
        return bodies().get(piece)
    if slot == "hair":
        return HAIR.get(piece)
    if slot == "beard":
        return BEARD.get(piece)
    if slot == "headwear":
        return HEADWEAR[pack].get(piece)
    return None


# ---------------------------------------------------------------------------
# Turning a profile by rule
# ---------------------------------------------------------------------------


def cells(layer):
    x0, y0 = layer.get("at", [0, 0])
    for dy, row in enumerate(layer["rows"]):
        for dx, ch in enumerate(row):
            if ch != ".":
                yield x0 + dx, y0 + dy, ch


def resample(row_cells, east, front, facing):
    """One row of a layer across a front span, from the facing half of the profile."""
    (e_lo, e_hi), (f_lo, f_hi) = east, front
    e_mid = (e_lo + e_hi) / 2
    hw = (f_hi - f_lo + 1) / 2
    centre = (f_lo + f_hi) / 2
    out = {}
    for c in range(f_lo, f_hi + 1):
        d = abs(c - centre) - 0.5
        t = d / (hw - 1) if hw > 1 else 0
        src = e_hi - round(t * (e_hi - e_mid)) if facing == "south" else e_lo + round(t * (e_mid - e_lo))
        if src in row_cells:
            out[c] = row_cells[src]
    return out


def turn(layers, body_id, facing):
    """The `facing` view of a profile's layers, by the rules of the module doc."""
    torso = TORSO[body_id]
    painted = {}  # depth -> {(x, y): ch}

    def put(depth, x, y, ch):
        if 0 <= x < 20 and 0 <= y < 26:
            painted.setdefault(depth, {})[(x, y)] = ch

    for layer in layers:
        depth = layer["depth"]
        cs = list(cells(layer))
        rows = {}
        for x, y, ch in cs:
            rows.setdefault(y, {})[x] = ch
        if depth in ("legs", "feet"):
            for x, y, ch in cs:
                for nx in ([x] if x < 9 else [9, 10] if x == 9 else [x + 1]):
                    put(depth, nx, y, ch)
        elif depth in ("torso", "armour", "belt"):
            for y, row in rows.items():
                for x, ch in resample(row, torso[0], torso[1], facing).items():
                    put(depth, x, y, ch)
        elif depth == "front_arm":
            for x, y, ch in cs:
                for dx in ARM_DX:
                    put(depth, x + dx, y, ch)
        elif depth == "back_arm":
            continue
        elif depth == "hand":
            for x, y, ch in cs:
                for dx in HAND_DX:
                    put(depth, x + dx, y + HAND_DY, ch)
        elif depth in ("weapon", "front"):
            for x, y, ch in cs:
                if facing == "south":
                    put(depth, x + 1, y + HAND_DY, ch)
                else:
                    put("back", 18 - x, y + HAND_DY, ch)
        elif depth == "back":
            xs = [x for x, _, _ in cs]
            shift = round(9.5 - (min(xs) + max(xs)) / 2) if xs else 0
            for x, y, ch in cs:
                put("back" if facing == "south" else "front", x + shift, y, ch)
        else:  # head, face, hair, beard, headwear
            if facing == "north" and depth in ("face", "beard"):
                continue
            for y, row in rows.items():
                for x, ch in resample(row, HEAD[0], HEAD[1], facing).items():
                    put(depth, x, y, ch)

    out = []
    for depth, grid in painted.items():
        xs = [x for x, _ in grid]
        ys = [y for _, y in grid]
        x0, y0 = min(xs), min(ys)
        rows = [
            "".join(grid.get((x, y), ".") for x in range(x0, max(xs) + 1)).rstrip(".") or "."
            for y in range(y0, max(ys) + 1)
        ]
        out.append(L(depth, (x0, y0), *rows))
    return out


# ---------------------------------------------------------------------------
# Writing the frames into the pack, after each `east:` block
# ---------------------------------------------------------------------------


def emit(direction, layers, indent):
    pad = " " * indent
    if not layers:
        return [f"{pad}{direction}: []"]
    lines = [f"{pad}{direction}:"]
    for layer in layers:
        lines.append(f"{pad}  - depth: {layer['depth']}")
        lines.append(f"{pad}    at: [{layer['at'][0]}, {layer['at'][1]}]")
        lines.append(f"{pad}    rows:")
        lines += [f'{pad}      - "{r}"' for r in layer["rows"]]
    return lines


def main():
    for rel in PACKS:
        path = ROOT / rel
        text = path.read_text()
        pack = yaml.safe_load(text)
        # The frames to add, in file order: one entry per `east:` key.
        todo = []
        for slot, pieces in pack["pieces"].items():
            for piece in pieces:
                drawn = hand_drawn(pack["id"], slot, piece["id"])
                for body_id, frames in [("svelte", piece["frames"])] + list(piece.get("per_body", {}).items()):
                    if "north" in frames and "south" in frames:
                        todo.append(None)
                        continue
                    if drawn and slot != "body":
                        new = drawn
                    elif slot == "body":
                        new = drawn
                    else:
                        new = {d: turn(frames["east"], body_id, d) for d in ("south", "north")}
                    todo.append(new)
        lines = text.split("\n")
        out = []
        i = 0
        k = 0
        while i < len(lines):
            line = lines[i]
            out.append(line)
            m = re.match(r"^( *)east:$", line)
            i += 1
            if not m:
                continue
            indent = len(m.group(1))
            while i < len(lines) and lines[i].strip() and len(lines[i]) - len(lines[i].lstrip()) > indent:
                out.append(lines[i])
                i += 1
            new = todo[k]
            k += 1
            if new is None:
                continue
            out += emit("south", new["south"], indent)
            out += emit("north", new["north"], indent)
        if k != len(todo):
            sys.exit(f"{rel}: {k} east blocks for {len(todo)} frame sets")
        path.write_text("\n".join(out))
        print(f"{rel}: {sum(1 for t in todo if t)} frame sets turned")


if __name__ == "__main__":
    main()
