use std::sync::Arc;

use axum::Router;
use axum_sessions::async_session::MemoryStore;
use color_eyre::Result;

use crate::{config::Config, database::ExecutorConnection};

pub async fn build(db: ExecutorConnection, cfg: Arc<Config>, store: MemoryStore) -> Result<Router> {
    todo!()
}
