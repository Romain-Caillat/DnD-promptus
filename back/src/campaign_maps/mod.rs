//! The maps a campaign owns (migration `019_campaign_maps.sql`), next to
//! the worlds' maps compiled into the server (`content::maps`).
//!
//! - maps/edit-map-gm: the GM creates a map (blank, or a copy of one of
//!   the world's), paints and places in the editor and saves the whole
//!   map, checked by `Map::validate`;
//! - maps/import-image-map: a Universal VTT file (`maps::from_uvtt`) or a
//!   plain image aligned on a grid becomes a map whose image is only the
//!   backdrop; the GM traces or touches up its walls in the editor;
//! - maps/generate-map-llm ([`generate`]): the model proposes a map for a
//!   scene.
//!
//! A map reaches the table only once the GM validated it; saving it
//! again withdraws the validation (the GM has the last word on what is
//! shown, `MEMORY.md` §3).

pub mod generate;

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use promptus_shared::maps::{
    Ambience, BASE_LAYER, Backdrop, CellKind, FORMAT_VERSION, Grid, Issue, Layer, Map, MapError,
    Scale, Visibility, Water, from_uvtt,
};
use promptus_shared::story::Campaign;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::live::{self, Topic};

/// Largest imported image, in bytes.
pub const MAX_BACKDROP_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Editor,
    Generated,
    Imported,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Self::Editor => "editor",
            Self::Generated => "generated",
            Self::Imported => "imported",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "generated" => Self::Generated,
            "imported" => Self::Imported,
            _ => Self::Editor,
        }
    }
}

/// One map of the campaign, as the GM's screens read it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignMap {
    pub map: Map,
    pub source: Source,
    pub node: Option<String>,
    /// It has an imported image behind its grid.
    pub backdrop: bool,
    pub validated_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

type Row = (
    Json<Map>,
    String,
    Option<String>,
    bool,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

const COLUMNS: &str = "map, source, node, backdrop IS NOT NULL, validated_at, updated_at";

fn campaign_map((Json(map), source, node, backdrop, validated_at, updated_at): Row) -> CampaignMap {
    CampaignMap {
        map,
        source: Source::parse(&source),
        node,
        backdrop,
        validated_at,
        updated_at,
    }
}

/// Every map of `campaign`, newest change first.
///
/// # Errors
///
/// A database error.
pub async fn all(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<CampaignMap>, AppError> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM campaign_maps WHERE campaign_id = $1 ORDER BY updated_at DESC"
    ))
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(campaign_map).collect())
}

async fn one(db: impl PgExecutor<'_>, campaign: Uuid, id: &str) -> Result<CampaignMap, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM campaign_maps WHERE campaign_id = $1 AND id = $2"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(campaign_map)
        .ok_or(AppError::NotFound("NO_SUCH_MAP"))
}

/// The map `id` the table may be shown: a validated map of the campaign,
/// else one of its world's.
///
/// # Errors
///
/// A database error.
pub async fn playable(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    story: &Campaign,
    id: &str,
) -> Result<Option<Map>, AppError> {
    let own: Option<Json<Map>> = sqlx::query_scalar(
        "SELECT map FROM campaign_maps
         WHERE campaign_id = $1 AND id = $2 AND validated_at IS NOT NULL",
    )
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(own
        .map(|Json(m)| m)
        .or_else(|| content::map(story, id).cloned()))
}

