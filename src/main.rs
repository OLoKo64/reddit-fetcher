mod arg_parser;
mod downloader;
mod types;

use chrono::Local;
use clap::Parser;
use log::{error, info};
use owo_colors::OwoColorize;
use std::{error::Error, io::Write};
use tokio::io::AsyncWriteExt;
use types::RedditResponseData;

use crate::types::RedditResponse;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = arg_parser::Args::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} | {} | {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    let subreddit = get_subreddit(args.subreddit, args.username).unwrap();

    let data = call_reddit(subreddit, args.limit, args.time).await.unwrap();

    // Download posts
    let mut downloaders = Vec::new();
    if args.download {
        info!("Downloading posts");
        // TODO - Remove clone
        let data = data.0.clone();

        for post in data {
            let url = post.url;
            let filename = post.title;
            downloaders.push(tokio::spawn(async move {
                let post = downloader::RedditPost::new(&url, &filename);

                post.download_post().await.unwrap_or_else(|err| {
                    error!("{} {}", filename.red(), err.red());
                });
            }));
        }
        futures::future::join_all(downloaders).await;
    }

    write_data_to_file(data, args.output).await.unwrap();
}

fn get_subreddit(subreddit: Option<String>, username: Option<String>) -> Result<String, String> {
    match (username, subreddit) {
        (Some(username), None) => {
            info!("Downloading posts from user {}", username.green());
            Ok(format!("u/{username}"))
        }
        (None, Some(subreddit)) => {
            info!("Downloading posts from subreddit {}", subreddit.green());
            Ok(format!("r/{subreddit}"))
        }
        _ => Err("Invalid arguments".to_string()),
    }
}

async fn call_reddit<T>(
    username: T,
    limit: u32,
    time: T,
) -> Result<RedditResponseData, Box<dyn Error>>
where
    T: AsRef<str>,
{
    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "https://www.reddit.com/{}/top.json?limit={}&t={}",
            username.as_ref(),
            limit,
            time.as_ref()
        ))
        .send()
        .await?
        .json::<RedditResponse>()
        .await?;

    let data = response
        .data
        .children
        .into_iter()
        .map(|child| child.data)
        .collect::<Vec<_>>();

    info!("Successfully retrieved {} posts", data.len().green());

    Ok(RedditResponseData(data))
}

async fn write_data_to_file<T>(
    RedditResponseData(data): RedditResponseData,
    filename: T,
) -> Result<(), Box<dyn Error>>
where
    T: AsRef<str>,
{
    let mut file = tokio::fs::File::create(filename.as_ref()).await?;
    file.write_all(serde_json::to_string_pretty(&data)?.as_bytes())
        .await?;

    info!("Successfully wrote data to {}", filename.as_ref());
    Ok(())
}
