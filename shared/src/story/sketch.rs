//! A map as text (`platform/edit-maps-by-claude`).
//!
//! Claude does not see pixels: to work on the atlas it reads a map as
//! rows of characters, with numbered axes so a cell can be named, the
//! grid's own glyphs (their legend says what each is: wall, water,
//! terrain), and one symbol over every thing placed — the map's doors,
//! exits, props, objects, starts and labels, and the preparation's pins,
//! NPCs, clues and fights (`story::pins`, `story::cast`). Symbols never
//! reuse a glyph of the grid's legend.
//!
//! The words a person reads are the caller's to write (the MCP server
//! writes them in French): this module returns the grid and structured
//! legends only.
//!
//! Hex maps (world scale, odd-r offset) take two characters a cell, odd
//! rows shifted by one, so the hexes interlock as they are drawn.
//!
//! Pure: the server renders it from the stored map and story.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::cast::Cast;
use super::pins::Pin;
use crate::maps::{Cell, Geometry, Map, Scale, Side, Visibility, Water};

/// The largest zone rendered at once, in cells: beyond it, the caller
/// asks for a zone.
pub const MAX_W: u32 = 80;
pub const MAX_H: u32 = 50;

/// A rectangle of cells: the top-left cell and a size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Zone {
    fn contains(&self, c: Cell) -> bool {
        c.x >= self.x
            && c.y >= self.y
            && i64::from(c.x) < i64::from(self.x) + i64::from(self.w)
            && i64::from(c.y) < i64::from(self.y) + i64::from(self.h)
    }

    /// The smallest zone holding `cells`, grown by `margin` and cut to
    /// `map`; `None` without cells.
    #[must_use]
    pub fn around(map: &Map, cells: &BTreeSet<Cell>, margin: i32) -> Option<Self> {
        let x0 = cells.iter().map(|c| c.x).min()? - margin;
        let y0 = cells.iter().map(|c| c.y).min()? - margin;
        let x1 = cells.iter().map(|c| c.x).max()? + margin;
        let y1 = cells.iter().map(|c| c.y).max()? + margin;
        Some(clamp(
            map,
            Self {
                x: x0,
                y: y0,
                w: u32::try_from(x1 - x0 + 1).unwrap_or(1),
                h: u32::try_from(y1 - y0 + 1).unwrap_or(1),
            },
        ))
    }
}

/// `zone` cut to the map and to [`MAX_W`] × [`MAX_H`].
fn clamp(map: &Map, zone: Zone) -> Zone {
    let (mw, mh) = (map.grid.width(), map.grid.height());
    let side = |v: i32, max: u32| {
        u32::try_from(v.max(0))
            .unwrap_or(0)
            .min(max.saturating_sub(1))
    };
    let x = side(zone.x, mw);
    let y = side(zone.y, mh);
    let w = zone.w.clamp(1, MAX_W).min(mw - x);
    let h = zone.h.clamp(1, MAX_H).min(mh - y);
    #[allow(clippy::cast_possible_wrap)]
    Zone {
        x: x as i32,
        y: y as i32,
        w,
        h,
    }
}

/// A map as text.
#[derive(Debug, Clone, Serialize)]
pub struct Sketch {
    pub map: String,
    pub name: String,
    pub scale: Scale,
    /// The whole map's size, in cells.
    pub width: u32,
    pub height: u32,
    /// The part drawn.
    pub zone: Zone,
    /// Whether the zone is smaller than the map.
    pub cut: bool,
    /// Two ruler lines (tens, units of `x`), then one line per row, its
    /// `y` first.
    pub rows: Vec<String>,
    /// The grid's glyphs used in the zone.
    pub terrain: Vec<Terrain>,
    /// Every thing placed in the zone, story marks first.
    pub legend: Vec<Mark>,
}

/// What a glyph of the grid stands for.
#[derive(Debug, Clone, Serialize)]
pub struct Terrain {
    pub glyph: char,
    pub terrain: String,
    pub wall: bool,
    pub void: bool,
    pub water: Water,
    pub difficult: bool,
    pub elevation: i8,
}

