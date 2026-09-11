use std::{cell::RefCell, rc::Rc};

use axum::{
    Json,
    extract::{FromRequest, Query, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::{Error, FixedLengthStream, HttpMetadata};

use crate::{
    AppState,
    auth::check_auth,
    generators::{SlugPattern, generate_slug},
    kv::Entry,
};

/// `POST /` – create a new short link.
///
/// Dispatches on `Content-Type`:
/// - `application/json` → `{ "url": "https://..." }` stores a url
/// - anything else      → the raw request body is streamed into R2 as a file.
///   `?pattern=` picks the slug shape and `?filename=` sets the download name.
#[worker::send]
pub(crate) async fn handler(
    State(state): State<AppState>,
    Query(params): Query<CreateParams>,
    request: Request,
) -> Response {
    if !check_auth(request.headers(), &state.auth_token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if content_type.starts_with("application/json") {
        let Json(body) = match Json::<UrlRequest>::from_request(request, &()).await {
            Ok(j) => j,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        };
        create_url(state, body).await
    } else {
        create_file(state, params, request).await
    }
}

async fn create_url(state: AppState, body: UrlRequest) -> Response {
    let slug = generate_slug(body.pattern.unwrap_or_default());
    let entry = Entry::Url { url: body.url };

    if let Err(e) = state.kv.put(&slug, &entry).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to store entry: {e}"),
        )
            .into_response();
    }

    Json(SlugResponse { slug }).into_response()
}

/// `POST /` with a non-JSON body – stream the raw request body straight into
/// R2 without buffering it in the isolate.
///
/// The R2 object is content-addressed: its key is the SHA-256 of the body,
/// supplied by the client as `?sha256=` and verified here as the bytes stream
/// past. Identical uploads therefore land on the same object (dedup), and the
/// public slug stays independent of storage.
async fn create_file(state: AppState, params: CreateParams, request: Request) -> Response {
    let headers = request.headers();

    // `FixedLengthStream` needs the exact byte count up front, so Content-Length
    // is mandatory. The CLI sends a plain body with this header set.
    let content_length = match headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        Some(len) => len,
        None => {
            return (
                StatusCode::LENGTH_REQUIRED,
                "Content-Length is required for file uploads",
            )
                .into_response();
        }
    };

    let r2_key = match params.sha256.as_deref() {
        Some(hash) if is_sha256_hex(hash) => hash.to_ascii_lowercase(),
        Some(_) => {
            return (StatusCode::BAD_REQUEST, "sha256 must be 64 hex characters").into_response();
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "sha256 query parameter is required for file uploads",
            )
                .into_response();
        }
    };

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let filename = params.filename.as_deref().unwrap_or("download").to_string();

    let slug = generate_slug(params.pattern.unwrap_or_default());

    // Upload only when the object isn't already there. On a `head` error, fall
    // through and let the `put` surface it.
    let already_stored = matches!(state.bucket.head(&r2_key).await, Ok(Some(_)));

    if !already_stored {
        let hasher = Rc::new(RefCell::new(Sha256::new()));
        let tee = hasher.clone();
        let body = request.into_body().into_data_stream().map(move |chunk| {
            chunk
                .map(|b| {
                    tee.borrow_mut().update(&b);
                    b.to_vec()
                })
                .map_err(|e| Error::RustError(e.to_string()))
        });
        let stream = FixedLengthStream::wrap(body, content_length);

        if let Err(e) = state
            .bucket
            .put(&r2_key, stream)
            .http_metadata(HttpMetadata {
                content_type: Some(content_type),
                content_disposition: Some(format!("attachment; filename=\"{filename}\"")),
                ..Default::default()
            })
            .execute()
            .await
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to store file: {e}"),
            )
                .into_response();
        }

        let digest = hex_lower(&hasher.borrow().clone().finalize());
        if digest != r2_key {
            let _ = state.bucket.delete(&r2_key).await;
            return (
                StatusCode::BAD_REQUEST,
                "body does not match the given sha256",
            )
                .into_response();
        }
    }

    // Store metadata in KV
    let entry = Entry::File {
        r2_key: r2_key.clone(),
    };
    if let Err(e) = state.kv.put(&slug, &entry).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to store entry: {e}"),
        )
            .into_response();
    }

    Json(SlugResponse { slug }).into_response()
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Query parameters for `POST /` file uploads.
#[derive(Deserialize)]
pub(crate) struct CreateParams {
    /// Slug shape to generate, e.g. `shorten`, `shorten:12`, `shady:movie`.
    /// See [`SlugPattern`]. Omitted falls back to the default.
    pattern: Option<SlugPattern>,
    /// Original file name, used for the `Content-Disposition` of the R2 object.
    filename: Option<String>,
    /// Lowercase hex SHA-256 of the body — becomes the content-addressed R2 key.
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct UrlRequest {
    url: String,
    /// Slug shape to generate, e.g. `shorten`, `shorten:12`, `shady:movie`.
    /// See [`SlugPattern`]. Omitted falls back to the default.
    #[serde(default)]
    pattern: Option<SlugPattern>,
}

#[derive(Serialize)]
struct SlugResponse {
    slug: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::ShadyPattern;

