use lazy_regex::regex;
use log::info;
use owo_colors::OwoColorize;
use std::{error::Error, path::Path};
use tokio::io::AsyncWriteExt;
use unicode_segmentation::UnicodeSegmentation;

pub struct RedditPost<'a> {
    url: &'a str,
    filename: &'a str,
}

impl<'a> RedditPost<'a> {
    pub fn new(url: &'a str, filename: &'a str) -> Self {
        Self { url, filename }
    }

    pub async fn download_post(&self) -> Result<(), Box<dyn Error>> {
        let url = self.parse_url()?;
        let mut filename = self.parse_filename();
        let file_extension = url.split('.').last().unwrap_or("jpg");
        if file_extension.is_empty()
            || ["/", r#"\"#].contains(&file_extension)
            || !["png", "jpg", "jpeg", "gif", "mp4"].contains(&file_extension)
        {
            return Err(
                format!("Failed to get file extension for {url}, skipping download").into(),
            );
        }

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download post {} | {}",
                url,
                response.status().red()
            )
            .into());
        }

        if !filename.ends_with(file_extension) {
            filename = format!("{filename}.{file_extension}");
        }

        let path = Path::new("posts").join(filename);
        tokio::fs::create_dir_all(&path.parent().unwrap()).await?;
        let file_with_path = path.to_str().unwrap();
        let mut file = tokio::fs::File::create(file_with_path).await?;
        file.write_all(response.bytes().await?.as_ref()).await?;

        info!("Successfully downloaded post {}", file_with_path.green());
        Ok(())
    }

    fn parse_url(&self) -> Result<String, String> {
        let is_url_file = lazy_regex::regex!(r#"\.[a-zA-Z0-9]+$"#);
        let imgur_gifv_regex = regex!(r#"https://i.imgur.com/.*.gifv"#);
        if !is_url_file.is_match(self.url) {
            return Err(format!("{} may not contain a file", self.url.yellow()));
        }
        let mut url = self.url.to_string();
        if imgur_gifv_regex.is_match(self.url) {
            url = self.url.replace("gifv", "mp4");
        }
        Ok(url)
    }

    fn parse_filename(&self) -> String {
        let filename = self.filename.replace(['\\', '/', ':'], "");
        filename.graphemes(true).take(50).collect()
    }
}