/// The validated maps of `campaign`, for the table's map choice.
///
/// # Errors
///
/// A database error.
pub async fn validated(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Map>, AppError> {
    let rows: Vec<Json<Map>> = sqlx::query_scalar(
        "SELECT map FROM campaign_maps
         WHERE campaign_id = $1 AND validated_at IS NOT NULL ORDER BY created_at",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|Json(m)| m).collect())
}

/// The GM's words for what is wrong with a map (French).
fn french(issue: &Issue) -> String {
    match issue {
        Issue::UnsupportedVersion(v) => format!("version de format {v} inconnue"),
        Issue::BadCellSize => "la taille d’une case doit être positive".into(),
        Issue::DuplicateId(id) => format!("l’identifiant « {id} » sert deux fois"),
        Issue::DuplicateLayer(id) => format!("le calque « {id} » est déclaré deux fois"),
        Issue::UnknownLayer { item, layer } => {
            format!("« {item} » est sur le calque « {layer} », qui n’existe pas")
        }
        Issue::OutOfBounds { item, cell } => {
            format!("« {item} » sort de la grille ({}, {})", cell.x, cell.y)
        }
        Issue::DoorNotOnWall { door, cell } => format!(
            "la porte « {door} » ({}, {}) n’est pas sur un mur",
            cell.x, cell.y
        ),
        Issue::StartNotWalkable { start, cell } => format!(
            "le départ « {start} » ({}, {}) n’est pas sur une case praticable",
            cell.x, cell.y
        ),
        Issue::LightRadius(id) => {
            format!(
                "la lumière « {id} » : la pénombre doit porter au moins aussi loin que la lumière vive"
            )
        }
        Issue::EmptyExit(id) => format!("la sortie « {id} » n’a aucune case"),
        Issue::EmptyProp(id) => format!("le décor « {id} » n’a pas de taille"),
        Issue::DoorOnHex(id) => format!("la porte « {id} » est sur une carte en hexagones"),
        Issue::BadBackdrop => "l’alignement de l’image de fond est invalide".into(),
    }
}

/// A map error as the GM reads it.
#[must_use]
pub fn invalid(e: &MapError) -> AppError {
    let detail = match e {
        MapError::Invalid(issues) => issues.iter().map(french).collect::<Vec<_>>().join(" ; "),
        MapError::Uvtt(why) => why.clone(),
        other => other.to_string(),
    };
    AppError::Invalid {
        code: "MAP_INVALID",
        detail,
    }
}

/// The tileset a new map of `story` is drawn with, and its first floor.
fn look(story: &Campaign) -> (String, String) {
    let tileset = content::theme(story).and_then(|t| t.tilesets.first());
    (
        tileset.map_or_else(|| "defaut".to_string(), |t| t.id.clone()),
        tileset
            .and_then(|t| t.materials.keys().next().cloned())
            .unwrap_or_else(|| "sol".to_string()),
    )
}

/// The layers a new map starts with: the map, and the GM's secrets.
fn layers() -> Vec<Layer> {
    vec![
        Layer {
            id: BASE_LAYER.into(),
            name: "Carte".into(),
            visibility: Visibility::All,
        },
        Layer {
            id: "secrets".into(),
            name: "Secrets".into(),
            visibility: Visibility::Gm,
        },
    ]
}

/// A free id for a map named `name` in `campaign`.
async fn fresh_id(
    db: &PgPool,
    campaign: Uuid,
    story: &Campaign,
    name: &str,
) -> Result<String, AppError> {
    let base = promptus_shared::story::v1::slug(name);
    let base = if base.is_empty() {
        "carte".to_string()
    } else {
        base
    };
    let taken: Vec<String> =
        sqlx::query_scalar("SELECT id FROM campaign_maps WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_all(db)
            .await?;
    let used = |id: &str| taken.iter().any(|t| t == id) || content::map(story, id).is_some();
    let mut id = base.clone();
    let mut n = 2;
    while used(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    Ok(id)
}

fn clean_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("NAME_REQUIRED"));
    }
    if name.chars().count() > 80 {
        return Err(AppError::BadRequest("TEXT_TOO_LONG"));
    }
    Ok(name.to_string())
}

/// Store a new map of `campaign`, unvalidated.
#[allow(clippy::too_many_arguments)]
async fn insert(
    pool: &PgPool,
    campaign: Uuid,
    map: &Map,
    source: Source,
    node: Option<&str>,
    backdrop: Option<(Vec<u8>, String)>,
) -> Result<CampaignMap, AppError> {
    map.validate().map_err(|e| invalid(&e))?;
    let (bytes, mime) = backdrop.unzip();
    let mut tx = pool.begin().await?;
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO campaign_maps (campaign_id, id, map, source, node, backdrop, backdrop_mime)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(&map.id)
    .bind(Json(map))
    .bind(source.as_str())
    .bind(node)
    .bind(bytes)
    .bind(mime)
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(campaign_map(row))
}

/// What the GM's maps screen lists.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub maps: Vec<CampaignMap>,
    /// The world's maps, to copy from.
    pub world: Vec<WorldMap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldMap {
    pub id: String,
    pub name: String,
}

/// # Errors
///
/// 404; a database error.
pub async fn listing(pool: &PgPool, gm: &CurrentGm, campaign: Uuid) -> Result<Listing, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    Ok(Listing {
        maps: all(pool, campaign).await?,
        world: content::maps(&row.story)
            .map(|m| WorldMap {
                id: m.id.clone(),
                name: m.name.clone(),
            })
            .collect(),
    })
}

