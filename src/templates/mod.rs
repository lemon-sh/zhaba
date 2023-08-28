use askama::Template;
use models::{Board, Flash};
use chrono::{Utc, Datelike};

pub mod models;

// first year in the year dropdown
const STARTING_YEAR: i32 = 2023;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub boards: Vec<Board>,
}

#[derive(Template)]
#[template(path = "posts.html")]
pub struct Posts {
    pub flash: Flash,
    pub board: Board,
    pub year: i32,
    pub month: u32,
    pub posts: Vec<models::Post>,
}
