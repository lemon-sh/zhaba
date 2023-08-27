use askama::Template;

use self::models::{Board, Flash};

pub mod models;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub boards: Vec<models::Board>,
}

#[derive(Template)]
#[template(path = "posts.html")]
pub struct Posts {
    pub flash: Flash,
    pub board: Board,
    pub posts: Vec<models::Post>,
}
