use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::whois::WhoisResult;

#[derive(Debug)]
pub struct Post {
    pub id: u64,
    pub content: String,
    pub image: Option<String>,
    pub ip: String,
    pub whois: Option<WhoisResult>,
    pub time: String,
}

#[derive(Debug)]
pub struct Board {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color: u32,
}

#[derive(Serialize, Deserialize)]
pub enum Flash {
    Success(Cow<'static, str>),
    Error(Cow<'static, str>),
    None,
}

impl Default for Flash {
    fn default() -> Self {
        Self::None
    }
}
