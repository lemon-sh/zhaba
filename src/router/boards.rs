use std::{net::Ipv4Addr, sync::OnceLock};

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
    database::InsertImage,
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
    let (content, image) = read_post_mp(mp).await.map_err(error::err_into_500)?;
    let Some(content) = content else {
        return Err(error::http_400())
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

    let image = if let Some(bytes) = image {
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

    state
        .db
        .create_post(board_name, content, ip, whois, image)
        .await
        .map_err(error::err_into_500)?;

    session
        .insert(
            "flash",
            Flash::Success("Post was added successfully".into()),
        )
        .unwrap();

    Ok(Redirect::to(&redirect_uri))
}

#[derive(Deserialize)]
pub struct DateRangeQuery {
    y: Option<i32>,
    m: Option<u32>,
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

    let Some(start) = NaiveDate::from_ymd_opt(year, month, 1) else { return Err(error::http_400()) };

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

async fn read_post_mp(mut mp: Multipart) -> color_eyre::Result<(Option<String>, Option<Bytes>)> {
    let mut content = None;
    let mut image = None;
    while let Some(field) = mp.next_field().await? {
        match field.name() {
            Some("content") => content = Some(field.text().await?),
            Some("image") => image = Some(field.bytes().await?),
            _ => {}
        }
    }
    Ok((content, image))
}
