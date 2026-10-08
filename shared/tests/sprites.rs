//! The layered sprites of the two witness worlds (characters/render-layered-sprite).
//!
//! Golden images: every look of `content/sprites/looks/<world>.yaml` is
//! rendered and compared, pixel for pixel, with
//! `shared/tests/golden/<world>/<id>.png`. After an intended change of a
//! pack or of the renderer, regenerate them and look at the result:
//!
//! ```sh
//! BLESS=1 cargo test -p promptus_shared --test sprites
//! ```
//!
//! Blessing also writes `planche.png` per world: every look, enlarged,
//! and `marche.png`: every look at rest facing south, east, north and
//! west (characters/walk-in-four-directions). Each look's
//! `<id>.marche.png` holds its four sheets, one row per direction.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use promptus_shared::maps::Cell;
use promptus_shared::sprite::{
    CharacterLook, Direction, Frame, Image, LookBook, Packs, SpriteError, Worn, compose,
    compose_frame, render, render_sheet,
};

/// The order of the rows of a `.marche.png` and of the columns of `marche.png`.
const TURN: [Direction; 4] = [
    Direction::South,
    Direction::East,
    Direction::North,
    Direction::West,
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn packs() -> Packs {
    let dir = root().join("content/sprites");
    let mut texts = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let pack = entry.unwrap().path().join("pack.yaml");
        if pack.is_file() {
            texts.push(read(&pack));
        }
    }
    Packs::from_yaml(texts.iter().map(String::as_str)).unwrap()
}

fn books() -> Vec<LookBook> {
    ["corsaires", "brasier"]
        .iter()
        .map(|w| {
            LookBook::from_yaml(&read(
                &root().join(format!("content/sprites/looks/{w}.yaml")),
            ))
            .unwrap()
        })
        .collect()
}

fn decode(bytes: &[u8]) -> Image {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    Image {
        width: info.width,
        height: info.height,
        rgba: buf,
    }
}

fn look<'a>(books: &'a [LookBook], id: &str) -> &'a CharacterLook {
    books
        .iter()
        .flat_map(|b| b.party.iter().chain(&b.foes))
        .find(|n| n.id == id)
        .map(|n| &n.look)
        .unwrap()
}

#[test]
fn every_look_matches_its_golden_image() {
    let packs = packs();
    let bless = std::env::var_os("BLESS").is_some();
    let mut failures = Vec::new();
    for book in books() {
        let dir = root().join("shared/tests/golden").join(&book.world);
        let mut drawn = Vec::new();
        let mut turned = Vec::new();
        let sides = [
            (&book.party, Direction::East),
            (&book.foes, Direction::West),
        ];
        for (looks, facing) in sides {
            for named in looks {
                let image = render(&packs, &named.look, facing)
                    .unwrap_or_else(|e| panic!("{} {}: {e}", book.world, named.id));
                let path = dir.join(format!("{}.png", named.id));
                if bless {
                    fs::create_dir_all(&dir).unwrap();
                    fs::write(&path, image.to_png()).unwrap();
                } else {
                    match fs::read(&path) {
                        Ok(bytes) if decode(&bytes) == image => {}
                        Ok(_) => failures.push(format!("{} differs", path.display())),
                        Err(_) => failures.push(format!("{} is missing", path.display())),
                    }
                }
                drawn.push(image);
                // Four sheets, one row per direction.
                let rows: Vec<Image> = TURN
                    .iter()
                    .map(|&d| {
                        render_sheet(&packs, &named.look, d)
                            .unwrap_or_else(|e| panic!("{} {} {d:?}: {e}", book.world, named.id))
                    })
                    .collect();
                let walk = stack(&rows);
                let path = dir.join(format!("{}.marche.png", named.id));
                if bless {
                    fs::write(&path, walk.to_png()).unwrap();
                } else {
                    match fs::read(&path) {
                        Ok(bytes) if decode(&bytes) == walk => {}
                        Ok(_) => failures.push(format!("{} differs", path.display())),
                        Err(_) => failures.push(format!("{} is missing", path.display())),
                    }
                }
                for d in TURN {
                    turned.push(render(&packs, &named.look, d).unwrap());
                }
            }
        }
        if bless {
            fs::write(dir.join("planche.png"), sheet(&drawn, 6).to_png()).unwrap();
            fs::write(dir.join("marche.png"), sheet(&turned, 4).to_png()).unwrap();
        }
    }
    assert!(
        failures.is_empty(),
        "golden sprites (BLESS=1 to regenerate, then look at them):\n{}",
        failures.join("\n")
    );
}

