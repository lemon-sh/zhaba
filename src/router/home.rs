use std::net::Ipv4Addr;

use axum::{
    body::{Body, Bytes},
    extract::{Multipart, Query, State},
    http::{Response, StatusCode},
    response::{IntoResponse, Redirect},
    TypedHeader,
};

use axum_sessions::extractors::WritableSession;
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

const PAGE_SIZE: usize = 25;

#[derive(Deserialize)]
pub struct PageQuery {
    p: Option<usize>,
}

pub async fn handle_home(
    State(state): State<AppState>,
    mut session: WritableSession,
    Query(query): Query<PageQuery>,
) -> Result<impl IntoResponse, Response<Body>> {
    let page = query.p.unwrap_or(0);
    let posts = state
        .db
        .posts_display(page * PAGE_SIZE, PAGE_SIZE)
        .await
        .map_err(error::err_into_500)?;
    let flash = session.get("flash").unwrap_or_default();
    if matches!(flash, Flash::None) {
        session.remove("flash");
    }
    Ok(templates::Index { posts, flash })
}

pub async fn handle_add(
    State(state): State<AppState>,
    mut session: WritableSession,
    TypedHeader(xforwardedfor): TypedHeader<headers::XForwardedFor>,
    mp: Multipart,
) -> Result<impl IntoResponse, Response<Body>> {
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
                    .posts_display(0, PAGE_SIZE)
                    .await
                    .map_err(error::err_into_500)?;

                let flash = Flash::Error("Image format not supported".into());
                let mut response = templates::Index { flash, posts }.into_response();
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

    state
        .db
        .post_insert(content, ip, whois, image)
        .await
        .map_err(error::err_into_500)?;

    session
        .insert(
            "flash",
            Flash::Success("Post was added successfully".into()),
        )
        .unwrap();
    Ok(Redirect::to("/").into_response())
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
