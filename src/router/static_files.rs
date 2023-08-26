use std::io::ErrorKind;

use askama_axum::IntoResponse;
use axum::{
    body::{boxed, Full, StreamBody},
    http::{header, StatusCode, Uri},
    response::Response, extract::State,
};
use rust_embed::RustEmbed;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use super::{error, AppState};

pub async fn image_handler(State(state): State<AppState>, uri: Uri) -> impl IntoResponse {
    let filename = uri.path().trim_start_matches('/').strip_prefix("img/").unwrap();
    let path = state.cfg.image_path.join(filename);
    let file = match File::open(path).await {
        Ok(o) => o,
        Err(e) if e.kind() == ErrorKind::NotFound => return Err(error::http_404()),
        Err(e) => return Err(error::err_into_500(e))
    };
    let body = StreamBody::new(ReaderStream::new(file));
    let mime = mime_guess::from_path(filename).first_or_octet_stream().to_string();
    
    Ok(([(header::CONTENT_TYPE, mime)], body))
}

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').strip_prefix("static/").unwrap().to_string();
    StaticFile(path)
}

#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticFiles;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match StaticFiles::get(path.as_str()) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap()
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(boxed(Full::from(error::HTML_404)))
                .unwrap(),
        }
    }
}