    // --- `POST /` file uploads: `?pattern=…&filename=…` -------------------
    //
    // `serde_urlencoded` is the exact decoder `axum::extract::Query` runs on
    // the raw query string, so these go through the real parser.

    fn query(q: &str) -> Result<CreateParams, serde_urlencoded::de::Error> {
        serde_urlencoded::from_str(q)
    }

    #[test]
    fn query_full_upload_params() {
        // What the CLI sends: reqwest percent-encodes the `:` to %3A.
        let p = query("filename=Holiday.Clip.mkv&pattern=shorten%3A20").unwrap();
        assert!(matches!(p.pattern, Some(SlugPattern::Shorten { len: 20 })));
        assert_eq!(p.filename.as_deref(), Some("Holiday.Clip.mkv"));
    }

    #[test]
    fn query_accepts_a_raw_colon_too() {
        let p = query("pattern=shady:movie").unwrap();
        assert!(matches!(
            p.pattern,
            Some(SlugPattern::Shady(ShadyPattern::Movie))
        ));
    }

    #[test]
    fn query_accepts_the_crack_shady_kind() {
        let p = query("pattern=shady:crack").unwrap();
        assert!(matches!(
            p.pattern,
            Some(SlugPattern::Shady(ShadyPattern::Crack))
        ));
    }

    #[test]
    fn query_accepts_the_tape_shady_kind() {
        let p = query("pattern=shady:tape").unwrap();
        assert!(matches!(
            p.pattern,
            Some(SlugPattern::Shady(ShadyPattern::Tape))
        ));
    }

    #[test]
    fn query_decodes_percent_escapes_in_filename() {
        let p = query("filename=r%C3%A9sum%C3%A9%20final.pdf").unwrap();
        assert_eq!(p.filename.as_deref(), Some("résumé final.pdf"));
        assert!(p.pattern.is_none());
    }

    #[test]
    fn query_decodes_plus_as_space_in_filename() {
        let p = query("filename=my+notes.txt").unwrap();
        assert_eq!(p.filename.as_deref(), Some("my notes.txt"));
    }

    #[test]
    fn query_missing_keys_are_none() {
        let p = query("").unwrap();
        assert!(p.pattern.is_none());
        assert!(p.filename.is_none());
        assert!(p.sha256.is_none());

        let p = query("filename=a.bin").unwrap();
        assert!(p.pattern.is_none());
    }

    /// What `sus upload` sends when `--pattern` is omitted: no `pattern` key at
    /// all. The upload path must still resolve to shady, never a bare random
    /// slug.
    ///
    /// Stops at the pattern rather than the finished slug: `generate_slug` goes
    /// through `Rng::new` → `worker::Date::now`, which only exists on wasm.
    #[test]
    fn an_upload_without_a_pattern_falls_back_to_shady() {
        let p = query(&format!("filename=a.bin&sha256={}", "a".repeat(64))).unwrap();
        assert!(p.pattern.is_none());
        assert!(matches!(
            p.pattern.unwrap_or_default(),
            SlugPattern::Shady(ShadyPattern::Movie)
        ));
    }

