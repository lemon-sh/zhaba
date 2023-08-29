use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect};
use axum_sessions::extractors::ReadableSession;

pub async fn handle_home() {

}

pub async fn handle_loginpage() {

}

pub async fn handle_login() {

}

pub async fn auth_middleware<B>(session: ReadableSession, request: Request<B>, next: Next<B>) -> impl IntoResponse {
    if session.get_raw("admin").as_deref() == Some("1") {
        Ok(next.run(request).await)
    } else {
        Err(Redirect::to("/admin/login"))
    }
}