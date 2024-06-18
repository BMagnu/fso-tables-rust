mod table;
mod curve;
mod builtins;

pub use table::*;
#[allow(unused_imports)]
pub use curve::*;
pub use builtins::*;

#[cfg(test)]
mod tests {
	use crate::curves::*;

	#[test]
	fn builtin() {
		let curve = BUILTIN_CURVES.iter().find(|curve| curve.name == "EaseInOutQuad");
		let available_curves = BUILTIN_CURVES.iter().map(|c| c).collect::<Vec<&Curve>>();

		assert!(curve.is_some());

		let curve = curve.unwrap();

		assert!((curve.calculate(0f32, &available_curves) - 0f32).abs() < 0.001);
		assert!((curve.calculate(0.25f32, &available_curves) - 0.125f32).abs() < 0.001);
		assert!((curve.calculate(0.5f32, &available_curves) - 0.5f32).abs() < 0.001);
		assert!((curve.calculate(0.75f32, &available_curves) - 0.875f32).abs() < 0.001);
		assert!((curve.calculate(1f32, &available_curves) - 1f32).abs() < 0.001);

		let (x_bounds, y_bounds) = curve.get_bounds();

		assert!((x_bounds.start - 0f32).abs() < 0.001);
		assert!((x_bounds.end - 1f32).abs() < 0.001);
		assert!((y_bounds.start - 0f32).abs() < 0.001);
		assert!((y_bounds.end - 1f32).abs() < 0.001);
	}

	#[test]
	fn subcurve() {
		let available_curves = BUILTIN_CURVES.iter().map(|c| c).collect::<Vec<&Curve>>();

		let curve = Curve {
			name: "".to_string(),
			keyframes: vec![
				CurveKeyframe{ pos: (0f32, 0f32), segment: CurveSegment::Subcurve { curve: "EaseInQuad".to_string() } },
				CurveKeyframe{ pos: (0.5f32, 0.5f32), segment: CurveSegment::Polynomial { ease_in: Some(true), degree: 2f32 } },
				CurveKeyframe{ pos: (1f32, 1f32), segment: CurveSegment::Constant }
			]
		};

		assert!((curve.calculate(0f32, &available_curves) - 0f32).abs() < 0.001);
		assert!((curve.calculate(0.25f32, &available_curves) - 0.125f32).abs() < 0.001);
		assert!((curve.calculate(0.5f32, &available_curves) - 0.5f32).abs() < 0.001);
		assert!((curve.calculate(0.75f32, &available_curves) - 0.625f32).abs() < 0.001);
		assert!((curve.calculate(1f32, &available_curves) - 1f32).abs() < 0.001);
	}
}