/// # Errors
///
/// 404, `NO_SUCH_MAP`.
pub async fn get(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
) -> Result<CampaignMap, AppError> {
    owned_by(campaigns::find(pool, campaign).await?, gm)?;
    one(pool, campaign, id).await
}

/// A blank map, or a copy of one of the world's.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewMap {
    pub name: String,
    #[serde(default)]
    pub copy: Option<String>,
    #[serde(default = "default_side")]
    pub width: u32,
    #[serde(default = "default_side")]
    pub height: u32,
    #[serde(default)]
    pub node: Option<String>,
}

const fn default_side() -> u32 {
    16
}

/// # Errors
///
/// 404; 400 `NAME_REQUIRED`, `TEXT_TOO_LONG`, `UNKNOWN_MAP`, `BAD_SIZE`,
/// `UNKNOWN_NODE`.
pub async fn create(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    new: &NewMap,
) -> Result<CampaignMap, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let name = clean_name(&new.name)?;
    check_node(&row, new.node.as_deref())?;
    let id = fresh_id(pool, campaign, &row.story, &name).await?;
    let map = match &new.copy {
        Some(copy) => {
            let mut m = content::map(&row.story, copy)
                .ok_or(AppError::BadRequest("UNKNOWN_MAP"))?
                .clone();
            m.id = id;
            m.name = name;
            m
        }
        None => blank(&row.story, id, name, new.width, new.height)?,
    };
    insert(
        pool,
        campaign,
        &map,
        Source::Editor,
        new.node.as_deref(),
        None,
    )
    .await
}

fn check_node(row: &CampaignRow, node: Option<&str>) -> Result<(), AppError> {
    match node {
        Some(n) if row.story.node(n).is_none() => Err(AppError::BadRequest("UNKNOWN_NODE")),
        _ => Ok(()),
    }
}

/// A floor of `width` × `height` cells inside a wall.
fn blank(
    story: &Campaign,
    id: String,
    name: String,
    width: u32,
    height: u32,
) -> Result<Map, AppError> {
    if !(3..=64).contains(&width) || !(3..=64).contains(&height) {
        return Err(AppError::BadRequest("BAD_SIZE"));
    }
    let (theme, floor) = look(story);
    let ground = CellKind {
        terrain: floor,
        wall: false,
        water: Water::None,
        difficult: false,
        void: false,
        elevation: 0,
    };
    let legend = BTreeMap::from([
        ('.', ground.clone()),
        (
            '#',
            CellKind {
                wall: true,
                ..ground
            },
        ),
    ]);
    let rows: Vec<String> = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                        '#'
                    } else {
                        '.'
                    }
                })
                .collect()
        })
        .collect();
    Ok(Map {
        version: FORMAT_VERSION,
        id,
        name,
        scale: Scale::Encounter,
        cell_meters: None,
        theme,
        ambience: Ambience::default(),
        backdrop: None,
        gm_notes: None,
        layers: layers(),
        grid: Grid::new(&legend, &rows).map_err(|e| invalid(&e.into()))?,
        doors: vec![],
        props: vec![],
        objects: vec![],
        lights: vec![],
        exits: vec![],
        labels: vec![],
        starts: vec![],
    })
}