/// What a symbol over the grid stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkKind {
    /// A place anchored here, holding scenes (`story::pins`).
    Pin,
    Npc,
    Clue,
    Fight,
    Exit,
    Door,
    Prop,
    Object,
    Start,
    Label,
}

#[derive(Debug, Clone, Serialize)]
pub struct Mark {
    pub symbol: char,
    pub kind: MarkKind,
    /// The thing's id (a place, NPC, clue, scene, or the map's own);
    /// several starts of one side share a mark, their ids comma-joined.
    pub id: String,
    pub name: String,
    pub cells: Vec<Cell>,
    /// Who the map's layer shows it to; `None` for the preparation's
    /// marks, which reach players only through the GM's reveals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<Visibility>,
    /// More about it: a pin's scenes, the scene an NPC, clue or fight is
    /// for, an exit's destination map, a door's state, a prop's kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Symbols for the preparation's marks, then for the map's things.
const STORY_SYMBOLS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ123456789";
const MAP_SYMBOLS: &str = "abcdefghijklmnopqrstuvwxyz!$%&*+?@^";
/// Shared by every mark once the symbols run out.
const SPILL: char = '¤';

struct Symbols {
    used: BTreeSet<char>,
}

impl Symbols {
    fn next(&mut self, pool: &str) -> char {
        let found = pool.chars().find(|c| !self.used.contains(c));
        match found {
            Some(c) => {
                self.used.insert(c);
                c
            }
            None => SPILL,
        }
    }
}

fn side_name(side: Option<Side>) -> &'static str {
    match side {
        Some(Side::Party) => "party",
        Some(Side::Foes) => "foes",
        Some(Side::Neutral) => "neutral",
        None => "",
    }
}

