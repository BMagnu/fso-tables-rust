use std::ops::Range;
use std::string::ToString;

use crate::curves::*;

impl Curve {
	pub fn calculate(&self, x: f32, curves: &Vec<&Curve>) -> f32 {
		assert!(self.keyframes.len() >= 2);

		if self.keyframes[0].pos.0 > x {
			return self.keyframes[0].pos.1;
		}
		else if self.keyframes[self.keyframes.len() - 1].pos.0 <= x {
			return self.keyframes[self.keyframes.len() - 1].pos.1;
		}

		let result = self.keyframes[1..].iter().enumerate().find(|(_, kf)| x < kf.pos.0).map(|(prev_index, kf)| {
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
		let x_bounds = first.pos.0..last.pos.0;

		let (min_y, max_y) = self.keyframes.iter().fold((f32::INFINITY, -f32::INFINITY), |(min_y, max_y), kf| (f32::min(min_y, kf.pos.1), f32::max(max_y, kf.pos.1)) );
		let y_bounds = min_y..max_y;

		(x_bounds, y_bounds)
	}
}
impl Default for Curve {
	fn default() -> Self { 
		Curve { name: "".to_string(), keyframes: vec![
			CurveKeyframe { pos: (0f32, 0f32), segment: CurveSegment::Linear },
			CurveKeyframe { pos: (1f32, 1f32), segment: CurveSegment::Constant }
		]} 
	}
}

impl CurveSegment {
	pub fn calculate(&self, x: f32, current: &CurveKeyframe, next: &CurveKeyframe, curves: &Vec<&Curve>) -> f32 {
		self.calculate_from_delta((x - current.pos.0) / (next.pos.0 - current.pos.0), curves) * (next.pos.1 - current.pos.1) + current.pos.1
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