/// Images of one width, one above the other.
fn stack(images: &[Image]) -> Image {
    Image {
        width: images[0].width,
        height: images.iter().map(|i| i.height).sum(),
        rgba: images.iter().flat_map(|i| i.rgba.iter().copied()).collect(),
    }
}

/// The looks side by side, enlarged, on the table's dark background.
fn sheet(images: &[Image], scale: u32) -> Image {
    let (w, h) = (images[0].width, images[0].height);
    let gap = 2;
    let width = (w + gap) * scale * u32::try_from(images.len()).unwrap() + gap * scale;
    let height = (h + 2 * gap) * scale;
    let mut rgba = [0x14, 0x14, 0x14, 0xFF].repeat((width * height) as usize);
    for (n, img) in (0u32..).zip(images) {
        let left = gap * scale + n * (w + gap) * scale;
        for y in 0..height {
            for x in 0..w * scale {
                let sy = y / scale;
                if sy < gap || sy >= gap + h {
                    continue;
                }
                let p = img.pixel(x / scale, sy - gap);
                let i = ((y * width + left + x) * 4) as usize;
                let a = u32::from(p[3]);
                for c in 0..3 {
                    let under = u32::from(rgba[i + c]);
                    rgba[i + c] =
                        u8::try_from((u32::from(p[c]) * a + under * (255 - a)) / 255).unwrap();
                }
            }
        }
    }
    Image {
        width,
        height,
        rgba,
    }
}

