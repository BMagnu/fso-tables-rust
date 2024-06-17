use once_cell::sync::Lazy;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
use crate::curves::*;

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