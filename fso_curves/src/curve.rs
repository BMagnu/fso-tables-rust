use std::ops::Range;
use std::string::ToString;
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
use fso_tables::{fso_table, FSOParser, FSOParsingError, FSOTable};

#[fso_table(table_start="#Curves", table_end="#End")]
pub struct CurveTable {
	#[unnamed]
	pub curves: Vec<Curve>
}

#[fso_table]
pub struct Curve {
	pub name: String,
	#[fso_name="$KeyFrames:"]
	pub keyframes: Vec<CurveKeyframe>
}

pub struct CurveKeyframe {
	pub x: f32,
	pub y: f32,
	pub segment: CurveSegment
}
impl<'parser, Parser: FSOParser<'parser>> FSOTable<'parser, Parser> for CurveKeyframe {
	fn parse(state: &'parser Parser) -> Result<Self, FSOParsingError> {
		state.consume_whitespace(false);
		state.consume_string("(")?;
		let x = <f32 as FSOTable<Parser>>::parse(state)?;
		let y = <f32 as FSOTable<Parser>>::parse(state)?;
		state.consume_whitespace_inline(&[]);
		state.consume_string("):")?;
		state.consume_whitespace_inline(&[]);
		let segment = <CurveSegment as FSOTable<Parser>>::parse(state)?;
		Ok(CurveKeyframe { x, y, segment })
	}

	fn dump(&self) { }
}

#[fso_table]
pub enum CurveSegment{
	Constant,
	Linear,
	Polynomial { degree: f32, ease_in: Option<bool> },
	Circular { ease_in: Option<bool> },
	#[use_as_default_string]
	Subcurve { curve: String }
}

impl Curve {
	pub fn calculate(&self, x: f32, curves: &Vec<&Curve>) -> f32 {
		assert!(self.keyframes.len() >= 2);

		if self.keyframes[0].x > x {
			return self.keyframes[0].y;
		}
		else if self.keyframes[self.keyframes.len() - 1].x <= x {
			return self.keyframes[self.keyframes.len() - 1].y;
		}

		let result = self.keyframes[1..].iter().enumerate().find(|(_, kf)| x < kf.x).map(|(prev_index, kf)| {
			let prev_kf = &self.keyframes[prev_index];
			prev_kf.segment.calculate(x, prev_kf, kf, curves)
		});

		if let Some(result) = result {
			result
		}
		else {
			//At this point, no keyframe was matched. Should be impossible
			unreachable!("Keyframe not found");
		}
	}
	
	pub fn get_bounds(&self) -> (Range<f32>, Range<f32>) {
		assert!(self.keyframes.len() >= 2);
		let first = self.keyframes.first().unwrap();
		let last = self.keyframes.last().unwrap();
		let x_bounds = first.x..last.x;

		let (min_y, max_y) = self.keyframes.iter().fold((f32::INFINITY, -f32::INFINITY), |(min_y, max_y), kf| (f32::min(min_y, kf.y), f32::max(max_y, kf.y)) );
		let y_bounds = min_y..max_y;

		(x_bounds, y_bounds)
	}
}

impl Default for Curve {
	fn default() -> Self { 
		Curve { name: "".to_string(), keyframes: vec![
			CurveKeyframe { x: 0f32, y: 0f32, segment: CurveSegment::Linear },
			CurveKeyframe { x: 1f32, y: 1f32, segment: CurveSegment::Constant }
		]} 
	}
}

impl CurveSegment {
	pub fn calculate(&self, x: f32, current: &CurveKeyframe, next: &CurveKeyframe, curves: &Vec<&Curve>) -> f32 {
		self.calculate_from_delta((x - current.x) / (next.x - current.x), curves) * (next.y - current.y) + current.y
	}
	
	fn calculate_from_delta(&self, t: f32, curves: &Vec<&Curve>) -> f32 {
		match self{
			CurveSegment::Constant => { 0f32 }
			CurveSegment::Linear => { t }
			&CurveSegment::Polynomial { ease_in, degree } => {
				if ease_in.unwrap_or(true) {
					t.powf(degree)
				}
				else {
					1f32 - (1f32 - t).powf(degree)
				}
			}
			&CurveSegment::Circular { ease_in } => {
				if ease_in.unwrap_or(true) {
					1f32 - (1f32 - t.powi(2)).sqrt()
				}
				else {
					(1f32 - (1f32 - t).powi(2)).sqrt()
				}
			}
			CurveSegment::Subcurve { curve } => { 
				curves.iter().find(|c| c.name.eq_ignore_ascii_case(curve)).map_or(0f32, |c| c.calculate(t, curves))
			}
		}
	}
}

pub static BUILTIN_CURVES: Lazy<Vec<Curve>> = Lazy::new(|| {
	let mut builtins = Vec::new();

	#[derive(PartialEq, EnumIter, Display)]
	enum EASE {EaseIn, EaseOut, EaseInOut}

	#[derive(Copy, Clone, EnumIter, Display)]
	enum TYPE {Circ, Quad = 2, Cubic = 3, Quart = 4, Quint = 5}

	for ease in EASE::iter() {
		for reverse in [true, false] {
			for interptype in TYPE::iter() {
				let mut name = ease.to_string() + &interptype.to_string();
				let mut keyframes: Vec<CurveKeyframe> = Vec::new();

				if reverse {
					name += "Rev";
				}

				let ease_in = Some(ease != EASE::EaseOut);

				keyframes.push(CurveKeyframe {
					x: 0f32,
					y: if reverse { 1f32 } else { 0f32 },
					segment: match interptype {
						TYPE::Circ => {
							CurveSegment::Circular { ease_in }
						}
						interptype => {
							CurveSegment::Polynomial { ease_in, degree: (interptype as i32) as f32 }
						}
					}
				});

				if ease == EASE::EaseInOut {
					keyframes.push(CurveKeyframe {
						x: 0.5f32,
						y: 0.5f32,
						segment: match interptype {
							TYPE::Circ => {
								CurveSegment::Circular { ease_in: Some(!ease_in.unwrap()) }
							}
							interptype => {
								CurveSegment::Polynomial { ease_in: Some(!ease_in.unwrap()), degree: (interptype as i32) as f32 }
							}
						}
					});
				}

				keyframes.push(CurveKeyframe {
					x: 1f32,
					y: if reverse { 0f32 } else { 1f32 },
					segment: CurveSegment::Constant {}
				});

				builtins.push(Curve { name, keyframes })
			}
		}
	}

	builtins
});