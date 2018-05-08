extern crate clap;
extern crate hyper;
extern crate reqwest;
#[macro_use]
extern crate slog;
extern crate rawr;
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

fn main() {
	// LOGGING SETUP
	let log_path = match env::var_os("HOME") {
		None => {
			println!("ERROR: Cannot open log file!");
			process::exit(1);
		}
		Some(path) => PathBuf::from(path).join(".config/rusddit/rusddit.log"),
	};
	let file = OpenOptions::new().create(true)
	                             .write(true)
	                             .truncate(true)
	                             .open(log_path)
	                             .unwrap();
	let drain = slog_json::Json::new(file).set_pretty(false)
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

	let client = RedditClient::new(
	                               "your user agent here",
	                               ApplicationOnlyAuthenticator::new(
		"pam5L9so0-c4mQ",
		"0123456789012345678901234",
	),
	);
	let subreddit = client.subreddit("rust");
	let hot_listing = subreddit.hot(ListingOptions::default())
	                           .expect("Could not fetch post listing!");

	let mut posts = Vec::new();
	for post in hot_listing.take(50) {
		posts.push(post);
	}

	let size = termion::terminal_size().unwrap();
	let mut state = Viewer { width: size.0,
	                         height: size.1,

	                         subreddit: "rust",
	                         posts: posts,
	                         index: 1,

	                         card_height: 2,
	                         card_margin: 1,
	                         max_cards: ((size.1 - 1) / 3) + 1, };

	state.draw(&stdout);
	for c in stdin.keys() {
		match c.unwrap() {
			Key::Char('q') => break,
			Key::Char('j') => {
				state.index += 1;
			}
			Key::Char('k') => {
				state.index = std::cmp::max(state.index - 1, 1);
			}
			_ => (),
		};

		state.draw(&stdout);
	}

	write!(stdout, "{}{}{}", termion::clear::All, termion::cursor::Goto(1, 1), termion::cursor::Show).unwrap();
	stdout.flush().unwrap();
}

struct Viewer {
	subreddit: String,
	posts: Vec<rawr::structures::submission::Submission<'a>>,
	index: u16,

	card_height: u16,
	card_margin: u16,
	max_cards: u16,
}

impl Viewer {
	fn draw(&self,
	        stdout: &termion::raw::RawTerminal) {
		write!(
		       stdout,
		       "{}{}{}",
		       termion::clear::All,
		       termion::cursor::Goto(1, 1),
		       termion::cursor::Hide
		);

		write!(stdout, "{}", self.subreddit);

		let mut height = 2;
		let mut index = self.index;
		for i in 0..21 {
			let post = &self.posts[(index - 1) as usize];
			write!(stdout, "{}", termion::cursor::Goto(3, height),);
			write!(stdout, "{}. ", index);
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

		for i in 0..self.card_height {
			write!(
			       stdout,
			       "{}{} {}",
			       termion::cursor::Goto(1, i + self.index * (self.card_height + 1)),
			       termion::color::Bg(color::White),
			       termion::color::Bg(color::Reset),
			);
		}

		stdout.flush().unwrap();
	}
}
