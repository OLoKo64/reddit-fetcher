mod arg_parser;
mod downloader;
mod types;

use arg_parser::Args;
use chrono::Local;
use clap::Parser;
use log::{error, info, warn};
use owo_colors::OwoColorize;
use std::{error::Error, io::Write, sync::Arc};
use tokio::{io::AsyncWriteExt, task::JoinSet};
use types::{Data, RedditResponseData};

use crate::types::RedditResponse;

#[tokio::main]
async fn main() {
    let args = arg_parser::Args::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());

            writeln!(
                buf,
                "{} | {} | {}",
                style.value(Local::now().format("%Y-%m-%d %H:%M:%S")),
                style.value(record.level()),
                record.args()
            )
        })
        .init();

    let subreddit = get_subreddit(&args.subreddit, &args.username, &args.query).unwrap();

    let data = reddit_caller(&args, &subreddit).await.unwrap();

    // Download posts
    let mut downloaders = JoinSet::new();
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
            // If you want it to be ordered you need to use a normal Vec and push the join handle, like std threads
            downloaders.spawn(async move {
                let post_reddit =
                    downloader::RedditPost::new(output_folder.as_ref(), &post.url, &post.title);

                post_reddit.download_post().await.unwrap_or_else(|err| {
                    error!("{} | {}", post.title.red(), err.red());
                });
            });
        }
        while let Some(result) = downloaders.join_next().await {
            result.unwrap();
        }
    }

    write_data_to_file(data, args.output).await.unwrap();
}

async fn reddit_caller(args: &Args, subreddit: &str) -> Result<Vec<Data>, Box<dyn Error>> {
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
        let mut data_round = call_reddit(args, subreddit, remaining_limit, &after)
            .await
            .unwrap();

        if data_round.0.is_empty() {
            warn!(
                "{}",
                "No more posts to fetch, skipping the rest of the calls".yellow()
            );
            break;
        }

        after = data_round.0.last().unwrap().name.clone();

        remaining_limit -= data_round.0.len() as u32;
        data.append(&mut data_round.0);
    }
    Ok(data)
}

fn number_of_calls(limit: u32) -> u32 {
    if limit > 100 {
        (f64::from(limit) / 100.0).ceil() as u32
    } else {
        1
    }
}

fn get_subreddit(
    subreddit: &Option<String>,
    username: &Option<String>,
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
    args: &Args,
    subreddit: T,
    limit: u32,
    after: T,
) -> Result<RedditResponseData, Box<dyn Error>>
where
    T: AsRef<str>,
{
    let query_formatted = match &args.query {
        Some(query) => format!("&q={query}"),
        None => String::new(),
    };

    let after_formatted = if after.as_ref().is_empty() {
        String::new()
    } else {
        format!("&after={}", after.as_ref())
    };

    let listing_formatted = if args.query.is_some() {
        "search"
    } else {
        args.listing.as_ref()
    };

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("RedditDownloader/1.0 by u/RedditorDownloader"),
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .default_headers(headers)
        .build()?;
    let response = client
        .get(format!(
            "https://www.reddit.com{}/{}.json?limit={}&t={}{}{}",
            subreddit.as_ref(),
            listing_formatted,
            limit,
            args.time,
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

    dbg!(&data.len());

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
