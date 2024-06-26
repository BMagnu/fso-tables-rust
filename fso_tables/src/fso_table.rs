use std::cell::{RefCell};
use std::cmp::min;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::iter::Peekable;
use std::path::Path;
use std::str::Chars;
use regex::Regex;

#[derive(Debug)]
pub struct FSOParsingError {
	pub line: usize,
	pub reason: String,
	pub comments: Option<String>,
	pub version_string: Option<String>
}

impl Display for FSOParsingError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {	
		write!(f, "Error at line {}: {}", self.line, self.reason)
	}
}
impl Error for FSOParsingError{ }

pub trait FSOParser<'a> {
	fn get(&self) -> &str;

	fn line(&self) -> usize;

	fn consume(&self, count: usize);
	
	//Returns (comments, version_string) in this whitespace. Will break immediately after a version string
	fn consume_whitespace(&self, stop_on_newline: bool) -> (Option<String>, Option<String>) {
		thread_local! { static VERSION_SYNTAX: Regex = Regex::new(r"\A;;FSO\x20\d+(?:\x2E\d+)+;;").unwrap(); }
		
		let mut comments: Option<String> = None;
		let mut version: Option<String> = None;
		let mut linebreak_since_comment = true;

		loop {
			self.consume_whitespace_inline(&[]);
			let current = self.get();
			
			let add_to_comment;
			
			let mut current_char : Peekable<Chars> = current.chars().peekable();
			match current_char.next() {
				Some('\n') if stop_on_newline => { break; }
				Some('\n') => {
					linebreak_since_comment = true;
					self.consume(1);
					continue;
				}
				Some(';') => { 
					//Comment or Version
					if VERSION_SYNTAX.with(|regex| regex.is_match(current)) {
						//Version
						self.consume(2);
						version = Some(format!(";;{};;", self.read_until_target(";;", true)));
						break;
					}
					else {
						add_to_comment = format!("{}", self.read_until_target("\n", true));
						linebreak_since_comment = true;
					}
				}
				Some('/') if current_char.peek().is_some_and(|c| *c == '/')=> {
					//Comment
					add_to_comment = format!("{}", self.read_until_target("\n", true));
					linebreak_since_comment = true;
				}
				Some(start @ '!') | Some(start @ '/') if current_char.peek().is_some_and(|c| *c == '*') => {
					//Mutliline comment
					current_char.next();
					self.consume(2);
					let mut target = "*".to_string();
					target.push(start);
					add_to_comment = format!("{}*{}*{}", start, self.read_until_target(target.as_str(), true), start);
				}
				_ => { break; }
			}
			if comments.is_none() {
				comments = Some(String::new());
			}
			if let Some(comment) = &mut comments {
				*comment = format!("{}{}{}", comment, if linebreak_since_comment { "" } else { "\n" }, add_to_comment);
				linebreak_since_comment = false;
			}
		}
		
		return (comments, version)
	}
	
	//Consumes whitespace and whitespace-likes (such as commas, tentatively)
	fn consume_whitespace_inline<const N: usize>(&self, also_consume: &[char;N]) {
		let current = self.get();
		let whitespaces = current.chars().take_while(|c| (*c != '\n' && c.is_whitespace()) || *c == ',' || also_consume.contains(c)).fold(0, |sum, c| sum + c.len_utf8());
		self.consume(whitespaces);
	}
	
	fn read_until_whitespace(&self) -> String {
		let current = self.get();
		let cropped = current.chars().take_while(|c| !c.is_whitespace() && *c != ',').collect::<String>();
		self.consume(cropped.len());
		cropped
	}
	
	fn read_until_target(&self, target: &str, consume_target: bool) -> &str {
		let current = self.get();
		let content_size = current.find(target).unwrap_or_else(|| current.len());
		self.consume(content_size + if consume_target { target.len() } else { 0 });
		&current[..content_size]
	}
	
	//Notably, this also does not include post-line comments!
	//Consumes until (excl) the last whitespace or first comment, or until (incl) the first char in also_stop
	fn read_until_last_whitespace_of_line_or_stop<const N: usize>(&self, also_stop: &[char;N]) -> &str {
		let current = self.get();
		let mut current_pos = 0usize;
		let mut last_non_whitespace = 0usize;
		let mut consume_until = 0usize;
		
		for c in current.chars() {
			current_pos += c.len_utf8();
			if also_stop.contains(&c) {
				consume_until = current_pos;
				break;
			}
			else if c == '\n' || c == ';' {
				break;
			}
			else if !c.is_whitespace() {
				last_non_whitespace = current_pos;
				consume_until = current_pos;
			}
		}
		
		self.consume(consume_until);
		return &current[..last_non_whitespace]
	}
	
	fn consume_string(&self, expect: &str) -> Result<(), FSOParsingError> {
		if self.get().starts_with(expect) {
			self.consume(expect.len());
			Ok(())
		}
		else { 
			let current = self.get();
			Err( FSOParsingError { reason: format!("Expected \"{}\", got {}", expect, &current[..min(current.len(), expect.len())]), line: self.line(), comments: None, version_string: None } )
		}
	}
}

