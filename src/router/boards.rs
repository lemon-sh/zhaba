use std::{net::Ipv4Addr, num::ParseIntError, sync::OnceLock};

use axum::{
    body::{Body, Bytes},
    extract::{Multipart, Path, Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
    TypedHeader,
};

use axum_sessions::extractors::WritableSession;
use bbscope::{BBCode, BBCodeTagConfig};
use chrono::{Datelike, Months, NaiveDate, Utc};
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use serde::Deserialize;

use crate::{
    database::{CreatePostResult, InsertImage},
    imghdr,
    templates::{self, models::Flash},
    whois::{self},
};

use super::{error, headers, AppState};

static BBCODE: OnceLock<BBCode> = OnceLock::new();

fn init_bbcode() -> BBCode {
    let config = BBCodeTagConfig {
        accepted_tags: vec![
            "b".into(),
            "i".into(),
            "sup".into(),
            "sub".into(),
            "u".into(),
            "s".into(),
        ],
        ..Default::default()
    };
    BBCode::from_config(config, None).unwrap()
}

pub async fn handle_home(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, Response<Body>> {
    let boards = state.db.get_boards().await.map_err(error::err_into_500)?;
    Ok(templates::Index { boards })
}

pub async fn handle_post(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    mut session: WritableSession,
    TypedHeader(xforwardedfor): TypedHeader<headers::XForwardedFor>,
    mp: Multipart,
) -> Result<Redirect, Response<Body>> {
    let redirect_uri = format!("/{board_name}");
    let post = read_post_mp(mp).await.map_err(error::err_into_500)?;
    let Some(content) = post.content else {
        return Err(error::http_400());
    };
    if content.is_empty() {
        session
            .insert("flash", Flash::Error("Post content cannot be empty".into()))
            .unwrap();
        return Ok(Redirect::to(&redirect_uri));
    }
    if content.len() > state.cfg.max_post_length {
        session
            .insert(
                "flash",
                Flash::Error(
                    format!(
                        "Post content too long (max {} chars)",
                        state.cfg.max_post_length
                    )
                    .into(),
                ),
            )
            .unwrap();
        return Ok(Redirect::to(&redirect_uri));
    }
    let content = BBCODE.get_or_init(init_bbcode).parse(&content);

    let ip = xforwardedfor
        .0
        .into_iter()
        .next()
        .unwrap_or(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .to_string();
    let whois = whois::whois(&state.cfg.whois_server, &ip)
        .await
        .map_err(error::err_into_500)?;

    let image = if let Some(bytes) = post.image {
        if bytes.is_empty() {
            None
        } else if let Some(ext) = imghdr::imghdr(&bytes) {
            let mut filename = Alphanumeric.sample_string(&mut thread_rng(), 32);
            filename.push_str(ext);
            Some(InsertImage {
                bytes,
                directory: state.cfg.image_path.clone(),
                filename,
            })
        } else {
            session
                .insert(
                    "flash",
                    Flash::Error("Image format not supported or invalid image".into()),
                )
                .unwrap();
            return Ok(Redirect::to(&redirect_uri));
        }
    } else {
        None
    };

    let reply = post.reply.transpose();
    if reply.is_err() {
        session
            .insert("flash", Flash::Error("Couldn't parse reply ID".into()))
            .unwrap();
        return Ok(Redirect::to(&redirect_uri));
    }

    if let CreatePostResult::InvalidReply = state
        .db
        .create_post(board_name, content, ip, whois, reply.unwrap(), image)
        .await
        .map_err(error::err_into_500)?
    {
        session
            .insert(
                "flash",
                Flash::Error("Couldn't find the post you are replying to".into()),
            )
            .unwrap();
    } else {
        session
            .insert(
                "flash",
                Flash::Success("Post was added successfully".into()),
            )
            .unwrap();
    }

    Ok(Redirect::to(&redirect_uri))
}

#[derive(Deserialize)]
pub struct DateRangeQuery {
    y: Option<i32>,
    m: Option<u32>,
}

pub struct PostResult {
    pub content: Option<String>,
    pub image: Option<Bytes>,
    pub reply: Option<Result<u64, ParseIntError>>,
}

pub async fn handle_view(
    State(state): State<AppState>,
    mut session: WritableSession,
    Path(board_name): Path<String>,
    range: Query<DateRangeQuery>,
) -> impl IntoResponse {
    let flash = session.get("flash").unwrap_or_default();
    if !matches!(flash, Flash::None) {
        session.remove("flash");
    }
    let board = state
        .db
        .get_board_by_name(board_name)
        .await
        .map_err(error::err_into_500)?;

    let Some(board) = board else {
        return Err(error::http_404());
    };

    let now = Utc::now();
    let year = range.y.unwrap_or_else(|| now.year());
    let month = range.m.unwrap_or_else(|| now.month());

    let Some(start) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Err(error::http_400());
    };

    let start_ts = start.and_hms_opt(0, 0, 0).unwrap().timestamp() as u64;
    let end_ts = (start + Months::new(1))
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .timestamp() as u64;

    let posts = state
        .db
        .get_posts(board.id, start_ts..end_ts)
        .await
        .map_err(error::err_into_500)?;

    Ok(templates::BoardView {
        board,
        year,
        admin: session.get_raw("admin"),
        flash,
        posts,
        month,
    })
}

async fn read_post_mp(mut mp: Multipart) -> color_eyre::Result<PostResult> {
    let mut content = None;
    let mut image = None;
    let mut reply = None;
    while let Some(field) = mp.next_field().await? {
        match field.name() {
            Some("content") => content = Some(field.text().await?),
            Some("image") => image = Some(field.bytes().await?),
            Some("reply") => {
                reply = {
                    let content = field.text().await?;
                    if !content.is_empty() {
                        Some(content.parse())
                    } else {
                        None
                    }
                }
            }
            _ => {}
        }
    }
    Ok(PostResult {
        content,
        image,
        reply,
    })
}
