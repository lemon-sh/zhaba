use core::fmt;
use std::{fs, fs::OpenOptions, io::Write, ops::Range, path::PathBuf, time::Instant};

use axum::body::Bytes;
use chrono::NaiveDateTime;
use color_eyre::{eyre::eyre, Result};
use rusqlite::{params, OptionalExtension, Rows};
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
    AddPost / create_post, (db, board: String, content: String, ip: String, whois: Option<WhoisResult>, image: Option<InsertImage>) => Result<()> {
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
            stmt.execute(params![content, Some(image.filename), ip, asn, mnt, board])?;
            drop(stmt);
            tx.commit()?;
        } else {
            let mut stmt = db.prepare_cached(queries::INSERT_POST)?;
            stmt.execute(params![content, <Option<String>>::None, ip, asn, mnt, board])?;
        }
        Ok(())
    }

    DeletePost / delete_post, (db, id: i64, imgdir: PathBuf) => Result<bool> {
        let tx = db.transaction()?;
        let mut stmt = tx.prepare_cached(queries::DELETE_POST)?;
        let Some(image): Option<Option<String>> = stmt.query_row([id], |r| r.get(0)).optional()? else {
            return Ok(false)
        };
        if let Some(image) = image {
            let path = imgdir.join(&image);
            fs::remove_file(path)?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(true)
    }

    GetPosts / get_posts, (db, board: i64, range: Range<u64>) => Result<Vec<models::Post>> {
        let mut stmt = db.prepare_cached(queries::SELECT_POSTS_BOARD_RANGE)?;
        let rows = stmt.query(params![board, range.start, range.end])?;

        posts_from_rows(rows)
    }

    GetBoards / get_boards, (db,) => rusqlite::Result<Vec<models::Board>> {
        let mut stmt = db.prepare_cached(queries::SELECT_BOARDS)?;
        let mut rows = stmt.query([])?;
        let mut boards = Vec::new();
        while let Some(row) = rows.next()? {
            boards.push(models::Board { id: row.get(0)?, name: row.get(1)?, description: row.get(2)?, color: row.get(3)? });
        }
        Ok(boards)
    }

    GetBoardByName / get_board_by_name, (db, board: String) => rusqlite::Result<Option<models::Board>> {
        let mut stmt = db.prepare_cached(queries::SELECT_BOARD_BY_NAME)?;
        stmt.query_row([board], |r| Ok(models::Board { id: r.get(0)?, name: r.get(1)?, description: r.get(2)?, color: r.get(3)? })).optional()
    }

    CreateBoard / create_board, (db, name: String, description: String, color: u32) => rusqlite::Result<()> {
        let mut stmt = db.prepare_cached(queries::INSERT_BOARD)?;
        stmt.execute(params![name, description, color])?;
        Ok(())
    }

    DeleteBoard / delete_board, (db, id: i64) => rusqlite::Result<()> {
        let mut stmt = db.prepare_cached(queries::DELETE_BOARD)?;
        stmt.execute([id])?;
        Ok(())
    }

    UpdateBoard / update_board, (db, board: models::Board) => rusqlite::Result<()> {
        let mut stmt = db.prepare_cached(queries::UPDATE_BOARD)?;
        stmt.execute(params![board.name, board.description, board.color, board.id])?;
        Ok(())
    }
}

fn posts_from_rows(mut rows: Rows) -> Result<Vec<models::Post>> {
    let mut posts = Vec::new();
    while let Some(row) = rows.next()? {
        let timestamp = row.get(6)?;
        let time = NaiveDateTime::from_timestamp_opt(timestamp, 0)
            .ok_or_else(|| eyre!("Invalid timestamp {timestamp}"))?;
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
            time,
        });
    }
    Ok(posts)
}
