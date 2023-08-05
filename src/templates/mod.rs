use askama::Template;

use self::models::Post;

pub mod models;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub posts: Vec<Post>
}
