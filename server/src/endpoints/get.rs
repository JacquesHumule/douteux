use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{AppState, kv::Entry};

/// `GET /*slug` – resolve a slug to a url or a presigned R2 file URL
#[worker::send]
pub(crate) async fn handler(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    let entry = match state.kv.get(&slug).await {
        Ok(Some(entry)) => entry,
        Ok(None) => return (StatusCode::NOT_FOUND, "Slug not found").into_response(),
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("KV error: {e}")).into_response();
        }
    };

    match entry {
        Entry::Url { url } => match HeaderValue::from_str(&url) {
            Ok(location) => {
                let mut headers = HeaderMap::new();
                headers.insert(header::LOCATION, location);
                (StatusCode::MOVED_PERMANENTLY, headers).into_response()
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Invalid URL").into_response(),
        },

        Entry::File { r2_key } => {
            // Presigned URL valid for 1 hour
            let signed_url = match state.r2_creds.presign_get(&r2_key, 3600_i64) {
                Ok(u) => u,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Presign failed: {e}"),
                    )
                        .into_response();
                }
            };

            match HeaderValue::from_str(&signed_url) {
                Ok(location) => {
                    let mut headers = HeaderMap::new();
                    headers.insert(header::LOCATION, location);
                    headers.insert(
                        header::CACHE_CONTROL,
                        HeaderValue::from_str("no-store, no-cache, must-revalidate").unwrap(),
                    );
                    (StatusCode::FOUND, headers).into_response()
                }
                Err(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Invalid presigned URL").into_response()
                }
            }
        }
    }
}
