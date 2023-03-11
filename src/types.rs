use serde::{Deserialize, Serialize};

pub struct RedditResponseData(pub Vec<Data>);

#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    pub title: String,
    pub url: String,
    domain: String,
    permalink: String,
    subreddit: String,
    pub name: String,
    upvote_ratio: f64,
    created_utc: f64,
    total_awards_received: i32,
    score: i32,
    ups: i32,
    downs: i32,
    num_comments: i32,
    is_video: bool,
    pub id: String,
    over_18: bool,
}
#[derive(Serialize, Deserialize)]
pub struct Children {
    pub data: Data,
}
#[derive(Serialize, Deserialize)]
pub struct TopData {
    pub children: Vec<Children>,
}

#[derive(Serialize, Deserialize)]
pub struct RedditResponse {
    pub data: TopData,
}