/// Save the whole map `id` as the editor sends it. Its id, format
/// version and backdrop image stay the server's; the validation is
/// withdrawn until the GM gives it again.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`; 400 `MAP_INVALID` with the GM's words.
pub async fn save(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
    raw: serde_json::Value,
) -> Result<CampaignMap, AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let before = one(&mut *tx, campaign, id).await?;
    let mut map: Map = serde_json::from_value(raw).map_err(|e| invalid(&MapError::Json(e)))?;
    map.id = before.map.id.clone();
    map.version = FORMAT_VERSION;
    let name = clean_name(&map.name)?;
    map.name = name;
    // The image behind the grid is the import's, whatever the editor sends.
    let image = before.map.backdrop.as_ref().and_then(|b| b.image.clone());
    match (before.backdrop, map.backdrop.as_mut()) {
        (true, Some(b)) => b.image = image,
        (true, None) => map.backdrop.clone_from(&before.map.backdrop),
        (false, Some(b)) => b.image = None,
        (false, None) => {}
    }
    map.validate().map_err(|e| invalid(&e))?;
    let row: Row = sqlx::query_as(&format!(
        "UPDATE campaign_maps SET map = $3, validated_at = NULL
         WHERE campaign_id = $1 AND id = $2 RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(id)
    .bind(Json(&map))
    .fetch_one(&mut *tx)
    .await?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(campaign_map(row))
}

/// The GM validates map `id`: from now on it may be shown at the table.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`.
pub async fn validate(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
) -> Result<CampaignMap, AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let row: Option<Row> = sqlx::query_as(&format!(
        "UPDATE campaign_maps SET validated_at = now()
         WHERE campaign_id = $1 AND id = $2 RETURNING {COLUMNS}"
    ))
    .bind(campaign)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let row = row.ok_or(AppError::NotFound("NO_SUCH_MAP"))?;
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(campaign_map(row))
}

/// # Errors
///
/// 404 `NO_SUCH_MAP`; 409 `MAP_ON_THE_TABLE` while it is shown.
pub async fn delete(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    id: &str,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let shown: Option<String> =
        sqlx::query_scalar("SELECT map_id FROM map_states WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_optional(&mut *tx)
            .await?;
    if shown.as_deref() == Some(id) {
        return Err(AppError::Conflict("MAP_ON_THE_TABLE"));
    }
    let gone = sqlx::query("DELETE FROM campaign_maps WHERE campaign_id = $1 AND id = $2")
        .bind(campaign)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::NotFound("NO_SUCH_MAP"));
    }
    live::touch(&mut tx, campaign, &Topic::Desk).await?;
    tx.commit().await?;
    Ok(())
}

/// The type of an image, from its first bytes; `None` for anything else.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Base64, bare or as a `data:` URL.
fn decode_image(raw: &str) -> Result<(Vec<u8>, String), AppError> {
    let data = raw.split_once(";base64,").map_or(raw, |(_, d)| d);
    let bytes = STANDARD
        .decode(data.trim())
        .map_err(|_| AppError::BadRequest("BAD_IMAGE"))?;
    if bytes.len() > MAX_BACKDROP_BYTES {
        return Err(AppError::BadRequest("IMAGE_TOO_LARGE"));
    }
    let mime = image_mime(&bytes).ok_or(AppError::BadRequest("BAD_IMAGE"))?;
    Ok((bytes, mime.to_string()))
}

