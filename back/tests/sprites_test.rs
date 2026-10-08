mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

const LOOK: &str = r#"{"pack":"marins-1718","body":"svelte","skin":"hale","hair":{"style":"court","colour":"brun"},"outfit":{"piece":"chemise","dye":"rouge"},"weapon":"sabre"}"#;

fn get(uri: &str, etag: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().uri(uri);
    if let Some(e) = etag {
        b = b.header(header::IF_NONE_MATCH, e);
    }
    b.body(Body::empty()).unwrap()
}

fn render_uri(look: &str, facing: &str) -> String {
    let encoded: String = look
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("/api/sprites/render.png?look={encoded}&facing={facing}")
}

#[tokio::test]
async fn a_description_is_drawn_as_a_png_without_an_account() {
    let app = common::app(common::test_pool().await);

    let response = app
        .clone()
        .oneshot(get(&render_uri(LOOK, "east"), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
    let etag = response.headers()[header::ETAG]
        .to_str()
        .unwrap()
        .to_string();
    let east = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&east[1..4], b"PNG");

    // The same description, same pixels, same bytes; facing west differs.
    let again = app
        .clone()
        .oneshot(get(&render_uri(LOOK, "east"), None))
        .await
        .unwrap();
    assert_eq!(to_bytes(again.into_body(), usize::MAX).await.unwrap(), east);
    let west = app
        .clone()
        .oneshot(get(&render_uri(LOOK, "west"), None))
        .await
        .unwrap();
    assert_ne!(west.headers()[header::ETAG].to_str().unwrap(), etag);

    // A client that has it already gets a 304.
    let cached = app
        .oneshot(get(&render_uri(LOOK, "east"), Some(&etag)))
        .await
        .unwrap();
    assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn an_unknown_piece_is_a_400_with_its_code() {
    let app = common::app(common::test_pool().await);
    let look = LOOK.replace("\"sabre\"", "\"trident\"");

    let response = app
        .clone()
        .oneshot(get(&render_uri(&look, "east"), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = common::read_json(response).await;
    assert_eq!(json["error"]["code"], "SPRITE_UNKNOWN_PIECE");

    let response = app
        .oneshot(get(&render_uri("{\"pack\":1}", "east"), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = common::read_json(response).await;
    assert_eq!(json["error"]["code"], "SPRITE_LOOK_INVALID");
}

#[tokio::test]
async fn the_looks_of_both_worlds_are_listed_and_all_draw() {
    let app = common::app(common::test_pool().await);

    let response = app
        .clone()
        .oneshot(get("/api/sprites/looks", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = common::read_json(response).await;
    let worlds = json["data"]["worlds"].as_array().unwrap();
    let names: Vec<&str> = worlds
        .iter()
        .map(|w| w["world"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["corsaires", "brasier"]);
    for world in worlds {
        assert_eq!(world["party"].as_array().unwrap().len(), 6);
        for entry in world["party"]
            .as_array()
            .unwrap()
            .iter()
            .chain(world["foes"].as_array().unwrap())
        {
            let look = serde_json::to_string(&entry["look"]).unwrap();
            let response = app
                .clone()
                .oneshot(get(&render_uri(&look, "east"), None))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", entry["id"]);
        }
    }
}

/// characters/walk-in-four-directions: every look of both worlds has a
/// sheet of four frames in each direction, and the sheet is not the still.
#[tokio::test]
async fn every_look_walks_in_four_directions_as_a_sheet() {
    let app = common::app(common::test_pool().await);
    let json = common::read_json(
        app.clone()
            .oneshot(get("/api/sprites/looks", None))
            .await
            .unwrap(),
    )
    .await;
    for world in json["data"]["worlds"].as_array().unwrap() {
        for entry in world["party"]
            .as_array()
            .unwrap()
            .iter()
            .chain(world["foes"].as_array().unwrap())
        {
            let look = serde_json::to_string(&entry["look"]).unwrap();
            for facing in ["south", "east", "north", "west"] {
                let uri = render_uri(&look, facing).replace("render.png", "sheet.png");
                let response = app.clone().oneshot(get(&uri, None)).await.unwrap();
                assert_eq!(
                    response.status(),
                    StatusCode::OK,
                    "{} {facing}",
                    entry["id"]
                );
                let still = app
                    .clone()
                    .oneshot(get(&render_uri(&look, facing), None))
                    .await
                    .unwrap();
                assert_ne!(
                    still.headers()[header::ETAG],
                    response.headers()[header::ETAG],
                    "{} {facing}",
                    entry["id"]
                );
                let png = to_bytes(response.into_body(), usize::MAX).await.unwrap();
                // The IHDR width: four frames of 22 pixels.
                let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
                assert_eq!(width, 4 * 22, "{} {facing}", entry["id"]);
            }
        }
    }
}
