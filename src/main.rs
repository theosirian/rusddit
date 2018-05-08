extern crate clap;
extern crate hyper;
extern crate reqwest;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
extern crate termion;
extern crate rawr;

#[macro_use]
extern crate serde_derive;

use hyper::mime;
use hyper::header::{qitem, Accept, Headers};

use rawr::prelude::*;
use rawr::auth::ApplicationOnlyAuthenticator;

use slog::Drain;

use reqwest::{Client};

use termion::{color, style};
use termion::raw::IntoRawMode;
use termion::input::TermRead;
use termion::event::Key;

use std::env;
use std::process;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, Write};
use std::time::Duration;
use std::path::PathBuf;

/*
#[derive(Deserialize)]
struct RedditPostData {
    author: String
}

#[derive(Deserialize)]
struct RedditPost {
    kind: String,
    data: RedditPostData
}

#[derive(Deserialize)]
struct RedditRData {
    dist: usize,
    children: vec![RedditPost]
}

#[derive(Deserialize)]
struct RedditRResponse {
    kind: String,
    data: RedditRData
}
*/

fn main() {
    // LOGGING SETUP
    let log_path = match env::var_os("HOME") {
        None => {
            println!("ERROR: Cannot open log file!");
            process::exit(1);
        }
        Some(path) => PathBuf::from(path).join(".config/rusddit/rusddit.log"),
    };
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .unwrap();
    let drain = slog_json::Json::new(file)
        .set_pretty(false)
        .set_newlines(true)
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let _log = slog::Logger::root(drain, o!());

    // RAW MODE
    let mut stdout = stdout().into_raw_mode().unwrap();
    let stdin = stdin();

    // GRAB FRONT PAGE
    /*
    let mut headers = Headers::new();
    headers.set(Accept(vec![qitem(mime::APPLICATION_JSON)]));
    let client = Client::builder()
        .gzip(true)
        .timeout(Duration::from_secs(8))
        .default_headers(headers)
        .build()
        .unwrap();
    write!(
        stdout,
        "{}{} Loading Frontpage... {}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        termion::cursor::Hide
    ).unwrap();
    stdout.flush().unwrap();
    match client.get("https://reddit.com/r/front/hot.json").send() {
        Ok(mut res) => {
            if res.status().is_success() {
                let json = res.json().unwrap();
                write!(
                    stdout,
                    "{}{} Frontpage Loaded... {}{}",
                    termion::clear::All,
                    termion::cursor::Goto(1, 1),
                    json,
                    termion::cursor::Hide
                ).unwrap();
            } else {
                write!(
                    stdout,
                    "{}{} Loading Failed... {}",
                    termion::clear::All,
                    termion::cursor::Goto(1, 1),
                    termion::cursor::Hide
                ).unwrap();
            }
            stdout.flush().unwrap();

            for c in stdin.keys() {
                write!(
                    stdout,
                    "{}{}",
                    termion::cursor::Goto(1, 2),
                    termion::clear::CurrentLine
                ).unwrap();

                match c.unwrap() {
                    Key::Char('q') => break,
                    _ => println!("Other"),
                };

                stdout.flush().unwrap();
            }
        }
        Err(e) => {
            // IF IT FAILED, LOG THE ERROR AND SHOW MESSAGE TO USER
            error!(_log, "{:?}", e);
            writeln!(stdout, "ERROR").unwrap();
        }
    };
    */
        
    let client = RedditClient::new("your user agent here", ApplicationOnlyAuthenticator::new("pam5L9so0-c4mQ", "0123456789012345678901234"));
    let subreddit = client.subreddit("rust");
    let hot_listing = subreddit.hot(ListingOptions::default()).expect("Could not fetch post listing!");
    for post in hot_listing.take(50) {
        println!("{}", post.title());
    }


    write!(stdout, "{}", termion::cursor::Show).unwrap();
}