#[derive(PartialEq)]
pub enum FSOBuilderListState {
	MutlilineList,
	InlineList
}

#[derive(Default)]
pub struct FSOBuilderState {
	pub list_state: Vec<FSOBuilderListState>
}

pub trait FSOBuilder {
	fn append(&mut self, content: &str);

	fn spew(self) -> String;

	fn get_state(&mut self) -> &mut FSOBuilderState;
}

#[derive(Default)]
struct FSOParserState {
	pos: usize,
	line: usize
}

pub struct FSOTableFileParser {
	original: String,
	state: RefCell<FSOParserState>
}
impl FSOTableFileParser {
	pub fn new(path: &Path) -> Result<Self, FSOParsingError>{
		let mut s = String::new();
		
		let mut file = match File::open(&path) {
			Ok(file) => { file }
			Err(err) => { return Err( FSOParsingError { reason: format!("Could not open file {}! Reason: {}.", path.to_string_lossy(), err), line: 0, comments: None, version_string: None }) }
		};

		match file.read_to_string(&mut s) {
			Ok(_) => {  }
			Err(err) => { return Err( FSOParsingError { reason: format!("Could not read from file {}! Reason: {}.", path.to_string_lossy(), err), line: 0, comments: None, version_string: None }) }
		};

		let parser = FSOTableFileParser {
			original: s,
			state: RefCell::new(FSOParserState::default())
		};

		Ok(parser)
	}
}

impl FSOParser<'_> for FSOTableFileParser {
	fn get(&self) -> &str {
		let start = self.state.borrow().pos;
		&self.original[start..]
	}
	
	fn line(&self) -> usize {
		self.state.borrow().line
	}

	fn consume(&self, count: usize) {
		if count == 0 {
			return;
		}
		
		let newlines = self.get()[..count].chars().filter(|c| *c == '\n').count();
		
		let mut state = self.state.borrow_mut();
		state.pos += count;
		state.line += newlines;
	}
}

#[derive(Default)]
pub struct FSOTableBuilder {
	buffer: String,
	state: FSOBuilderState
}

impl FSOBuilder for FSOTableBuilder {
	fn append(&mut self, content: &str) {
		self.buffer.push_str(content);
	}

	fn spew(self) -> String {
		self.buffer
	}

	fn get_state(&mut self) -> &mut FSOBuilderState {
		&mut self.state
	}
}

pub struct FSOParsingHangingGobble {
	pub comments: Option<String>,
	pub version_string: Option<String>
}

pub trait FSOTable {
	fn parse<'parser, Parser: FSOParser<'parser>>(state: &'parser Parser, hanging_gobble: Option<FSOParsingHangingGobble>) -> Result<(Self, Option<FSOParsingHangingGobble>), FSOParsingError> where Self: Sized;
	fn spew(&self, state: &mut impl FSOBuilder);
}