use crate::{
    router::{error, AppState},
    templates,
    templates::models::Flash,
};
use axum::{body::Body, extract::{Path, State}, http::{Request, Response}, middleware::Next, response::{IntoResponse, Redirect}, Form, TypedHeader};
use axum_sessions::extractors::{ReadableSession, WritableSession};
use rusqlite::ErrorCode;
use serde::Deserialize;
use crate::router::headers;
use crate::templates::models::Board;

pub async fn handle_home(
    State(state): State<AppState>,
    mut session: WritableSession,
) -> Result<impl IntoResponse, Response<Body>> {
    let boards = state.db.get_boards().await.map_err(error::err_into_500)?;
    let flash = session.get("flash").unwrap_or_default();
    if !matches!(flash, Flash::None) {
        session.remove("flash");
    }
    Ok(templates::AdminHome { flash, boards })
}

pub async fn handle_loginpage(session: ReadableSession) -> impl IntoResponse {
    if session.get_raw("admin").is_some() {
        Err(Redirect::to("/admin").into_response())
    } else {
        Ok(templates::Login::default())
    }
}

#[derive(Deserialize)]
pub struct LoginForm {
    user: String,
    pass: String,
}

pub async fn handle_login(
    State(state): State<AppState>,
    mut session: WritableSession,
    Form(login_form): Form<LoginForm>,
) -> impl IntoResponse {
    let Some(admin) = state.cfg.admins.iter().find(|a| a.name == login_form.user) else {
        return templates::Login {
            flash: Flash::Error("Invalid login".into()),
        }.into_response()
    };
    if admin.password != login_form.pass {
        return templates::Login {
            flash: Flash::Error("Invalid password".into()),
        }
        .into_response();
    }
    session.insert_raw("admin", login_form.user);
    Redirect::to("/admin").into_response()
}

pub async fn handle_logout(mut session: WritableSession) -> impl IntoResponse {
    session.destroy();
    Redirect::to("/")
}

#[derive(Deserialize)]
pub struct UpdateBoardForm {
    name: String,
    description: String,
    color: String,
}

pub async fn handle_createboard(
    State(state): State<AppState>,
    mut session: WritableSession,
    Form(create_form): Form<UpdateBoardForm>,
) -> impl IntoResponse {
    match state
        .db
        .create_board(create_form.name, create_form.description, parse_html_color(&create_form.color)?)
        .await
    {
        Ok(_) => session
            .insert("flash", Flash::Success("Board successfully created".into()))
            .unwrap(),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {
            session
                .insert("flash", Flash::Error("Board already exists".into()))
                .unwrap()
        }
        Err(e) => return Err(error::err_into_500(e)),
    }
    Ok(Redirect::to("/admin"))
}

pub async fn handle_deleteboard(
    State(state): State<AppState>,
    mut session: WritableSession,
    Path(board_id): Path<i64>,
) -> Result<impl IntoResponse, Response<Body>> {
    state.db.delete_board(board_id).await.map_err(error::err_into_500)?;
    session.insert("flash", Flash::Success("Board successfully deleted".into())).unwrap();
    Ok(Redirect::to("/admin"))
}

pub async fn handle_updateboard(
    State(state): State<AppState>,
    mut session: WritableSession,
    Path(board_id): Path<i64>,
    Form(update_form): Form<UpdateBoardForm>,
) -> Result<impl IntoResponse, Response<Body>> {
    state.db.update_board(Board {
        id: board_id,
        name: update_form.name,
        description: update_form.description,
        color: parse_html_color(&update_form.color)?,
    }).await.map_err(error::err_into_500)?;
    session.insert("flash", Flash::Success("Board successfully deleted".into())).unwrap();
    Ok(Redirect::to("/admin"))
}

pub async fn handle_deletepost(
    State(state): State<AppState>,
    mut session: WritableSession,
    TypedHeader(headers::Referer(referer)): TypedHeader<headers::Referer>,
    Path(post_id): Path<i64>,
) -> Result<impl IntoResponse, Response<Body>> {
    state.db.delete_post(post_id).await.map_err(error::err_into_500)?;
    session.insert("flash", Flash::Success("Post successfully deleted".into())).unwrap();

    Ok(Redirect::to(&referer))
}

pub async fn auth_middleware<B>(
    session: ReadableSession,
    request: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    if session.get_raw("admin").is_some() {
        drop(session);
        let resp = next.run(request).await;
        Ok(resp)
    } else {
        Err(Redirect::to("/admin/login"))
    }
}

fn parse_html_color(color: &str) -> Result<u32, Response<Body>> {
    let color_hex = color.get(1..).ok_or_else(|| error::http_400())?;
    u32::from_str_radix(color_hex, 16).map_err(|_| error::http_400())
}
