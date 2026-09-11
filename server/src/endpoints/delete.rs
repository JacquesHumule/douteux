use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{AppState, auth::check_auth, kv::Entry};

/// `DELETE /*slug` – remove a slug and its associated R2 object if any
#[worker::send]
pub(crate) async fn handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    request: Request,
) -> Response {
    if !check_auth(request.headers(), &state.auth_token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let entry = match state.kv.get(&slug).await {
        Ok(Some(entry)) => entry,
        Ok(None) => return (StatusCode::NOT_FOUND, "Slug not found").into_response(),
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("KV error: {e}")).into_response();
        }
    };

    if let Err(e) = state.kv.delete(&slug).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("KV delete error: {e}"),
        )
            .into_response();
    }

    // The R2 object is content-addressed and may be shared by other slugs
    // (dedup). Only drop it once nothing else points at it.
    if let Entry::File { r2_key } = &entry {
        let others = match state.kv.list().await {
            Ok(list) => list,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("KV list error: {e}"),
                )
                    .into_response();
            }
        };
        let still_referenced = others
            .iter()
            .any(|(_, entry)| matches!(entry, Some(Entry::File { r2_key: k }) if k == r2_key));

        if !still_referenced && let Err(e) = state.bucket.delete(r2_key).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("R2 delete error: {e}"),
            )
                .into_response();
        }
    }

    StatusCode::NO_CONTENT.into_response()
}
