use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
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

mod admin;
mod boards;
mod error;
mod headers;
mod static_files;

#[derive(Clone)]
pub struct AppState {
    db: ExecutorConnection,
    cfg: Arc<Config>,
}

pub async fn build(db: ExecutorConnection, cfg: Arc<Config>, store: MemoryStore) -> Result<Router> {
    let secret = base64::decode_config(&cfg.cookie_secret, URL_SAFE_NO_PAD)?;

    let admin_router = Router::new()
        .route("/admin", get(admin::handle_home))
        .route("/admin/board/create", post(admin::handle_createboard))
        .route("/admin/board/:b/delete", post(admin::handle_deleteboard))
        .route("/admin/board/:b/update", post(admin::handle_updateboard))
        .route("/admin/post/:p/delete", post(admin::handle_deletepost))
        .route("/admin/logout", post(admin::handle_logout))
        .route_layer(middleware::from_fn(admin::auth_middleware))
        .route("/admin/login", get(admin::handle_loginpage))
        .route("/admin/login", post(admin::handle_login));

    let router = Router::new()
        .route("/", get(boards::handle_home))
        .route("/:b", get(boards::handle_view))
        .route("/:b/post", post(boards::handle_post))
        .route("/static/*file", get(static_files::static_handler))
        .route("/img/*file", get(static_files::image_handler))
        .fallback_service(get(|| async { error::http_404() }))
        .merge(admin_router)
        .layer(SessionLayer::new(store, &secret))
        .layer(DefaultBodyLimit::max(cfg.max_upload_size))
        .with_state(AppState { db, cfg });
    Ok(router)
}