/// `map` as text, with `pins` and `cast` (the preparation's marks on
/// it), in `zone` — by default the whole map, cut to [`MAX_W`] ×
/// [`MAX_H`].
#[must_use]
pub fn sketch(map: &Map, pins: &[Pin], cast: &Cast, zone: Option<Zone>) -> Sketch {
    let whole = Zone {
        x: 0,
        y: 0,
        w: map.grid.width(),
        h: map.grid.height(),
    };
    let zone = clamp(map, zone.unwrap_or(whole));
    let mut symbols = Symbols {
        used: map.grid.legend().map(|(g, _)| g).collect(),
    };
    symbols.used.insert(' ');
    let mut legend: Vec<Mark> = Vec::new();
    let mut push = |kind: MarkKind,
                    id: String,
                    name: String,
                    cells: Vec<Cell>,
                    layer: Option<Visibility>,
                    note: Option<String>,
                    symbols: &mut Symbols| {
        let cells: Vec<Cell> = cells.into_iter().filter(|c| zone.contains(*c)).collect();
        if cells.is_empty() {
            return;
        }
        let story = matches!(
            kind,
            MarkKind::Pin | MarkKind::Npc | MarkKind::Clue | MarkKind::Fight
        );
        let symbol = symbols.next(if story { STORY_SYMBOLS } else { MAP_SYMBOLS });
        legend.push(Mark {
            symbol,
            kind,
            id,
            name,
            cells,
            layer,
            note,
        });
    };
    for p in pins {
        let scenes: Vec<&str> = p.scenes.iter().map(|s| s.id.as_str()).collect();
        push(
            MarkKind::Pin,
            p.location.clone(),
            p.name.clone(),
            vec![p.at],
            None,
            Some(scenes.join(", ")).filter(|s| !s.is_empty()),
            &mut symbols,
        );
    }
    for n in &cast.npcs {
        push(
            MarkKind::Npc,
            n.npc.clone(),
            n.name.clone(),
            vec![n.at],
            None,
            n.scene.clone(),
            &mut symbols,
        );
    }
    for c in &cast.clues {
        push(
            MarkKind::Clue,
            c.clue.clone(),
            c.text.clone(),
            vec![c.at],
            None,
            Some(c.scene.clone()),
            &mut symbols,
        );
    }
    for f in &cast.fights {
        push(
            MarkKind::Fight,
            f.scene.clone(),
            f.title.clone(),
            vec![f.at],
            None,
            None,
            &mut symbols,
        );
    }
    let vis = |layer: &str| Some(map.visibility(layer));
    for e in &map.exits {
        push(
            MarkKind::Exit,
            e.id.clone(),
            e.label.clone().unwrap_or_default(),
            e.cells.clone(),
            vis(&e.layer),
            Some(e.to.clone()),
            &mut symbols,
        );
    }
    for d in &map.doors {
        let state = serde_json::to_value(d.state)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string));
        push(
            MarkKind::Door,
            d.id.clone(),
            d.label.clone().unwrap_or_default(),
            vec![d.at],
            vis(&d.layer),
            state,
            &mut symbols,
        );
    }
    for p in &map.props {
        push(
            MarkKind::Prop,
            p.id.clone(),
            p.label.clone().unwrap_or_default(),
            p.cells().collect(),
            vis(&p.layer),
            Some(p.kind.clone()),
            &mut symbols,
        );
    }
    for o in &map.objects {
        push(
            MarkKind::Object,
            o.id.clone(),
            o.label.clone().unwrap_or_default(),
            vec![o.at],
            vis(&o.layer),
            Some(o.kind.clone()),
            &mut symbols,
        );
    }
    // Starts without an entity are grouped by side and layer: a party of
    // six is one mark, not six.
    let mut groups: BTreeMap<(&str, &str), Vec<&crate::maps::Start>> = BTreeMap::new();
    for s in &map.starts {
        if let Some(entity) = &s.entity {
            push(
                MarkKind::Start,
                s.id.clone(),
                entity.clone(),
                vec![s.at],
                vis(&s.layer),
                Some(side_name(s.side).to_string()).filter(|s| !s.is_empty()),
                &mut symbols,
            );
        } else {
            groups
                .entry((side_name(s.side), s.layer.as_str()))
                .or_default()
                .push(s);
        }
    }
    for ((side, layer), starts) in groups {
        let ids: Vec<&str> = starts.iter().map(|s| s.id.as_str()).collect();
        push(
            MarkKind::Start,
            ids.join(","),
            String::new(),
            starts.iter().map(|s| s.at).collect(),
            vis(layer),
            Some(side.to_string()).filter(|s| !s.is_empty()),
            &mut symbols,
        );
    }
    for l in &map.labels {
        push(
            MarkKind::Label,
            String::new(),
            l.text.clone(),
            vec![l.at],
            vis(&l.layer),
            None,
            &mut symbols,
        );
    }
    // The preparation's marks win a shared cell: they come first in the
    // legend, so they are drawn last.
    let mut over: HashMap<Cell, char> = HashMap::new();
    for m in legend.iter().rev() {
        for c in &m.cells {
            over.insert(*c, m.symbol);
        }
    }
    let hex = map.geometry() == Geometry::Hex;
    let mut tens = String::from("    ");
    let mut units = String::from("    ");
    let mut used = BTreeSet::new();
    let mut rows = Vec::with_capacity(zone.h as usize + 2);
    for dx in 0..zone.w {
        #[allow(clippy::cast_possible_wrap)]
        let x = zone.x + dx as i32;
        let t = if x % 10 == 0 || dx == 0 {
            char::from_digit(u32::try_from((x / 10) % 10).unwrap_or(0), 10).unwrap_or(' ')
        } else {
            ' '
        };
        let u = char::from_digit(u32::try_from(x % 10).unwrap_or(0), 10).unwrap_or(' ');
        tens.push(t);
        units.push(u);
        if hex {
            tens.push(' ');
            units.push(' ');
        }
    }
    rows.push(tens.trim_end().to_string());
    rows.push(units.trim_end().to_string());
    for dy in 0..zone.h {
        #[allow(clippy::cast_possible_wrap)]
        let y = zone.y + dy as i32;
        let mut line = format!("{y:>3} ");
        if hex && y % 2 == 1 {
            line.push(' ');
        }
        for dx in 0..zone.w {
            #[allow(clippy::cast_possible_wrap)]
            let c = Cell::new(zone.x + dx as i32, y);
            let glyph = map.grid.glyph(c).unwrap_or(' ');
            used.insert(glyph);
            line.push(over.get(&c).copied().unwrap_or(glyph));
            if hex {
                line.push(' ');
            }
        }
        rows.push(line.trim_end().to_string());
    }
    let terrain = map
        .grid
        .legend()
        .filter(|(g, _)| used.contains(g))
        .map(|(glyph, k)| Terrain {
            glyph,
            terrain: k.terrain.clone(),
            wall: k.wall,
            void: k.void,
            water: k.water,
            difficult: k.difficult,
            elevation: k.elevation,
        })
        .collect();
    Sketch {
        map: map.id.clone(),
        name: map.name.clone(),
        scale: map.scale,
        width: map.grid.width(),
        height: map.grid.height(),
        cut: zone != whole,
        zone,
        rows,
        terrain,
        legend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::cast::NpcMark;

    const ROOM: &str = r##"
version: 1
id: taverne
name: La taverne
scale: place
theme: port-1718
grid:
  legend:
    "X": { terrain: pierre, wall: true }
    ".": { terrain: plancher }
    "~": { terrain: eau, water: deep }
  rows:
    - "XXXXX"
    - "X...X"
    - "X..~X"
    - "XXXXX"
props:
  - { id: comptoir, kind: comptoir, label: Le comptoir, at: [1, 1], size: [2, 1], blocks_movement: true }
labels:
  - { text: La salle, at: [3, 1] }
"##;

    fn room() -> Map {
        Map::from_yaml(ROOM).unwrap()
    }

    #[test]
    fn draws_the_grid_with_numbered_axes_and_a_symbol_per_thing() {
        let map = room();
        let cast = Cast {
            npcs: vec![NpcMark {
                npc: "pnj_gwen".into(),
                name: "Gwen".into(),
                index: 0,
                at: Cell::new(2, 2),
                scene: None,
            }],
            ..Cast::default()
        };
        let s = sketch(&map, &[], &cast, None);
        assert_eq!(s.rows[1], "    01234");
        // Gwen first (story marks first), then the counter over two
        // cells, then the label; the water keeps its own glyph.
        assert_eq!(s.rows[3], "  1 XaabX");
        assert_eq!(s.rows[4], "  2 X.A~X");
        assert_eq!(s.legend[0].symbol, 'A');
        assert_eq!(s.legend[1].cells, vec![Cell::new(1, 1), Cell::new(2, 1)]);
        assert!(
            s.terrain
                .iter()
                .any(|t| t.glyph == '~' && t.water == Water::Deep)
        );
        assert!(!s.cut);
    }

    #[test]
    fn a_zone_draws_only_its_cells_and_their_things() {
        let map = room();
        let s = sketch(
            &map,
            &[],
            &Cast::default(),
            Some(Zone {
                x: 3,
                y: 1,
                w: 9,
                h: 9,
            }),
        );
        assert_eq!(
            s.zone,
            Zone {
                x: 3,
                y: 1,
                w: 2,
                h: 3
            }
        );
        assert!(s.cut);
        assert_eq!(s.rows[0], "    0");
        assert_eq!(s.rows[1], "    34");
        // The counter is outside the zone: only the label is listed.
        assert_eq!(s.legend.len(), 1);
        assert_eq!(s.legend[0].kind, MarkKind::Label);
    }

    #[test]
    fn a_hex_map_takes_two_characters_a_cell_and_shifts_odd_rows() {
        let map = Map::from_yaml(
            r#"
version: 1
id: region
name: Région
scale: world
theme: cotes-1718
grid:
  legend:
    "~": { terrain: mer }
    "o": { terrain: terre }
  rows:
    - "~oo"
    - "oo~"
"#,
        )
        .unwrap();
        let s = sketch(&map, &[], &Cast::default(), None);
        assert_eq!(s.rows[1], "    0 1 2");
        assert_eq!(s.rows[2], "  0 ~ o o");
        assert_eq!(s.rows[3], "  1  o o ~");
    }
}
