use askama::Template;

pub mod models;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub boards: Vec<models::Board>,
}

#[derive(Template)]
#[template(path = "posts.html")]
pub struct Posts {
    pub flash: models::Flash,
    pub board: models::Board,
    pub posts: Vec<models::Post>,
}
