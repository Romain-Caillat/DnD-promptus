//! One lock per campaign for world writes (`MEMORY.md` §3). Ports the
//! intent of V1's `concurrency.test.ts`: simultaneous writes by the GM
//! and players must all land, none overwriting another.
//!
//! Each writer holds its transaction open for a moment between reading
//! and writing, so the writes really overlap. A control test runs the
//! same writers without the lock and shows updates being lost: it proves
//! the harness is concurrent enough for the main test to mean something.

mod common;

use std::time::Duration;

use promptus_back::auth::guard::CurrentGm;
use promptus_back::campaigns;
use promptus_back::error::AppError;
use promptus_shared::story::{FlagValue, WorldState, from_yaml};
use sqlx::PgPool;
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../content/fixtures/phare-de-kerbrume.yaml");
const WRITERS: usize = 12;

async fn setup(pool: &PgPool) -> (CurrentGm, Uuid) {
    let (id, _) = common::signed_in_gm(pool, "Romain").await;
    let gm = CurrentGm {
        id,
        display_name: "Romain".into(),
    };
    let row = campaigns::create(pool, &gm, &from_yaml(FIXTURE).unwrap())
        .await
        .unwrap();
    (gm, row.id)
}

async fn world_of(pool: &PgPool, id: Uuid) -> WorldState {
    campaigns::find(pool, id).await.unwrap().unwrap().world
}

/// Writer `i` sets flag `w{i}`; `locked` chooses the campaign lock or a
/// plain read.
async fn write_flag(pool: PgPool, id: Uuid, i: usize, locked: bool) {
    let mut tx = pool.begin().await.unwrap();
    let mut row = if locked {
        campaigns::lock(&mut tx, id).await.unwrap().unwrap()
    } else {
        campaigns::find(&mut *tx, id).await.unwrap().unwrap()
    };
    tokio::time::sleep(Duration::from_millis(30)).await;
    row.world.set_flag(&format!("w{i}"), FlagValue::Bool(true));
    campaigns::save_world(&mut tx, id, &row.world)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn run_writers(pool: &PgPool, id: Uuid, locked: bool) {
    let tasks: Vec<_> = (0..WRITERS)
        .map(|i| tokio::spawn(write_flag(pool.clone(), id, i, locked)))
        .collect();
    for t in tasks {
        t.await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn simultaneous_world_writes_all_land() {
    let pool = common::test_pool_sized(WRITERS as u32 + 2).await;
    let (_, id) = setup(&pool).await;
    run_writers(&pool, id, true).await;
    assert_eq!(world_of(&pool, id).await.flags.len(), WRITERS);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn control_without_the_lock_writes_are_lost() {
    let pool = common::test_pool_sized(WRITERS as u32 + 2).await;
    let (_, id) = setup(&pool).await;
    run_writers(&pool, id, false).await;
    let kept = world_of(&pool, id).await.flags.len();
    assert!(
        kept < WRITERS,
        "without the lock, {kept} of {WRITERS} writes kept"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_world_serialises_read_modify_write() {
    let pool = common::test_pool_sized(WRITERS as u32 + 2).await;
    let (gm, id) = setup(&pool).await;
    // Each call increments a counter it reads from the world: a lost
    // update shows as a total below the number of calls.
    let tasks: Vec<_> = (0..WRITERS)
        .map(|_| {
            let (pool, gm) = (pool.clone(), gm.clone());
            tokio::spawn(async move {
                campaigns::update_world(&pool, &gm, id, |_, w| {
                    let n = match w.flags.get("count") {
                        Some(FlagValue::Int(n)) => *n,
                        _ => 0,
                    };
                    std::thread::sleep(Duration::from_millis(5));
                    w.set_flag("count", FlagValue::Int(n + 1));
                    Ok(())
                })
                .await
                .unwrap();
            })
        })
        .collect();
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(
        world_of(&pool, id).await.flags.get("count"),
        Some(&FlagValue::Int(WRITERS as i64))
    );
}

#[tokio::test]
async fn another_gm_cannot_write_the_world() {
    let pool = common::test_pool().await;
    let (_, id) = setup(&pool).await;
    let (marc, _) = common::signed_in_gm(&pool, "Marc").await;
    let marc = CurrentGm {
        id: marc,
        display_name: "Marc".into(),
    };
    let r = campaigns::update_world(&pool, &marc, id, |c, w| {
        w.reveal_clue(c, "cl_gwen").unwrap();
        Ok(())
    })
    .await;
    assert!(matches!(r, Err(AppError::NotFound("NOT_FOUND"))), "{r:?}");
    assert_eq!(world_of(&pool, id).await, WorldState::default());
}
