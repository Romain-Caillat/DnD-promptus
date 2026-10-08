// `sqlx::migrate!` embeds `migrations/` at compile time: rebuild when a
// migration is added or changed, or a stale binary misses it.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
