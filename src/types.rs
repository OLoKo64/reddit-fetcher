use serde::{Deserialize, Serialize};

pub struct RedditResponseData(pub Vec<Data>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub title: String,
    pub url: String,
    domain: String,
    permalink: String,
    upvote_ratio: f64,
    created_utc: f64,
    score: i32,
    ups: i32,
    downs: i32,
    num_comments: i32,
    is_video: bool,
    pub id: String,
    over_18: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Children {
    pub data: Data,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TopData {
    pub children: Vec<Children>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedditResponse {
    pub data: TopData,
}
