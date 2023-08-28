use std::{borrow::Cow, net::Ipv4Addr, str::FromStr};
use std::sync::OnceLock;

use axum::{
    body::{Body, Bytes},
    extract::{Multipart, Path, Query, State},
    http::{Response, StatusCode},
    response::{IntoResponse, Redirect},
    TypedHeader,
};

use axum_sessions::extractors::WritableSession;
use bbscope::{BBCode, BBCodeTagConfig};
use chrono::NaiveDate;
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
        accepted_tags: vec!["b".into(), "i".into(), "sup".into(), "sub".into(), "u".into(), "s".into()],
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
) -> Result<axum::response::Response, Response<Body>> {
    async fn bad_request_and_flash(
        state: &AppState,
        board_name: String,
        error: Cow<'static, str>,
    ) -> Result<axum::response::Response, Response<Body>> {
        let board = state
            .db
            .get_board_by_name(board_name)
            .await
            .map_err(error::err_into_500)?;

        let posts = state
            .db
            .posts_display(board.id, state.cfg.page_size)
            .await
            .map_err(error::err_into_500)?;

        let flash = Flash::Error(error);
        let mut response = templates::Posts {
            board,
            flash,
            posts,
        }
        .into_response();
        *response.status_mut() = StatusCode::BAD_REQUEST;
        Ok(response)
    }

    let (content, image) = read_post_mp(mp).await.map_err(error::err_into_500)?;
    let Some(content) = content else {
        return Err(error::http_400())
    };
    if content.is_empty() {
        return bad_request_and_flash(&state, board_name, "Post content cannot be empty".into())
            .await;
    }
    if content.len() > state.cfg.max_post_length {
        return bad_request_and_flash(
            &state,
            board_name,
            format!(
                "Post content too long (max {} chars)",
                state.cfg.max_post_length
            )
            .into(),
        )
        .await;
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
            return bad_request_and_flash(
                &state,
                board_name,
                "Image format not supported or invalid image".into(),
            )
            .await;
        }
    } else {
        None
    };

    let redirect_uri = format!("/{board_name}");

    state
        .db
        .post_insert(board_name, content, ip, whois, image)
        .await
        .map_err(error::err_into_500)?;

    session
        .insert(
            "flash",
            Flash::Success("Post was added successfully".into()),
        )
        .unwrap();
    Ok(Redirect::to(&redirect_uri).into_response())
}

#[derive(Deserialize)]
pub struct DateRangeQuery {
    s: Option<String>,
    e: Option<String>,
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
    let posts = if let (Some(start), Some(end)) = (&range.s, &range.e) {
        let Ok(start) = NaiveDate::from_str(start) else { return Err(error::http_400()) };
        let Ok(end) = NaiveDate::from_str(end) else { return Err(error::http_400()) };
        let start_ts = start.and_hms_opt(23, 59, 59).unwrap().timestamp() as u64;
        let end_ts = end.and_hms_opt(23, 59, 59).unwrap().timestamp() as u64;
        state
            .db
            .posts_display_range(board.id, start_ts..end_ts, state.cfg.page_size)
            .await
            .map_err(error::err_into_500)?
    } else {
        state
            .db
            .posts_display(board.id, state.cfg.page_size)
            .await
            .map_err(error::err_into_500)?
    };

    println!("board {board:?} posts {posts:?}");

    Ok(templates::Posts {
        board,
        flash,
        posts,
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
