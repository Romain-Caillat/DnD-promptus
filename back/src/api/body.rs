use axum::Json;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;

use crate::error::AppError;

/// A JSON request body whose rejection keeps the API's error envelope
/// (400 `INVALID_BODY`) instead of axum's plain-text one.
pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                tracing::info!("request body refused: {rejection}");
                Err(AppError::BadRequest("INVALID_BODY"))
            }
        }
    }
}
