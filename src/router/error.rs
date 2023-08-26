use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use std::fmt::Debug;

pub const HTML_400: &[u8] = include_bytes!("html/400.html");
pub const HTML_404: &[u8] = include_bytes!("html/404.html");
pub const HTML_500: &[u8] = include_bytes!("html/500.html");

pub fn http_404() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(HTML_404))
        .unwrap()
}

pub fn http_400() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(HTML_400))
        .unwrap()
}

pub fn err_into_500<T: Debug>(e: T) -> Response<Body> {
    tracing::error!("{e:?}");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(HTML_500))
        .unwrap()
}
