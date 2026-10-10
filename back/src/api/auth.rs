//! GM sign-in endpoints.
//!
//! Public (nobody has a session yet):
//! - `POST /api/auth/code` → `{ email }`: emails a sign-in code (204);
//! - `POST /api/auth/verify` → `{ email, code, displayName? }`: a session
//!   (200, or 201 when the account was created by this sign-in; session
//!   cookie). A first sign-in without `displayName` answers 409
//!   `DISPLAY_NAME_REQUIRED` and keeps the code.
//!
//! GM (behind `require_gm`):
//! - `POST /api/auth/sign-out` → ends the session, clears the cookie.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use super::body::Body;
use crate::auth::guard::CurrentGm;
use crate::auth::{codes, session};
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeBody {
    pub email: String,
}

/// `POST /api/auth/code`
///
/// # Errors
///
/// 400 `INVALID_EMAIL`, 429 `CODE_TOO_SOON` / `TOO_MANY_CODES`, 503
/// `EMAIL_UNAVAILABLE`.
pub async fn request_code(
    State(state): State<AppState>,
    Body(body): Body<CodeBody>,
) -> Result<Response, AppError> {
    codes::request(&state.pool, state.auth.mailer.as_ref(), &body.email).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VerifyBody {
    pub email: String,
    pub code: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

/// `POST /api/auth/verify`
///
/// # Errors
///
/// 400 `INVALID_EMAIL` / `INVALID_DISPLAY_NAME`, 401 `INVALID_CODE`, 409
/// `DISPLAY_NAME_REQUIRED`.
pub async fn verify(
    State(state): State<AppState>,
    Body(body): Body<VerifyBody>,
) -> Result<Response, AppError> {
    let done = codes::verify(
        &state.pool,
        &body.email,
        &body.code,
        body.display_name.as_deref(),
    )
    .await?;
    let status = if done.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok(signed_in(status, &state, &done.gm, &done.token))
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
