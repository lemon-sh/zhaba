use std::{time::{Duration, Instant}, path::PathBuf};

use color_eyre::Result;
use axum::body::Bytes;
use rusqlite::{ErrorCode, OptionalExtension, params};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

mod queries;

macro_rules! generate_executor {
    ($($task:ident / $fn:ident, ($db:ident, $($arg:ident: $ty:ty),*) => $ret:ty $handler:block)*) => {
        #[derive(Clone)]
        pub struct ExecutorConnection(UnboundedSender<Task>);

        #[derive(Debug)]
        enum Task {
            $($task{tx:oneshot::Sender<$ret>,$($arg:$ty,)*}),*
        }

        impl ExecutorConnection {
            $(pub async fn $fn(&self, $($arg: $ty),*) -> $ret {
                let (tx, rx) = oneshot::channel();
                self.0.send(Task::$task{tx,$($arg),*}).unwrap();
                rx.await.unwrap()
            })*
        }

        pub struct DbExecutor {
            rx: UnboundedReceiver<Task>,
            db: rusqlite::Connection,
        }

        impl DbExecutor {
            pub fn create(dbpath: &str) -> rusqlite::Result<(Self, ExecutorConnection)> {
                let (tx, rx) = unbounded_channel();
                let db = rusqlite::Connection::open(dbpath)?;
                db.execute_batch(include_str!("schema.sql"))?;
                tracing::info!("Database connected ({})", dbpath);
                Ok((Self { rx, db }, ExecutorConnection(tx)))
            }

            pub fn run(mut self) {
                while let Some(task) = self.rx.blocking_recv() {
                    let before = Instant::now();
                    tracing::debug!("received task {:?}", task);
                    match task {
                        $(Task::$task{tx,$($arg),*} => {
                            let $db = &mut self.db;
                            let _e = tx.send((||$handler)());
                        })*
                    }
                    let runtime = Instant::now().duration_since(before).as_secs_f64() / 1000.0;
                    tracing::debug!("task took {}ms", runtime);
                }
            }
        }
    };
}

generate_executor! {
    AddPost / add_post, (db, content: String, ip: String, show_ip: bool, time: u64, image: Option<(Bytes, PathBuf)>) => Result<()> {
        if let Some((image_data, image_path)) = image {
            let tx = db.transaction()?;
            let mut stmt = tx.prepare_cached(queries::INSERT_POST)?;
            todo!()
        } else {
            let mut stmt = db.prepare_cached(queries::INSERT_POST)?;
            stmt.execute(params![content, ip, show_ip, <Option<String>>::None, time])?;
        }
        
        Ok(())
    }
}