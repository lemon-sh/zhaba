use askama::Template;
use chrono::{Datelike, Utc};
use models::{Board, Flash};

pub mod models;

// first year in the year dropdown
const STARTING_YEAR: i32 = 2023;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub boards: Vec<Board>,
}

#[derive(Template)]
#[template(path = "board.html")]
pub struct BoardView {
    pub flash: Flash,
    pub admin: Option<String>,
    pub board: Board,
    pub year: i32,
    pub month: u32,
    pub posts: Vec<models::Post>,
}

#[derive(Template, Default)]
#[template(path = "login.html")]
pub struct Login {
    pub flash: Flash,
}

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminHome {
    pub flash: Flash,
    pub boards: Vec<Board>,
}

#[derive(Template)]
#[template(path = "about.html")]
pub struct About;
