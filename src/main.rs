use std::{str::FromStr, thread, sync::Arc, time::Duration};

use axum_sessions::async_session::{self, MemoryStore};
use color_eyre::{Result, eyre::Context};
use config::Config;
use tokio::{sync::broadcast, select, time::sleep};
use tracing::Level;

use crate::database::DbExecutor;

mod database;
mod config;
mod router;

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    tracing::debug!("Installed ctrl+c handler");
    select! {
        _ = sigterm.recv() => (),
        _ = sigint.recv() => ()
    }
}

#[cfg(windows)]
async fn terminate_signal() {
    use tokio::signal::windows::ctrl_c;
    let mut ctrlc = ctrl_c().unwrap();
    tracing::debug!("Installed ctrl+c handler");
    let _ = ctrlc.recv().await;
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;

    let cfg = Arc::new(Config::load().wrap_err("Failed to load the configuration file")?);

    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::from_str(&cfg.log_level)?)
        .init();

    tracing::info!(concat!(
        "Initializing - ZSERadio v",
        env!("CARGO_PKG_VERSION")
    ));

    let (db_exec, db_conn) = DbExecutor::create(cfg.db.as_deref().unwrap_or("zseradio.db3"))?;
    let exec_thread = thread::spawn(move || db_exec.run());

    let session_store = async_session::MemoryStore::new();
    let (ctx, _) = broadcast::channel(1);
    let maintenance_task = tokio::spawn(maintenance(
        ctx.subscribe(),
        session_store.clone(),
        3600
    ));

    let router = router::build(db_conn, cfg.clone(), session_store).await?;

    tracing::info!("Listening on {}", cfg.listen);
    if let Err(e) = axum::Server::bind(&cfg.listen)
        .serve(router.into_make_service())
        .with_graceful_shutdown(terminate_signal())
        .await
    {
        tracing::error!("An error has occurred: {e}, shutting down");
    }

    tracing::info!("Waiting for the maintenance task to shut down");
    let _ = ctx.send(());
    maintenance_task.await.unwrap();
    tracing::info!("Waiting for the database to shut down");
    exec_thread.join().unwrap();
    tracing::info!("Shutdown complete!");
    Ok(())
}


async fn maintenance(
    mut shutdown: broadcast::Receiver<()>,
    session_store: MemoryStore,
    interval_secs: u64,
) {
    let interval = Duration::from_secs(interval_secs);
    loop {
        if let Err(e) = session_store.cleanup().await {
            tracing::error!("Failed to cleanup sessions: {e}");
        } else {
            tracing::info!("Sessions cleaned up");
        }
        select! {
            _ = sleep(interval) => {}
            _ = shutdown.recv() => return,
        }
    }
}