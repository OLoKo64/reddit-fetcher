# reddit-fetcher

A simple script to fetch and download all images and videos from a subreddit or user.

## Usage

```
  -u, --username <USERNAME>    The username of the Reddit user
  -s, --subreddit <SUBREDDIT>  The subreddit to download posts from
  -l, --limit <LIMIT>          The number of posts to download [default: 25]
  -q, --query <QUERY>          The parameter to search by, it is not scoped to the subreddit
      --listing <LISTING>      The type of posts to download, either controversial, best, hot, new, random, rising, top [default: top]
  -t, --time <TIME>            The time period to download posts from [default: all]
  -o, --output <OUTPUT>        The output file to write the posts to [default: data.json]
  -d, --download               Download posts from the given subreddit
  -h, --help                   Print help
```

## Build

```bash
cargo build --release
```
