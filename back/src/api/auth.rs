//! GM sign-in endpoints.
//!
//! Public (nobody has a session yet):
//! - `GET  /api/auth/status` → `{ needsSetup }`: the first account is
//!   still to be created;
//! - `POST /api/auth/register/options` → `{ code, displayName }`: checks
//!   the setup code or an invitation, returns a registration challenge;
//! - `POST /api/auth/register` → `{ ceremonyId, credential }`: creates
//!   the account and signs it in (201, session cookie);
//! - `POST /api/auth/sign-in/options` → a sign-in challenge;
//! - `POST /api/auth/sign-in` → `{ ceremonyId, credential }`: a session.
//!
//! GM (behind `require_gm`):
//! - `POST /api/auth/sign-out` → ends the session, clears the cookie.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use webauthn_rs::prelude::{PublicKeyCredential, RegisterPublicKeyCredential};

use super::body::Body;
use crate::auth::guard::CurrentGm;
use crate::auth::{accounts, session, setup};
use crate::error::AppError;
use crate::state::AppState;

fn signed_in(status: StatusCode, state: &AppState, gm: &CurrentGm, token: &str) -> Response {
    (
        status,
        [(
            header::SET_COOKIE,
            session::set_cookie(token, state.auth.secure_cookie),
        )],
        Json(json!({ "data": gm })),
    )
        .into_response()
}

/// `GET /api/auth/status`
///
/// # Errors
///
/// Fails on a database error.
pub async fn status(State(state): State<AppState>) -> Result<Response, AppError> {
    let needs_setup = setup::gm_count(&state.pool).await? == 0;
    Ok(Json(json!({ "data": { "needsSetup": needs_setup } })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RegisterOptionsBody {
    pub code: String,
    pub display_name: String,
}

/// `POST /api/auth/register/options`
///
/// # Errors
///
/// 400 `INVALID_DISPLAY_NAME`, 403 `INVALID_REGISTRATION_CODE`.
pub async fn register_options(
    State(state): State<AppState>,
    Body(body): Body<RegisterOptionsBody>,
) -> Result<Response, AppError> {
    let challenge =
        accounts::begin_registration(&state.pool, &state.auth, &body.code, &body.display_name)
            .await?;
    Ok(Json(json!({ "data": challenge })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RegisterBody {
    pub ceremony_id: String,
    pub credential: RegisterPublicKeyCredential,
}

/// `POST /api/auth/register`
///
/// # Errors
///
/// 401 `INVALID_PASSKEY`, 403 `INVALID_REGISTRATION_CODE`, 400
/// `PASSKEY_ALREADY_REGISTERED`.
pub async fn register(
    State(state): State<AppState>,
    Body(body): Body<RegisterBody>,
) -> Result<Response, AppError> {
    let (gm, token) = accounts::finish_registration(
        &state.pool,
        &state.auth,
        &body.ceremony_id,
        &body.credential,
    )
    .await?;
    Ok(signed_in(StatusCode::CREATED, &state, &gm, &token))
}

/// `POST /api/auth/sign-in/options`
///
/// # Errors
///
/// 503 `TOO_MANY_CEREMONIES` when the challenge table is full.
pub async fn sign_in_options(State(state): State<AppState>) -> Result<Response, AppError> {
    let challenge = state.auth.passkeys.begin_sign_in()?;
    Ok(Json(json!({ "data": challenge })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SignInBody {
    pub ceremony_id: String,
    pub credential: PublicKeyCredential,
}

/// `POST /api/auth/sign-in`
///
/// # Errors
///
/// 401 `INVALID_PASSKEY`.
pub async fn sign_in(
    State(state): State<AppState>,
    Body(body): Body<SignInBody>,
) -> Result<Response, AppError> {
    let (gm, token) = accounts::finish_sign_in(
        &state.pool,
        &state.auth,
        &body.ceremony_id,
        &body.credential,
    )
    .await?;
    Ok(signed_in(StatusCode::OK, &state, &gm, &token))
}

/// `POST /api/auth/sign-out` (GM)
///
/// # Errors
///
/// Fails on a database error.
pub async fn sign_out(
    State(state): State<AppState>,
    _gm: CurrentGm,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(token) = session::token_from_headers(&headers) {
        session::delete(&state.pool, &token).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            session::clear_cookie(state.auth.secure_cookie),
        )],
    )
        .into_response())
}
