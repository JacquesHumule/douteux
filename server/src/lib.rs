use axum::{Router, body::Body, routing::get};
use tower_service::Service;
use worker::*;

use crate::r2::R2Creds;

mod auth;
mod endpoints;
mod generators;
mod kv;
mod r2;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    kv: kv::KvStore,
    bucket: worker::Bucket,
    /// R2 S3-compatible credentials, loaded from Worker secrets
    r2_creds: R2Creds,
    /// Bearer token for write/delete routes
    auth_token: String,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/",
            get(endpoints::list::handler).post(endpoints::create::handler),
        )
        .route(
            "/{*slug}",
            get(endpoints::get::handler).delete(endpoints::delete::handler),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Cloudflare Worker entrypoint
// ---------------------------------------------------------------------------

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> Result<axum::http::Response<Body>> {
    let kv = env.kv("KV")?;
    let bucket = env.bucket("R2")?;

    let r2_creds = R2Creds::from_env(&env)?;

    let state = AppState {
        kv: kv::KvStore::new(kv),
        bucket,
        r2_creds,
        auth_token: env.secret("AUTH_TOKEN")?.to_string(),
    };
    Ok(router(state).call(req).await?)
}
