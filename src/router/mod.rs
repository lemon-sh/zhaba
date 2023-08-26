use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use axum_sessions::{
    async_session::{
        base64::{self, URL_SAFE_NO_PAD},
        MemoryStore,
    },
    SessionLayer,
};
use color_eyre::Result;

use crate::{config::Config, database::ExecutorConnection};

mod error;
mod headers;
mod home;
mod static_files;

#[derive(Clone)]
pub struct AppState {
    db: ExecutorConnection,
    cfg: Arc<Config>,
}

pub async fn build(db: ExecutorConnection, cfg: Arc<Config>, store: MemoryStore) -> Result<Router> {
    let secret = base64::decode_config(&cfg.cookie_secret, URL_SAFE_NO_PAD)?;
    let router = Router::new()
        .route("/", get(home::handle_home))
        .route("/add", post(home::handle_add))
        .route("/static/*file", get(static_files::static_handler))
        .route("/img/*file", get(static_files::image_handler))
        .fallback_service(get(|| async { error::http_404() }))
        .layer(SessionLayer::new(store, &secret))
        .with_state(AppState { db, cfg });
    Ok(router)
}
