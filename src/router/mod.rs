use std::sync::Arc;

use axum::{routing::get, Router, extract::{State, Query}, body::Body, response::{IntoResponse, Response}};
use axum_sessions::{
    async_session::{
        base64::{self, URL_SAFE_NO_PAD},
        MemoryStore,
    },
    SessionLayer,
};
use color_eyre::Result;
use serde::Deserialize;

use crate::{config::Config, database::ExecutorConnection, templates};

mod error;
mod static_files;

const PAGE_SIZE: usize = 25;

#[derive(Clone)]
pub struct AppState {
    db: ExecutorConnection,
}

pub async fn build(db: ExecutorConnection, cfg: Arc<Config>, store: MemoryStore) -> Result<Router> {
    let secret = base64::decode_config(&cfg.cookie_secret, URL_SAFE_NO_PAD)?;
    let router = Router::new()
        .route("/", get(home))
        .route("/static/*file", get(static_files::static_handler))
        .fallback_service(get(|| async { error::http_404() }))
        .layer(SessionLayer::new(store, &secret))
        .with_state(AppState { db });
    Ok(router)
}

#[derive(Deserialize)]
struct PageQuery {
    p: Option<usize>
}

async fn home(State(state): State<AppState>, Query(query): Query<PageQuery>) -> Result<impl IntoResponse, Response<Body>> {
    let page = query.p.unwrap_or(0);
    let posts = state.db.posts_display(page*PAGE_SIZE, PAGE_SIZE).await.map_err(error::err_into_500)?;
    Ok(templates::Index { posts })
}
