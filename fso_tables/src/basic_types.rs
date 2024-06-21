use std::cmp::min;
use std::str::FromStr;
use crate::{FSOBuilder, FSOBuilderListState, FSOParser, FSOParsingError, FSOTable};

impl FSOTable for String {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		state.consume_whitespace_inline(&['"']);
		let result = state.read_until_last_whitespace_of_line_or_stop(&['"']);
		Ok(result.to_string())
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		match state.get_state().list_state.last() {
			Some( FSOBuilderListState::InlineList ) => {
				state.append(format!("\"{}\"", self).as_str());
			}
			_ => {
				state.append( self.as_str());
			}
		}
	}

}

impl FSOTable for bool {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		state.consume_whitespace_inline(&[]);
		let result = state.read_until_whitespace();
		match result.clone().to_lowercase().as_str() {
			"yes" | "true" | "on" => {
				Ok(true)
			}
			"no" | "false" | "off" => {
				Ok(false)
			}
			_ => {
				Err(FSOParsingError { reason: format!("Expected boolean value, got {}.", result), line: state.line() })
			}
		}
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(if *self { "YES" } else { "NO" });
	}
}

fn parse_number<'a, Parser: FSOParser<'a>, T: FromStr>(state: &Parser, allow_dot: bool, allow_minus: bool) -> Result<T, FSOParsingError> {
	state.consume_whitespace_inline(&[]);
	let current = state.get();
	let mut have_dot = !allow_dot;
	let mut to_consume = 0usize;

	for c in current.chars() {
		if c.is_ascii_digit() || ((c == '+' || (c == '-' && allow_minus)) && to_consume == 0) {
			to_consume += 1;
		}
		else if c == '.' && !have_dot {
			to_consume += 1;
			have_dot = true;
		}
		else {
			break;
		}
	}

	if to_consume == 0 {
		return Err(FSOParsingError { reason: format!("Expected {}, got {}!", if allow_dot { "float" } else { "int" }, &current[..min(4, current.len())]), line: state.line() } );
	}

	state.consume(to_consume);
	match <T as FromStr>::from_str(&current[..to_consume]) {
		Ok(f) => { Ok(f) },
		Err( _ ) => { Err(FSOParsingError { reason: format!("Expected {}, got {}!", if allow_dot { "float" } else { "int" }, &current[..to_consume]), line: state.line() } ) }
	}
}

impl FSOTable for f32 {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		parse_number(state, true, true)
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(self.to_string().as_str());
	}
}

impl FSOTable for f64 {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		parse_number(state, true, true)
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(self.to_string().as_str());
	}
}

impl FSOTable for i32 {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		parse_number(state, false, true)
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(self.to_string().as_str());
	}
}

impl FSOTable for i64 {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		parse_number(state, false, true)
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(self.to_string().as_str());
	}
}

impl FSOTable for u32 {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		parse_number(state, false, false)
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(self.to_string().as_str());
	}
}

impl FSOTable for u64 {
	fn parse<'a, Parser: FSOParser<'a>>(state: &Parser) -> Result<Self, FSOParsingError> {
		parse_number(state, false, false)
	}

	fn spew(&self, state: &mut impl FSOBuilder) {
		state.append(self.to_string().as_str());
	}
}