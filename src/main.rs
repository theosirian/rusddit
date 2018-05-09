#![feature(core_intrinsics)]
extern crate clap;
extern crate hyper;
extern crate rawr;
extern crate reqwest;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
extern crate termion;

use hyper::header::{qitem, Accept, Headers};
use hyper::mime;

use rawr::auth::ApplicationOnlyAuthenticator;
use rawr::prelude::*;

use slog::Drain;

use reqwest::Client;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{color, style};

use std::env;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::process;
use std::time::Duration;

fn print_type_of<T>(_: &T) {
    println!("{}", unsafe { std::intrinsics::type_name::<T>() });
}

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

    let client = RedditClient::new(
        "your user agent here",
        ApplicationOnlyAuthenticator::new("pam5L9so0-c4mQ", "0123456789012345678901234"),
    );
    let subreddit = client.subreddit("rust");
    let hot_listing = subreddit
        .hot(ListingOptions::default())
        .expect("Could not fetch post listing!");

    let mut posts = Vec::new();
    for post in hot_listing.take(50) {
        posts.push(post);
    }

    let size = termion::terminal_size().unwrap();
    let mut state = Viewer {
        width: size.0,
        height: size.1,

        subreddit: String::from("rust"),
        posts: posts,
        index: 0,

        card_text: 2,
        card_margin: 1,
    };

    state.draw(&mut stdout);
    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('q') => break,
            Key::Char('j') => {
                state.index += 1;
            }
            Key::Char('k') => {
                state.index = if state.index == 0 { 0 } else { state.index - 1 };
            }
            _ => (),
        };

        state.draw(&mut stdout);
    }

    write!(
        stdout,
        "{}{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        termion::cursor::Show
    ).unwrap();
    stdout.flush().unwrap();
}

struct Viewer<'a> {
    width: u16,
    height: u16,

    subreddit: String,
    posts: Vec<rawr::structures::submission::Submission<'a>>,
    index: u16,

    card_text: u16,
    card_margin: u16,
}

impl<'a> Viewer<'a> {
    fn card_height(&self) -> u16 {
        self.card_text + self.card_margin
    }

    fn max_cards(&self) -> u16 {
        ((self.height - 1) / self.card_height()) + 1
    }

    fn draw(&self, stdout: &mut termion::raw::RawTerminal<std::io::Stdout>) {
        write!(
            stdout,
            "{}{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            termion::cursor::Hide
        );

        write!(stdout, "{}", self.subreddit);

        let mut height = 2;
        let mut index: usize = self.index as usize;
        for i in 0..21 {
            let post = &self.posts[index];
            write!(stdout, "{}", termion::cursor::Goto(3, height),);
            write!(stdout, "{}. ", index + 1);
            if post.nsfw() {
                write!(
                    stdout,
                    "{}{}[NSFW] {}{}",
                    termion::color::Bg(color::Red),
                    termion::color::Fg(color::White),
                    termion::color::Bg(color::Reset),
                    termion::color::Fg(color::Reset),
                );
            }
            write!(stdout, "{}", post.title());
            height += 1;
            if height > self.height {
                break;
            }
            write!(stdout, "{}", termion::cursor::Goto(3, height),);
            if post.is_self_post() {
                write!(stdout, "self.{}", self.subreddit);
            } else if let Some(url) = post.link_url() {
                write!(stdout, "{}", url);
            } else {
                unreachable!();
            }
            height += 2;
            if height > self.height {
                break;
            }
            index += 1;
        }

        for i in 0..self.card_text {
            write!(
                stdout,
                "{}{} {}",
                termion::cursor::Goto(1, 2 + i + self.index * self.card_height()),
                termion::color::Bg(color::White),
                termion::color::Bg(color::Reset),
            );
        }

        stdout.flush().unwrap();
    }
}
