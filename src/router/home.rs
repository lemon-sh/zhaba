use std::net::SocketAddr;

use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, Multipart, Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
};

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

use super::{error, AppState};

const PAGE_SIZE: usize = 25;

#[derive(Deserialize)]
pub struct PageQuery {
    p: Option<usize>,
}

pub async fn home_get(
    State(state): State<AppState>,
    Query(query): Query<PageQuery>,
) -> Result<impl IntoResponse, Response<Body>> {
    let page = query.p.unwrap_or(0);
    let posts = state
        .db
        .posts_display(page * PAGE_SIZE, PAGE_SIZE)
        .await
        .map_err(error::err_into_500)?;
    Ok(templates::Index {
        posts,
        ..Default::default()
    })
}

pub async fn home_post(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mp: Multipart,
) -> Result<impl IntoResponse, Response<Body>> {
    let (content, image) = read_post_mp(mp).await.map_err(error::err_into_500)?;
    let Some(content) = content else {
        return Err(error::http_400())
    };

    let ip = addr.ip().to_string();
    let whois = whois::whois(state.cfg.whois_server, ip.as_str())
        .await
        .map_err(error::err_into_500)?;

    // for display
    let posts = state
        .db
        .posts_display(0, PAGE_SIZE)
        .await
        .map_err(error::err_into_500)?;

    let image = if let Some(bytes) = image {
        let fileid = Alphanumeric.sample_string(&mut thread_rng(), 32);
        // TODO: rewrite imghdr as a module of this project, it's dead simple
        let ext = match imghdr::from_bytes(&bytes) {
            Some(imghdr::Type::Png) => ".png",
            Some(imghdr::Type::Jpeg) => ".jpg",
            Some(imghdr::Type::Gif) => ".gif",
            Some(imghdr::Type::Webp) => ".webp",
            _ => {
                let flash = Flash::Error("Image format not supported".into());
                let mut response = templates::Index { flash, posts }.into_response();
                *response.status_mut() = StatusCode::BAD_REQUEST;
                return Ok(response);
            }
        };
        let filename = format!("{fileid}.{ext}");
        Some(InsertImage {
            bytes,
            directory: state.cfg.image_path.clone(),
            filename,
        })
    } else {
        None
    };

    state
        .db
        .post_insert(content, ip, whois, image)
        .await
        .map_err(error::err_into_500)?;

    let flash = Flash::Success("Post was added successfully".into());
    Ok(templates::Index { flash, posts }.into_response())
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
