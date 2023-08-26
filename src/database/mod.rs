use core::fmt;
use std::{fs::OpenOptions, io::Write, path::PathBuf, time::Instant};

use axum::body::Bytes;
use chrono::NaiveDateTime;
use color_eyre::{eyre::eyre, Result};
use rusqlite::params;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{templates::models, whois::WhoisResult};

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
                    tracing::debug!("task took {}ms", Instant::now().duration_since(before).as_secs_f64() / 1000.0);
                }
            }
        }
    };
}

pub struct InsertImage {
    pub bytes: Bytes,
    pub directory: PathBuf,
    pub filename: String,
}

impl fmt::Debug for InsertImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InsertImage")
            .field("bytes", &"...")
            .field("directory", &self.directory)
            .field("filename", &self.filename)
            .finish()
    }
}

generate_executor! {
    AddPost / post_insert, (db, content: String, ip: String, whois: Option<WhoisResult>, image: Option<InsertImage>) => Result<()> {
        let (asn, mnt) = if let Some(whois) = whois {
            (Some(whois.asn), Some(whois.mnt))
        } else {
            (None, None)
        };
        if let Some(image) = image {
            let tx = db.transaction()?;
            let mut stmt = tx.prepare_cached(queries::INSERT_POST)?;
            let path = image.directory.join(&image.filename);
            OpenOptions::new().write(true).truncate(true).create_new(true).open(path)?.write_all(&image.bytes)?;
            stmt.execute(params![content, Some(image.filename), ip, asn, mnt])?;
            drop(stmt);
            tx.commit()?;
        } else {
            let mut stmt = db.prepare_cached(queries::INSERT_POST)?;
            stmt.execute(params![content, <Option<String>>::None, ip, asn, mnt])?;
        }
        Ok(())
    }

    GetPosts / posts_display, (db, offset: usize, page_size: usize) => Result<Vec<models::Post>> {
        let mut stmt = db.prepare_cached(queries::SELECT_POSTS)?;
        let mut rows = stmt.query([offset, page_size])?;

        let mut posts = Vec::new();
        while let Some(row) = rows.next()? {
            let timestamp = row.get(6)?;
            let time = NaiveDateTime::from_timestamp_opt(timestamp, 0).ok_or_else(|| eyre!("Invalid timestamp {timestamp}"))?;
            let time = time.format("%Y-%m-%d %H:%m").to_string();
            let whois = if let (Some(asn), Some(mnt)) = (row.get(4)?, row.get(5)?) {
                Some(WhoisResult { asn, mnt })
            } else {
                None
            };
            posts.push(models::Post {
                id: row.get(0)?,
                content: row.get(1)?,
                image: row.get(2)?,
                ip: row.get(3)?,
                whois,
                time
            });
        }

        Ok(posts)
    }
}