/// What the GM imports: a Universal VTT file, or an image and the grid
/// they aligned on it.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Import {
    #[serde(rename_all = "camelCase")]
    Uvtt { name: String, file: String },
    #[serde(rename_all = "camelCase")]
    Image {
        name: String,
        /// Base64 or a `data:` URL.
        image: String,
        /// Pixels of the image per cell.
        cell_px: f64,
        /// Pixels left and above the first whole cell.
        offset_x: f64,
        offset_y: f64,
        columns: u32,
        rows: u32,
    },
}

/// # Errors
///
/// 404; 400 `NAME_REQUIRED`, `TEXT_TOO_LONG`, `BAD_IMAGE`,
/// `IMAGE_TOO_LARGE`, `BAD_SIZE`, `MAP_INVALID`.
pub async fn import(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    import: &Import,
) -> Result<CampaignMap, AppError> {
    let row = owned_by(campaigns::find(pool, campaign).await?, gm)?;
    let (theme, floor) = look(&row.story);
    let (map, image) = match import {
        Import::Uvtt { name, file } => {
            let name = clean_name(name)?;
            let id = fresh_id(pool, campaign, &row.story, &name).await?;
            let read = from_uvtt(file, &id, &name, &theme, &floor).map_err(|e| invalid(&e))?;
            let image = if read.image_base64.is_empty() {
                None
            } else {
                Some(decode_image(&read.image_base64)?)
            };
            let mut map = read.map;
            if image.is_none() {
                map.backdrop = None;
            }
            (map, image)
        }
        Import::Image {
            name,
            image,
            cell_px,
            offset_x,
            offset_y,
            columns,
            rows,
        } => {
            let name = clean_name(name)?;
            let image = decode_image(image)?;
            if !(cell_px.is_finite() && *cell_px >= 8.0) {
                return Err(AppError::BadRequest("BAD_SIZE"));
            }
            let id = fresh_id(pool, campaign, &row.story, &name).await?;
            let mut map = blank(&row.story, id, name, *columns, *rows)?;
            // No wall yet: the GM traces them on the image.
            map.grid = open_floor(&map, *columns, *rows)?;
            map.theme = theme;
            map.backdrop = Some(Backdrop {
                prompt: None,
                image: Some("import".into()),
                cell_px: Some(*cell_px),
                offset: Some([*offset_x, *offset_y]),
            });
            (map, Some(image))
        }
    };
    insert(pool, campaign, &map, Source::Imported, None, image).await
}

/// `map`'s floor without its surrounding wall.
fn open_floor(map: &Map, width: u32, height: u32) -> Result<Grid, AppError> {
    let ground = map
        .grid
        .legend()
        .find(|(g, _)| *g == '.')
        .map(|(_, k)| k.clone())
        .ok_or(AppError::Internal("blank map has no floor".into()))?;
    let legend = BTreeMap::from([('.', ground)]);
    let rows: Vec<String> = (0..height).map(|_| ".".repeat(width as usize)).collect();
    Grid::new(&legend, &rows).map_err(|e| invalid(&e.into()))
}

/// The imported image behind map `id` of `campaign`.
///
/// # Errors
///
/// 404 `NO_SUCH_MAP`.
pub async fn backdrop(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    id: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let row: Option<(Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT backdrop, backdrop_mime FROM campaign_maps WHERE campaign_id = $1 AND id = $2",
    )
    .bind(campaign)
    .bind(id)
    .fetch_optional(db)
    .await?;
    match row {
        Some((Some(bytes), mime)) => Ok((bytes, mime.unwrap_or_else(|| "image/png".into()))),
        _ => Err(AppError::NotFound("NO_SUCH_MAP")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_images_pass_as_a_backdrop() {
        let png = STANDARD.encode(b"\x89PNG\r\n\x1a\nrest");
        assert_eq!(decode_image(&png).unwrap().1, "image/png");
        let url = format!("data:image/png;base64,{png}");
        assert_eq!(decode_image(&url).unwrap().1, "image/png");
        let text = STANDARD.encode(b"<svg onload=alert(1)>");
        assert!(matches!(
            decode_image(&text),
            Err(AppError::BadRequest("BAD_IMAGE"))
        ));
    }
}
