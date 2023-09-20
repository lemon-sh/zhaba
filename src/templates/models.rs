use chrono::NaiveDateTime;
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
    pub reply: Option<ReplyTo>,
    pub time: NaiveDateTime,
    pub board: u64,
}

#[derive(Debug)]
pub struct ReplyTo {
    pub id: u64,
    pub time: NaiveDateTime,
    pub board: u64,
    pub board_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Board {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color: u32,
}

#[derive(Default, Serialize, Deserialize)]
pub enum Flash {
    Success(Cow<'static, str>),
    Error(Cow<'static, str>),
    #[default]
    None,
}
