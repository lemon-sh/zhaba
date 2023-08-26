use askama::Template;

use self::models::{Post, Flash};

pub mod models;

#[derive(Template, Default)]
#[template(path = "index.html")]
pub struct Index {
    pub flash: Flash,
    pub posts: Vec<Post>
}