/// The ids of the looks are the campaign's: every party slot and every
/// adversary has a look, and no look names someone the campaign lacks.
#[test]
fn looks_cover_the_party_and_the_adversaries_of_each_campaign() {
    for book in books() {
        let campaign: serde_yaml_ng::Value = serde_yaml_ng::from_str(&read(
            &root().join(format!("content/campaigns/{}/campagne.yaml", book.world)),
        ))
        .unwrap();
        assert_eq!(campaign["id"].as_str(), Some(book.campaign.as_str()));
        let ids = |key: &str| -> BTreeSet<String> {
            campaign[key]
                .as_sequence()
                .map(|s| {
                    s.iter()
                        .filter_map(|e| e["id"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        };
        let party: BTreeSet<String> = book.party.iter().map(|n| n.id.clone()).collect();
        assert_eq!(party, ids("party"), "{}: party slots", book.world);
        let foes: BTreeSet<String> = book.foes.iter().map(|n| n.id.clone()).collect();
        let adversaries = ids("adversaries");
        assert!(
            adversaries.is_subset(&foes),
            "{}: adversaries without a look: {:?}",
            book.world,
            adversaries.difference(&foes).collect::<Vec<_>>()
        );
        let known: BTreeSet<String> = adversaries.union(&ids("npcs")).cloned().collect();
        assert!(
            foes.is_subset(&known),
            "{}: looks for no one: {:?}",
            book.world,
            foes.difference(&known).collect::<Vec<_>>()
        );
    }
}

/// Distinct tones of an image's opaque pixels.
fn tones(img: &Image, other: &Image) -> (BTreeSet<[u8; 4]>, BTreeSet<[u8; 4]>) {
    let mut a = BTreeSet::new();
    let mut b = BTreeSet::new();
    for (p, q) in img.rgba.chunks(4).zip(other.rgba.chunks(4)) {
        if p != q {
            a.insert([p[0], p[1], p[2], p[3]]);
            b.insert([q[0], q[1], q[2], q[3]]);
        }
    }
    (a, b)
}

#[test]
fn a_dye_recolours_only_its_piece_in_three_tones() {
    use promptus_shared::sprite::colour::{DARK, LIGHT, Rgb};
    let packs = packs();
    let books = books();
    let red = look(&books, "pj_bretteur").clone();
    let mut blue = red.clone();
    blue.outfit = Some(Worn {
        piece: "gilet".into(),
        dye: Some("bleu".into()),
        accent: Some("noir".into()),
    });
    let a = render(&packs, &red, Direction::East).unwrap();
    let b = render(&packs, &blue, Direction::East).unwrap();
    let three = |hex: &str| -> BTreeSet<[u8; 4]> {
        let c = Rgb::parse(hex).unwrap();
        [c, c.adjust(LIGHT), c.adjust(DARK)]
            .iter()
            .map(|c| [c.0, c.1, c.2, 0xFF])
            .collect()
    };
    let (before, after) = tones(&a, &b);
    assert!(!after.is_empty(), "the dye changed nothing");
    assert!(before.is_subset(&three("#A3362E")), "{before:?}");
    assert!(after.is_subset(&three("#4F6D9A")), "{after:?}");
    // A raw colour works as well as a swatch id.
    let mut raw = blue.clone();
    raw.outfit.as_mut().unwrap().dye = Some("#4f6d9a".into());
    assert_eq!(render(&packs, &raw, Direction::East).unwrap(), b);
}

#[test]
fn the_outline_never_covers_the_face() {
    let packs = packs();
    let books = books();
    // A long beard and a hat around the face, a weapon in front of it.
    for id in [
        "pj_quartier_maitre",
        "pj_bretteur",
        "pj_canonnier",
        "pj_xenologue",
    ] {
        let c = compose(&packs, look(&books, id), Direction::East).unwrap();
        let face = c.face.iter().filter(|&&f| f).count();
        assert!(face > 40, "{id}: the face is {face} cells");
        assert!(c.outline.iter().filter(|&&o| o).count() > 40);
        let covered = c
            .face
            .iter()
            .zip(&c.outline)
            .filter(|&(&f, &o)| f && o)
            .count();
        assert_eq!(covered, 0, "{id}: outline over the face");
    }
}

#[test]
fn the_outline_surrounds_the_silhouette() {
    let packs = packs();
    let books = books();
    let img = render(&packs, look(&books, "pj_vigie"), Direction::East).unwrap();
    // The outline ink of the pack.
    let ink = [0x1B, 0x14, 0x26, 0xFF];
    // Walking in from each side of every row, the first opaque pixel is
    // either outline ink or the shadow: never a coloured pixel.
    for y in 0..img.height {
        for xs in [
            (0..img.width).collect::<Vec<_>>(),
            (0..img.width).rev().collect(),
        ] {
            if let Some(x) = xs.into_iter().find(|&x| img.pixel(x, y)[3] == 0xFF) {
                assert_eq!(img.pixel(x, y), ink, "row {y}, column {x}");
            }
        }
    }
}

#[test]
fn west_is_the_mirror_of_east() {
    let packs = packs();
    let books = books();
    let l = look(&books, "gueule-rouge");
    let east = render(&packs, l, Direction::East).unwrap();
    let west = render(&packs, l, Direction::West).unwrap();
    assert_ne!(east, west);
    for y in 0..east.height {
        for x in 0..east.width {
            assert_eq!(east.pixel(x, y), west.pixel(east.width - 1 - x, y));
        }
    }
}

#[test]
fn a_drop_shadow_lies_under_the_feet() {
    let packs = packs();
    let books = books();
    let img = render(&packs, look(&books, "pj_navigateur"), Direction::East).unwrap();
    let last = img.height - 1;
    let shadow = (0..img.width)
        .filter(|&x| {
            let p = img.pixel(x, last);
            p[3] > 0 && p[3] < 0xFF
        })
        .count();
    assert!(shadow >= 2, "no shadow on the bottom row");
}

#[test]
fn an_unknown_piece_or_colour_is_a_typed_error() {
    let packs = packs();
    let books = books();
    let mut l = look(&books, "pj_bretteur").clone();
    l.weapon = Some(Worn::plain("trident"));
    assert_eq!(
        render(&packs, &l, Direction::East),
        Err(SpriteError::UnknownPiece {
            pack: "marins-1718".into(),
            slot: "weapon",
            piece: "trident".into(),
        })
    );
    // A piece of the other pack is unknown here too.
    l.weapon = Some(Worn::plain("pistolet-laser"));
    assert!(matches!(
        render(&packs, &l, Direction::East),
        Err(SpriteError::UnknownPiece { .. })
    ));
    let mut l = look(&books, "pj_bretteur").clone();
    l.skin = "vert".into();
    assert_eq!(
        render(&packs, &l, Direction::East),
        Err(SpriteError::UnknownColour {
            palette: "skin",
            value: "vert".into(),
        })
    );
    let mut l = look(&books, "pj_bretteur").clone();
    l.pack = "zombies".into();
    assert_eq!(
        render(&packs, &l, Direction::East),
        Err(SpriteError::UnknownPack("zombies".into()))
    );
}

#[test]
fn a_description_reads_from_yaml_and_json_alike() {
    let yaml = "pack: marins-1718\nbody: svelte\nskin: hale\nhair: { colour: brun }\nweapon: sabre\n\
                outfit: { piece: chemise, dye: rouge }\n";
    let from_yaml: CharacterLook = serde_yaml_ng::from_str(yaml).unwrap();
    let json = serde_json::to_string(&from_yaml).unwrap();
    let from_json: CharacterLook = serde_json::from_str(&json).unwrap();
    assert_eq!(from_yaml, from_json);
    assert_eq!(from_json.weapon, Some(Worn::plain("sabre")));
    // A field the format does not know is refused, not ignored.
    assert!(serde_yaml_ng::from_str::<CharacterLook>(&format!("{yaml}cape: rouge\n")).is_err());
}

#[test]
fn a_broken_pack_is_refused_with_its_reason() {
    let text = read(&root().join("content/sprites/marins-1718/pack.yaml"));
    // A grid character missing from the legend.
    let bad = text.replacen("- \"ss.ss\"", "- \"sZ.ss\"", 1);
    assert_ne!(bad, text, "the fixture row moved");
    let err = Packs::from_yaml([bad.as_str()]).unwrap_err().to_string();
    assert!(err.contains("not in the legend"), "{err}");
    // A dyed piece without its own colour.
    let bad = text.replacen(
        "      dye: noir\n      accent: or\n",
        "      accent: or\n",
        1,
    );
    assert_ne!(bad, text, "the fixture piece moved");
    let err = Packs::from_yaml([bad.as_str()]).unwrap_err().to_string();
    assert!(err.contains("no default dye"), "{err}");
    // A piece that does not say how it looks from the back.
    let bad = text.replacen("        north:\n", "        west:\n", 1);
    assert_ne!(bad, text, "the fixture frame moved");
    let err = Packs::from_yaml([bad.as_str()]).unwrap_err().to_string();
    assert!(err.contains("no north frame"), "{err}");
}

// --- characters/walk-in-four-directions -----------------------------------

/// Opaque pixels of an image, by row.
fn opaque_rows(img: &Image) -> Vec<Vec<u32>> {
    (0..img.height)
        .map(|y| {
            (0..img.width)
                .filter(|&x| img.pixel(x, y)[3] == 0xFF)
                .collect()
        })
        .collect()
}

#[test]
fn every_look_turns_four_ways_without_covering_its_face() {
    let packs = packs();
    for book in books() {
        for named in book.party.iter().chain(&book.foes) {
            let mut seen = Vec::new();
            for d in TURN {
                let c = compose(&packs, &named.look, d).unwrap();
                let covered = c
                    .face
                    .iter()
                    .zip(&c.outline)
                    .filter(|&(&f, &o)| f && o)
                    .count();
                assert_eq!(covered, 0, "{} {d:?}: outline over the face", named.id);
                seen.push(render(&packs, &named.look, d).unwrap());
            }
            // The front, the back and the profile are three drawings.
            assert_ne!(seen[0], seen[1], "{}: front = profile", named.id);
            assert_ne!(seen[0], seen[2], "{}: front = back", named.id);
            assert_ne!(seen[1], seen[2], "{}: profile = back", named.id);
        }
    }
}

#[test]
fn the_front_shows_the_eyes_and_the_back_hides_them() {
    let packs = packs();
    let books = books();
    let eye = [0x1B, 0x14, 0x26, 0xFF];
    // A bare head: the eyes are the only ink inside the face.
    let mut l = look(&books, "pj_vigie").clone();
    l.headwear = None;
    l.beard = None;
    for (d, eyes) in [(Direction::South, 2), (Direction::North, 0)] {
        let c = compose(&packs, &l, d).unwrap();
        let img = c.to_image();
        let inked = (0..img.height)
            .flat_map(|y| (0..img.width).map(move |x| (x, y)))
            .filter(|&(x, y)| c.face[(y * c.width + x) as usize] && img.pixel(x, y) == eye)
            .count();
        assert_eq!(inked, eyes, "{d:?}");
    }
}

#[test]
fn a_sheet_holds_rest_breath_and_two_steps() {
    let packs = packs();
    let books = books();
    let l = look(&books, "pj_canonnier");
    for d in TURN {
        let sheet = render_sheet(&packs, l, d).unwrap();
        let one = render(&packs, l, d).unwrap();
        assert_eq!((sheet.width, sheet.height), (4 * one.width, one.height));
        for (i, frame) in Frame::SHEET.iter().enumerate() {
            let drawn = compose_frame(&packs, l, d, *frame).unwrap().to_image();
            let left = u32::try_from(i).unwrap() * one.width;
            for y in 0..one.height {
                for x in 0..one.width {
                    assert_eq!(
                        sheet.pixel(left + x, y),
                        drawn.pixel(x, y),
                        "{d:?} {frame:?}"
                    );
                }
            }
        }
        assert_eq!(
            compose_frame(&packs, l, d, Frame::Rest).unwrap().to_image(),
            one
        );
    }
}

#[test]
fn breathing_lowers_the_body_and_keeps_the_feet() {
    let packs = packs();
    let books = books();
    let l = look(&books, "pj_bretteur");
    for d in TURN {
        let rest = opaque_rows(&compose_frame(&packs, l, d, Frame::Rest).unwrap().to_image());
        let breath = opaque_rows(
            &compose_frame(&packs, l, d, Frame::Breath)
                .unwrap()
                .to_image(),
        );
        let top = |rows: &[Vec<u32>]| rows.iter().position(|r| !r.is_empty()).unwrap();
        assert_eq!(top(&breath), top(&rest) + 1, "{d:?}: the head did not drop");
        let last = rest.len() - 2;
        assert_eq!(breath[last], rest[last], "{d:?}: the feet moved");
    }
}

#[test]
fn a_step_spreads_the_legs_in_profile_and_lifts_a_foot_from_the_front() {
    let packs = packs();
    let books = books();
    let l = look(&books, "pj_navigateur");
    let width =
        |rows: &[Vec<u32>], y: usize| rows[y].last().unwrap_or(&0) - rows[y].first().unwrap_or(&0);
    // Profile: the soles are further apart in the stride.
    let rest = opaque_rows(
        &compose_frame(&packs, l, Direction::East, Frame::Rest)
            .unwrap()
            .to_image(),
    );
    let step = opaque_rows(
        &compose_frame(&packs, l, Direction::East, Frame::StepA)
            .unwrap()
            .to_image(),
    );
    let sole = rest.len() - 2;
    assert!(width(&step, sole) > width(&rest, sole) + 2, "no stride");
    // Front: one foot leaves the ground in each step, not the same one.
    let soles = |f: Frame| {
        opaque_rows(
            &compose_frame(&packs, l, Direction::South, f)
                .unwrap()
                .to_image(),
        )[sole]
            .clone()
    };
    let (a, b, still) = (soles(Frame::StepA), soles(Frame::StepB), soles(Frame::Rest));
    assert!(
        a.len() < still.len() && b.len() < still.len(),
        "no foot lifted"
    );
    let centre = |r: &[u32]| r.iter().sum::<u32>() / u32::try_from(r.len()).unwrap();
    assert!(centre(&a) > centre(&b), "both steps lift the same foot");
}

#[test]
fn a_character_turns_toward_where_it_goes() {
    let c = |x, y| Cell::new(x, y);
    assert_eq!(Direction::toward(c(3, 3), c(5, 3)), Some(Direction::East));
    assert_eq!(Direction::toward(c(3, 3), c(1, 4)), Some(Direction::West));
    assert_eq!(Direction::toward(c(3, 3), c(3, 1)), Some(Direction::North));
    assert_eq!(Direction::toward(c(3, 3), c(4, 6)), Some(Direction::South));
    // A perfect diagonal keeps the profile.
    assert_eq!(Direction::toward(c(3, 3), c(2, 2)), Some(Direction::West));
    assert_eq!(Direction::toward(c(3, 3), c(3, 3)), None);
}
