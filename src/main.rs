mod arg_parser;
mod downloader;
mod types;

use chrono::Local;
use clap::Parser;
use log::{error, info, warn};
use owo_colors::OwoColorize;
use std::{error::Error, io::Write, sync::Arc};
use tokio::io::AsyncWriteExt;
use types::{Data, RedditResponseData};

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

    let rounds = number_of_calls(args.limit);

    let mut data = Vec::new();
    let mut remaining_limit = args.limit;
    let mut after = String::new();
    for round in 0..rounds {
        info!(
            "Fetching posts {}/{} | Remaining: {}",
            round + 1,
            rounds,
            remaining_limit.green()
        );
        let mut data_round = call_reddit(
            &subreddit,
            &args.query,
            &args.listing,
            remaining_limit,
            &args.time,
            &after,
        )
        .await
        .unwrap();

        if data_round.0.is_empty() {
            warn!("No more posts to fetch, skipping the rest of the calls");
            break;
        }

        after = data_round.0.last().unwrap().name.clone();

        remaining_limit -= data_round.0.len() as u32;
        data.append(&mut data_round.0);
    }

    // Download posts
    let mut downloaders = Vec::new();
    if args.download {
        info!("Downloading posts");
        // TODO - Remove clone
        let data = data.clone();
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

fn number_of_calls(limit: u32) -> u32 {
    if limit > 100 {
        (f64::from(limit) / 100.0).ceil() as u32
    } else {
        1
    }
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
    after: T,
) -> Result<RedditResponseData, Box<dyn Error>>
where
    T: AsRef<str>,
{
    let query_formatted = match &query {
        Some(query) => format!("&q={query}"),
        None => String::new(),
    };

    let after_formatted = if after.as_ref().is_empty() {
        String::new()
    } else {
        format!("&after={}", after.as_ref())
    };

    let listing_formatted = if query.is_some() {
        "search"
    } else {
        listing.as_ref()
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "https://www.reddit.com{}/{}.json?limit={}&t={}{}{}",
            username.as_ref(),
            listing_formatted,
            limit,
            time.as_ref(),
            query_formatted,
            after_formatted
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

async fn write_data_to_file<T>(data: Vec<Data>, filename: T) -> Result<(), Box<dyn Error>>
where
    T: AsRef<str>,
{
    let mut file = tokio::fs::File::create(filename.as_ref()).await?;
    file.write_all(serde_json::to_string_pretty(&data)?.as_bytes())
        .await?;

    info!("Successfully wrote data to {}", filename.as_ref());
    Ok(())
}
