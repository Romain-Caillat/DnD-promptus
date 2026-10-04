//! Pure Rust code shared by the server and the Tauri shell: the rules
//! engine and the domain models. No I/O lives here — the server stays
//! authoritative on every rule, and this crate is what it applies.
//!
//! Empty until the `engine` epic lands; nothing depends on it yet.

pub mod story;
