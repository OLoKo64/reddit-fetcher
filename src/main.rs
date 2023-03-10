mod arg_parser;
mod downloader;
mod types;

use chrono::Local;
use clap::Parser;
use log::{error, info};
use owo_colors::OwoColorize;
use std::{error::Error, io::Write, sync::Arc};
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

    let subreddit = get_subreddit(args.subreddit, args.username, &args.query).unwrap();

    let data = call_reddit(
        &subreddit,
        &args.query,
        &args.listing,
        args.limit,
        &args.time,
    )
    .await
    .unwrap();

    // Download posts
    let mut downloaders = Vec::new();
    if args.download {
        info!("Downloading posts");
        // TODO - Remove clone
        let data = data.0.clone();
        let folder_suffix = if subreddit.trim().is_empty() {
            format!(
                "-{}",
                args.query
                    .unwrap_or_default()
                    .replace(['/', ' ', '\\'], "-")
            )
        } else {
            subreddit.replace('/', "-")
        };
        let output_folder = Arc::new(format!("posts{folder_suffix}"));

        tokio::fs::create_dir_all(output_folder.as_ref())
            .await
            .unwrap();

        for post in data {
            let output_folder = Arc::clone(&output_folder);
            downloaders.push(tokio::spawn(async move {
                let post_reddit =
                    downloader::RedditPost::new(output_folder.as_ref(), &post.url, &post.title);

                post_reddit.download_post().await.unwrap_or_else(|err| {
                    error!("{} | {}", post.title.red(), err.red());
                });
            }));
        }
        futures::future::join_all(downloaders).await;
    }

    write_data_to_file(data, args.output).await.unwrap();
}

fn get_subreddit(
    subreddit: Option<String>,
    username: Option<String>,
    query: &Option<String>,
) -> Result<String, String> {
    match (username, subreddit, query) {
        (_, _, Some(query)) => {
            info!("Searching for posts with query {}", query.green());
            Ok(String::new())
        }
        (Some(username), None, _) => {
            info!("Downloading posts from user {}", username.green());
            Ok(format!("/u/{username}"))
        }
        (None, Some(subreddit), _) => {
            info!("Downloading posts from subreddit {}", subreddit.green());
            Ok(format!("/r/{subreddit}"))
        }
        _ => Err("Invalid arguments".to_string()),
    }
}

async fn call_reddit<T>(
    username: T,
    query: &Option<String>,
    listing: T,
    limit: u32,
    time: T,
) -> Result<RedditResponseData, Box<dyn Error>>
where
    T: AsRef<str>,
{
    let query_formatted = match &query {
        Some(query) => format!("&q={query}"),
        None => String::new(),
    };

    let listing_formatted = if query.is_some() {
        "search"
    } else {
        listing.as_ref()
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "https://www.reddit.com{}/{}.json?limit={}&t={}{}",
            username.as_ref(),
            listing_formatted,
            limit,
            time.as_ref(),
            query_formatted
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
