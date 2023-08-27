use std::{net::Ipv4Addr, str::FromStr};

use axum::{
    body::{Body, Bytes},
    extract::{Multipart, Path, Query, State},
    http::{Response, StatusCode},
    response::{IntoResponse, Redirect},
    TypedHeader,
};

use axum_sessions::extractors::WritableSession;
use chrono::NaiveDate;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use serde::Deserialize;

use crate::{
    database::InsertImage,
    templates::{self, models::Flash},
    whois::{self},
};

use super::{error, headers, AppState};

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
) -> impl IntoResponse {
    let (content, image) = read_post_mp(mp).await.map_err(error::err_into_500)?;
    let Some(content) = content else {
        return Err(error::http_400())
    };

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
        let mut filename = Alphanumeric.sample_string(&mut thread_rng(), 32);
        // TODO: rewrite imghdr as a module of this project, it's dead simple
        let ext = match imghdr::from_bytes(&bytes) {
            Some(imghdr::Type::Png) => ".png",
            Some(imghdr::Type::Jpeg) => ".jpg",
            Some(imghdr::Type::Gif) => ".gif",
            Some(imghdr::Type::Webp) => ".webp",
            _ => {
                let posts = state
                    .db
                    .posts_display(board_name.clone(), state.cfg.page_size)
                    .await
                    .map_err(error::err_into_500)?;

                let board = state
                    .db
                    .get_board_metadata(board_name)
                    .await
                    .map_err(error::err_into_500)?;

                let flash = Flash::Error("Image format not supported".into());
                let mut response = templates::Posts {
                    board,
                    flash,
                    posts,
                }
                .into_response();
                *response.status_mut() = StatusCode::BAD_REQUEST;
                return Ok(response);
            }
        };
        filename.push_str(ext);
        Some(InsertImage {
            bytes,
            directory: state.cfg.image_path.clone(),
            filename,
        })
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
        .get_board_metadata(board_name.clone())
        .await
        .map_err(error::err_into_500)?;
    let posts = if let (Some(start), Some(end)) = (&range.s, &range.e) {
        let Ok(start) = NaiveDate::from_str(start) else { return Err(error::http_400()) };
        let Ok(end) = NaiveDate::from_str(end) else { return Err(error::http_400()) };
        let start_ts = start.and_hms_opt(23, 59, 59).unwrap().timestamp() as u64;
        let end_ts = end.and_hms_opt(23, 59, 59).unwrap().timestamp() as u64;
        state
            .db
            .posts_display_range(board_name, start_ts..end_ts, state.cfg.page_size)
            .await
            .map_err(error::err_into_500)?
    } else {
        state
            .db
            .posts_display(board_name, state.cfg.page_size)
            .await
            .map_err(error::err_into_500)?
    };

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
