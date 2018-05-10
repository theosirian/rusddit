#![feature(core_intrinsics)]

extern crate rawr;
use rawr::auth::ApplicationOnlyAuthenticator;
use rawr::prelude::*;

extern crate termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, raw, screen, style};

#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
use slog::Drain;

extern crate clap;
extern crate nanoid;
extern crate sys_info;
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
	let file = OpenOptions::new().create(true).write(true).truncate(true).open(log_path).unwrap();
	let drain = slog_json::Json::new(file).set_pretty(false).set_newlines(true).build().fuse();
	let drain = slog_async::Async::new(drain).build().fuse();
	let _log = slog::Logger::root(drain, o!());

	// RAW MODE
	let mut stdout = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());
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
	let client = RedditClient::new(&user_agent, ApplicationOnlyAuthenticator::new("pam5L9so0-c4mQ", &device_id));
	let subreddit = client.subreddit("rust");
	let hot_listing = subreddit.hot(ListingOptions::default()).expect("Could not fetch post listing!");

	let mut posts = Vec::new();
	for post in hot_listing.take(50) {
		posts.push(post);
	}

	let size = termion::terminal_size().unwrap();
	let mut state = Viewer { width: size.0,
	                         height: size.1,

	                         subreddit: String::from("rust"),
	                         posts: posts,
	                         index: Indexing::FromTop(0, 0, 0),

	                         card_text: 4,
	                         card_margin: 1, };

	state.index = Indexing::FromTop(0, 0, state.max_cards());

	state.draw(&mut stdout);
	for c in stdin.keys() {
		match c.unwrap() {
			Key::Char('q') => break,
			Key::Char('j') => {
				state.index = match state.index {
					Indexing::FromBottom(current, top, bottom) => {
						let current = current + 1;
						if bottom <= current {
							let diff = current - bottom + 1;
							Indexing::FromBottom(current, top + diff, bottom + diff)
						} else {
							Indexing::FromBottom(current, top, bottom)
						}
					}
					Indexing::FromTop(current, top, bottom) => {
						let current = current + 1;
						if bottom <= current {
							let diff = current - bottom;
							Indexing::FromBottom(current, top + diff, bottom + diff)
						} else {
							if current == bottom - 1 {
								Indexing::FromBottom(current, top, bottom)
							} else {
								Indexing::FromTop(current, top, bottom)
							}
						}
					}
					Indexing::Center(current) => Indexing::Center(current + 1),
				}
			}
			Key::Char('k') => {
				state.index = match state.index {
					Indexing::FromBottom(current, top, bottom) => {
						let current = if current == 0 { 0 } else { current - 1 };
						if top > current {
							let diff = top - current;
							Indexing::FromTop(current, top - diff, bottom - diff)
						} else {
							if current == top {
								Indexing::FromTop(current, top, bottom)
							} else {
								Indexing::FromBottom(current, top, bottom)
							}
						}
					}
					Indexing::FromTop(current, top, bottom) => {
						let current = if current == 0 { 0 } else { current - 1 };
						if top > current {
							let diff = top - current;
							Indexing::FromTop(current, top - diff, bottom - diff)
						} else {
							Indexing::FromTop(current, top, bottom)
						}
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

	write!(stdout, "{}{}{}{}", clear::All, style::Reset, cursor::Goto(1, 1), cursor::Show).unwrap();
	stdout.flush().unwrap();
}

enum Indexing {
	FromBottom(u16, u16, u16),
	FromTop(u16, u16, u16),
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

	fn max_cards(&self) -> u16 {
		let h = self.height - 2;
		let c = self.card_height();
		(h / c) + 1
	}

	fn draw(&self,
	        stdout: &mut screen::AlternateScreen<raw::RawTerminal<std::io::Stdout>>) {
		write!(stdout, "{}{}{}", clear::All, cursor::Hide, color::Bg(color::Black)).unwrap();
		for j in 0..self.height {
			write!(stdout, "{}", cursor::Goto(1, j + 1)).unwrap();
			for i in 0..self.width {
				write!(stdout, " ").unwrap();
			}
		}

		let (mode, selection, first, last) = match self.index {
			Indexing::FromTop(current, top, bottom) => (String::from("free:t"), current, top, bottom),
			Indexing::FromBottom(current, top, bottom) => (String::from("free:b"), current, top, bottom),
			Indexing::Center(current) => {
				let half = self.max_cards();
				let (above, below) = if current >= half {
					(half, self.max_cards() - half)
				} else {
					(current, self.max_cards() - current)
				};
				(String::from("center"), current, current - above, current + below)
			}
		};

		write!(stdout, "{}{}{}{} mode={} index={} top={} bottom={}", cursor::Goto(1,1), style::Bold, self.subreddit, style::Reset, mode, selection, first, last).unwrap();

		let reset = format!("{}{}{}", color::Fg(color::Reset), style::NoBold, style::NoUnderline);

		let mut lines = Vec::new();
		for index in first..last {
			let post = &self.posts[index as usize];

			// Title line.
			let mut line = Vec::new();
			let setup = format!("{}{}", color::Fg(color::White), style::Bold);
			write!(&mut line, "{}{}. {}{}", setup, index + 1, post.title(), reset).unwrap();
			lines.push(line);

			// Link or self line
			let mut line = Vec::new();
			let setup = format!("{}{}", color::Fg(color::Blue), style::Underline);
			let link = if let Some(url) = post.link_url() {
				url
			} else {
				format!("self.{}", self.subreddit)
			};
			write!(&mut line, "{}{}{}", setup, link, reset).unwrap();
			lines.push(line);

			// Write score, vote, time, comment count
			let mut line = Vec::new();
			let setup = format!("{}", color::Fg(color::White));
			let vote = if let Some(vote) = post.likes() {
				match vote {
					true => format!("{}+{}", color::Fg(color::Green), color::Fg(color::Reset)),
					false => format!("{}-{}", color::Fg(color::Red), color::Fg(color::Reset)),
				}
			} else {
				format!("{}o{}", color::Fg(color::White), color::Fg(color::Reset))
			};
			write!(&mut line, "{}{} pts {} {} - {} comments{}", setup, post.score(), vote, post.created(), post.reply_count(), reset).unwrap();
			lines.push(line);

			// Write user, subreddit, nsfw marker and flair
			let mut line = Vec::new();
			let setup = format!("{}", color::Fg(color::White));
			let user = format!("{}u/{}{}", color::Fg(color::Green), post.author().name, reset);
			let sub = format!("{}r/{}{}", color::Fg(color::Yellow), post.subreddit().name, reset);
			let nsfw = if post.nsfw() {
				format!("{}NSFW{}", color::Bg(color::Red), color::Bg(color::Reset))
			} else {
				String::from("")
			};
			let flair = if let Some(flair) = post.get_flair_text() {
				format!("{}[{}]{}", color::Fg(color::Magenta), flair, color::Fg(color::Reset))
			} else {
				String::from("")
			};
			write!(&mut line, "{}by {} in {} {} {}{}", setup, user, sub, nsfw, flair, reset).unwrap();
			lines.push(line);
			lines.push(Vec::new());
		}
		lines.pop();

		let height = self.height as usize - 2;
		match self.index {
			Indexing::FromBottom(_, _, _) => {
				let len = lines.len();
				let diff = if height > len { height - len } else { len - height };
				let lines = lines.split_off(diff);
				for (i, line) in lines.iter().enumerate() {
					write!(stdout, "{}", cursor::Goto(2, i as u16 + 2)).unwrap();
					stdout.write(&line[..]);
				}
				for i in 0..self.card_text {
					let pos = (i + ((selection - first) * self.card_height())) as usize;
					if diff > pos {
						continue;
					}
					let pos = pos + 2 - diff;
					write!(stdout, "{}{} {}", cursor::Goto(1, pos as u16), color::Bg(color::White), color::Bg(color::Reset),).unwrap();
				}
			}
			Indexing::FromTop(_, _, _) | Indexing::Center(_) => {
				if lines.len() > height {
					let _ = lines.split_off(height);
				}
				for (i, line) in lines.iter().enumerate() {
					write!(stdout, "{}", cursor::Goto(2, i as u16 + 2)).unwrap();
					stdout.write(&line[..]);
				}
				for i in 0..self.card_text {
					let pos = (i + ((selection - first) * self.card_height())) as usize;
					if pos > height {
						break;
					}
					let pos = pos + 2;
					write!(stdout, "{}{} {}", cursor::Goto(1, pos as u16), color::Bg(color::White), color::Bg(color::Reset),).unwrap();
				}
			}
		}

		stdout.flush().unwrap();
	}
}
