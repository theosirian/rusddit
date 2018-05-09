#![feature(core_intrinsics)]

extern crate rawr;
use rawr::auth::ApplicationOnlyAuthenticator;
use rawr::prelude::*;

extern crate termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{color, cursor, style};

#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
use slog::Drain;

extern crate sys_info;

extern crate nanoid;

extern crate clap;

use std::env;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::process;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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

	// BUILD USER AGENT STRING
	let system_info = sys_info::os_release().unwrap();
	let device_id = nanoid::simple();
	let user_agent = format!(
	                         "{sys}:{dev_id}:{version} (by /u/osirian and /u/gchicha)",
	                         sys = system_info,
	                         dev_id = device_id,
	                         version = VERSION
	);

	// CREATE CLIENT
	let client = RedditClient::new(
	                               &user_agent,
	                               ApplicationOnlyAuthenticator::new("pam5L9so0-c4mQ", &device_id,),
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

	                         subreddit: String::from("rust"),
	                         posts: posts,
	                         index: Indexing::Free(0, 0, 0),

	                         card_text: 4,
	                         card_margin: 1, };

	state.index = Indexing::Free(0, 0, state.max_cards());

	state.draw(&mut stdout);
	for c in stdin.keys() {
		match c.unwrap() {
			Key::Char('q') => break,
			Key::Char('j') => {
				state.index = match state.index {
					Indexing::Free(current, top, bottom) => {
						let current = current + 1;
						let (top, bottom) = if bottom < current {
							let diff = current - bottom;
							(top + diff, bottom + diff)
						} else {
							(top, bottom)
						};
						Indexing::Free(current, top, bottom)
					}
					Indexing::Center(current) => Indexing::Center(current + 1),
				}
			}
			Key::Char('k') => {
				state.index = match state.index {
					Indexing::Free(current, top, bottom) => {
						let current = if current == 0 { 0 } else { current - 1 };
						let (top, bottom) = if top > current {
							let diff = top - current;
							(top - diff, bottom - diff)
						} else {
							(top, bottom)
						};
						Indexing::Free(current, top, bottom)
					}
					Indexing::Center(current) => {
						let current = if current == 0 { 0 } else { current - 1 };
						Indexing::Center(current)
					}
				}
			}
			_ => (),
		};

		state.draw(&mut stdout);
	}

	write!(
        stdout,
        "{}{}{}",
        termion::clear::All,
        cursor::Goto(1, 1),
        cursor::Show
        ).unwrap();
	stdout.flush().unwrap();
}

enum Indexing {
	Free(u16, u16, u16),
	Center(u16),
}

struct Viewer<'a> {
	width: u16,
	height: u16,

	subreddit: String,
	posts: Vec<rawr::structures::submission::Submission<'a>>,
	index: Indexing,

	card_text: u16,
	card_margin: u16,
}

impl<'a> Viewer<'a> {
	fn card_height(&self) -> u16 { self.card_text + self.card_margin }

	fn max_cards(&self) -> u16 { ((self.height - 1) / self.card_height()) + 1 }

	fn overflows(&self) -> bool { (self.height - 1) % self.card_height() != 0 }

	fn overflow_amount(&self) -> u16 { (self.height - 1) % self.card_height() }

	fn fits_overflow(&self,
	                 line: u16)
	                 -> bool {
		(self.card_height() - line) < (self.overflow_amount())
	}

