use askama_axum::IntoResponse;
use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use std::fmt::Debug;

pub const HTML_404: &[u8] = include_bytes!("html/404.html");
pub const HTML_500: &[u8] = include_bytes!("html/500.html");

pub fn http_404() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(HTML_404))
        .unwrap()
}

pub fn err_into_500<T: Debug>(e: T) -> Response<Body> {
    tracing::error!("{e:?}");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(HTML_500))
        .unwrap()
}
