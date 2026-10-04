//! Pure Rust code shared by the server and the Tauri shell: the rules
//! engine and the domain models. No I/O lives here — callers read files
//! and hand the text over; the server stays authoritative on every rule,
//! and this crate is what it applies.

pub mod rules;