	fn draw(&self,
	        stdout: &mut termion::raw::RawTerminal<std::io::Stdout>) {
		write!(
		       stdout,
		       "{}{}{}",
		       termion::clear::All,
		       cursor::Goto(1, 1),
		       cursor::Hide
		);

		write!(
		       stdout,
		       "{}{}{}",
		       termion::style::Bold,
		       self.subreddit,
		       termion::style::Reset
		);

		let (selection, first, last) = match self.index {
			Indexing::Free(current, top, bottom) => (current, top, bottom),
			Indexing::Center(current) => {
				let half = self.max_cards();
				let (above, below) = if current >= half {
					(half, self.max_cards() - half)
				} else {
					(current, self.max_cards() - current)
				};
				(current, current - above, current + below)
			}
		};
		let mut height = 2;
		let reset = format!(
		                    "{}{}{}",
		                    color::Bg(color::Reset),
		                    color::Fg(color::Reset),
		                    termion::style::Reset
		);

		for index in first..last {
			let post = &self.posts[index as usize];
			// Title line.
			if index != first || (selection == last && self.fits_overflow(0)) {
				write!(
				       stdout,
				       "{pos}{bg}{fg}{st}{number}. {title}{reset}",
				       pos = cursor::Goto(3, height),
				       bg = color::Bg(color::Black),
				       fg = color::Fg(color::White),
				       st = termion::style::Bold,
				       number = index + 1,
				       title = post.title(),
				       reset = reset,
				);
				height += 1;
				if height > self.height {
					break;
				}
			}
			// Link or self line
			if index != first || (selection == last && self.fits_overflow(1)) {
				let link = if let Some(url) = post.link_url() {
					url
				} else {
					format!("self.{}", self.subreddit)
				};
				write!(
				       stdout,
				       "{pos}{bg}{fg}{st}{link}{reset}",
				       pos = cursor::Goto(3, height,),
				       bg = color::Bg(color::Black,),
				       fg = color::Fg(color::Blue,),
				       st = termion::style::Underline,
				       link = link,
				       reset = reset,
				);
				height += 1;
				if height > self.height {
					break;
				}
			}
			// Write score, vote, time, comment count
			if index != first || (selection == last && self.fits_overflow(2)) {
				let vote = if let Some(vote) = post.likes() {
					match vote {
						true => format!("{}+{}", color::Fg(color::Green), color::Fg(color::Reset)),
						false => format!("{}-{}", color::Fg(color::Red), color::Fg(color::Reset)),
					}
				} else {
					format!("{}o{}", color::Fg(color::White), color::Fg(color::Reset))
				};
				write!(
				       stdout,
				       "{pos}{bg}{fg}{score} pts {vote} {time} - {comments} comments{reset}\r",
				       pos = cursor::Goto(3, height),
				       bg = color::Bg(color::Black),
				       fg = color::Fg(color::White),
				       score = post.score(),
				       vote = vote,
				       time = post.created(),
				       comments = post.reply_count(),
				       reset = reset,
				);
				height += 1;
				if height > self.height {
					break;
				}
			}
			// Write user, subreddit, nsfw marker and flair
			if index != first || (selection == last && self.fits_overflow(3)) {
				let nsfw = if post.nsfw() {
					format!("{}NSFW{}", color::Bg(color::Red), color::Bg(color::Reset))
				} else {
					String::from("")
				};
				let flair = if let Some(flair) = post.get_flair_text() {
					format!(
					        "{}[{}]{}",
					        color::Fg(color::Magenta),
					        flair,
					        color::Fg(color::Reset)
					)
				} else {
					String::from("")
				};
				write!(
				       stdout,
				       "{pos}{bg}{fg}{user} {sub}{subreddit} {nsfw} {flair}{reset}",
				       pos = cursor::Goto(3, height,),
				       bg = color::Bg(color::Black,),
				       fg = color::Fg(color::Green,),
				       user = post.author().name,
				       sub = color::Fg(color::Yellow,),
				       subreddit = post.subreddit().name,
				       nsfw = nsfw,
				       flair = flair,
				       reset = reset,
				);
				height += 1;
				if height > self.height {
					break;
				}
			}
			height += self.card_margin;
			if height > self.height {
				break;
			}
		}

		for i in 0..self.card_text {
			write!(
			       stdout,
			       "{}{} {}",
			       cursor::Goto(1, 2 + i + selection * self.card_height()),
			       color::Bg(color::White),
			       color::Bg(color::Reset),
			);
		}

		stdout.flush().unwrap();
	}
}
