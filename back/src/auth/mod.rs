//! platform/sign-in-gm — GM accounts.
//!
//! A GM signs in with their email and a code sent to it ([`codes`],
//! through a [`mailer`]) and holds a server-side session behind
//! an HttpOnly cookie; every GM route sits behind [`guard::require_gm`].
//! Players have no account: they join with a hashed device token
//! (`session/invite-and-join`), through their own extractor, never this
//! one ([`player`]). A shared screen (TV) has its own token and guard
//! too ([`screen`]). A GM may also mint personal access tokens for a
//! program on their own machine, which reach campaign preparation only
//! ([`api_tokens`]).

pub mod api_tokens;
pub mod codes;
pub mod guard;
pub mod mailer;
pub mod player;
pub mod screen;
pub mod session;
pub mod tokens;
