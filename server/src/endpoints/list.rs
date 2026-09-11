use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{AppState, auth::check_auth, kv::Entry};

/// `GET /` – list all slugs. Requires auth.
#[worker::send]
pub(crate) async fn handler(State(state): State<AppState>, request: Request) -> Response {
    if !check_auth(request.headers(), &state.auth_token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let entries = match state.kv.list().await {
        Ok(entries) => entries,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("KV list error: {e}"),
            )
                .into_response();
        }
    };

    let list: Vec<ListEntry> = entries
        .into_iter()
        .map(|(slug, entry)| ListEntry {
            slug,
            kind: match entry {
                Some(Entry::Url { url }) => ListEntryKind::Url { url },
                Some(Entry::File { r2_key }) => ListEntryKind::File { r2_key },
                None => ListEntryKind::Unknown,
            },
        })
        .collect();

    Json(list).into_response()
}

#[derive(Serialize)]
struct ListEntry {
    slug: String,
    #[serde(flatten)]
    kind: ListEntryKind,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ListEntryKind {
    Url { url: String },
    File { r2_key: String },
    Unknown,
}
