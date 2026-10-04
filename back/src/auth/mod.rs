//! platform/sign-in-gm — GM accounts.
//!
//! A GM signs in with a passkey and holds a server-side session behind
//! an HttpOnly cookie; every GM route sits behind [`guard::require_gm`].
//! Players have no account: they will join with a hashed device token
//! (`session/invite-and-join`), through their own extractor, never this
//! one.

pub mod accounts;
pub mod guard;
pub mod invites;
pub mod passkeys;
pub mod session;
pub mod setup;
pub mod tokens;
