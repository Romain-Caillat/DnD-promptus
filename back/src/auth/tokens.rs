//! Random secrets handed to a client, and the form the server keeps.
//!
//! A token is 32 bytes from the OS CSPRNG, sent as 43 characters of
//! URL-safe base64. The server stores only its SHA-256: with 256 bits of
//! entropy there is nothing to brute-force, so a plain hash (no salt, no
//! key) is enough for a database dump to yield nothing usable, and a
//! lookup stays a unique-index hit. GM sessions and invitations use it;
//! player device tokens (`session/invite-and-join`) are meant to as well.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// A fresh random token, as the client will hold it.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// What the database stores for `token`.
pub fn hash_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}
