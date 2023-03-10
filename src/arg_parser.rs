use clap::{ArgGroup, Parser};

#[derive(Parser)]
#[command(group(
    ArgGroup::new("reddit")
        .required(true)
        .args(["username", "subreddit"])
))]
#[command(group(
    ArgGroup::new("type")
        .required(true)
        .args(["query", "listing"])
))]
/// Downloads posts from a Reddit user's posts
pub struct Args {
    /// The username of the Reddit user
    #[clap(short, long)]
    pub username: Option<String>,

    /// The subreddit to download posts from
    #[clap(short, long)]
    pub subreddit: Option<String>,

    /// The number of posts to download
    #[clap(short, long, default_value = "25")]
    pub limit: u32,

    /// The parameter to search by
    #[clap(short, long)]
    pub query: Option<String>,

    /// The type of posts to download, either controversial, best, hot, new, random, rising, top
    #[clap(long, default_value = "top", value_parser = parser_listing)]
    pub listing: String,

    /// The time period to download posts from
    #[clap(short, long, default_value = "all", value_parser = parser_time)]
    pub time: String,

    /// The output file to write the posts to
    #[clap(short, long, default_value = "data.json")]
    pub output: String,

    /// Download posts from the given subreddit
    #[clap(short, long)]
    pub download: bool,
}

fn parser_listing(value: &str) -> Result<String, String> {
    match value {
        "controversial" | "best" | "hot" | "new" | "random" | "rising" | "top" => {
            Ok(value.to_string())
        }
        _ => Err(format!(
            "Invalid listing type: {value}, must be one of: controversial, best, hot, new, random, rising, top"
        )),
    }
}

fn parser_time(value: &str) -> Result<String, String> {
    match value {
        "hour" | "day" | "week" | "month" | "year" | "all" => Ok(value.to_string()),
        _ => Err(format!(
            "Invalid time period: {value}, must be one of: hour, day, week, month, year, all"
        )),
    }
}