    #[test]
    fn query_carries_the_content_hash() {
        let digest = "a".repeat(64);
        let p = query(&format!("filename=a.bin&sha256={digest}")).unwrap();
        assert_eq!(p.sha256.as_deref(), Some(digest.as_str()));
    }

    #[test]
    fn sha256_hex_validation() {
        let ok = "0123456789abcdef".repeat(4); // 64 hex chars
        assert!(is_sha256_hex(&ok));
        assert!(!is_sha256_hex(&ok[..63])); // too short
        assert!(!is_sha256_hex(&format!("{ok}0"))); // too long
        assert!(!is_sha256_hex(&"g".repeat(64))); // non-hex
        assert!(is_sha256_hex(&ok.to_ascii_uppercase())); // case-insensitive
    }

    #[test]
    fn hex_lower_encodes_bytes() {
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");

        let digest = hex_lower(&Sha256::new().chain_update(b"").finalize());
        assert_eq!(digest.len(), 64);
        assert!(is_sha256_hex(&digest));
    }

    #[test]
    fn query_rejects_an_invalid_pattern() {
        let Err(err) = query("pattern=whoops&filename=a.bin") else {
            panic!("expected a parse error");
        };
        assert!(err.to_string().contains("unknown pattern"), "{err}");
    }

    #[test]
    fn query_rejects_a_bad_shorten_length() {
        assert!(query("pattern=shorten%3Amany").is_err());
    }

    // --- `POST /` url shortening: JSON body ------------------------------

    fn json(body: &str) -> serde_json::Result<UrlRequest> {
        serde_json::from_str(body)
    }

    #[test]
    fn json_url_with_pattern() {
        let r = json(r#"{"url":"https://example.com/a/b?x=1","pattern":"shorten:8"}"#).unwrap();
        assert_eq!(r.url, "https://example.com/a/b?x=1");
        assert!(matches!(r.pattern, Some(SlugPattern::Shorten { len: 8 })));
    }

    #[test]
    fn json_pattern_is_optional() {
        let r = json(r#"{"url":"https://example.com"}"#).unwrap();
        assert!(r.pattern.is_none());
        assert!(matches!(
            r.pattern.unwrap_or_default(),
            SlugPattern::Shady(ShadyPattern::Movie)
        ));
    }

    #[test]
    fn json_null_pattern_is_none() {
        let r = json(r#"{"url":"https://example.com","pattern":null}"#).unwrap();
        assert!(r.pattern.is_none());
    }

    #[test]
    fn json_shady_pattern() {
        let r = json(r#"{"url":"https://example.com","pattern":"shady"}"#).unwrap();
        assert!(matches!(
            r.pattern,
            Some(SlugPattern::Shady(ShadyPattern::Movie))
        ));
    }

    #[test]
    fn json_shady_crack_pattern() {
        let r = json(r#"{"url":"https://example.com","pattern":"shady:crack"}"#).unwrap();
        assert!(matches!(
            r.pattern,
            Some(SlugPattern::Shady(ShadyPattern::Crack))
        ));
    }

    #[test]
    fn json_shady_tape_pattern() {
        let r = json(r#"{"url":"https://example.com","pattern":"shady:tape"}"#).unwrap();
        assert!(matches!(
            r.pattern,
            Some(SlugPattern::Shady(ShadyPattern::Tape))
        ));
    }

    #[test]
    fn json_rejects_an_invalid_pattern() {
        let Err(err) = json(r#"{"url":"https://example.com","pattern":"nonsense"}"#) else {
            panic!("expected a parse error");
        };
        assert!(err.to_string().contains("unknown pattern"), "{err}");
    }

    #[test]
    fn json_requires_the_url_field() {
        assert!(json(r#"{"pattern":"shorten"}"#).is_err());
    }

    #[test]
    fn json_ignores_unknown_fields() {
        let r = json(r#"{"url":"https://example.com","pattern":"shorten","extra":true}"#).unwrap();
        assert!(matches!(r.pattern, Some(SlugPattern::Shorten { .. })));
    }
}
