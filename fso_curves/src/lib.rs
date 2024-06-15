pub use self::curve::*;
mod curve;

#[cfg(test)]
mod tests {
	use super::curve::*;

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
	}

	#[test]
	fn subcurve() {
		let available_curves = BUILTIN_CURVES.iter().map(|c| c).collect::<Vec<&Curve>>();
		
		let curve = Curve {
			name: "".to_string(),
			keyframes: vec![
				CurveKeyframe{ x: 0f32, y: 0f32, segment: CurveSegment::Subcurve { curve: "EaseInQuad".to_string() } },
				CurveKeyframe{ x: 0.5f32, y: 0.5f32, segment: CurveSegment::Polynomial { ease_in: true, degree: 2f32 } },
				CurveKeyframe{ x: 1f32, y: 1f32, segment: CurveSegment::Constant }
			]
		};

		assert!((curve.calculate(0f32, &available_curves) - 0f32).abs() < 0.001);
		assert!((curve.calculate(0.25f32, &available_curves) - 0.125f32).abs() < 0.001);
		assert!((curve.calculate(0.5f32, &available_curves) - 0.5f32).abs() < 0.001);
		assert!((curve.calculate(0.75f32, &available_curves) - 0.625f32).abs() < 0.001);
		assert!((curve.calculate(1f32, &available_curves) - 1f32).abs() < 0.001);
	}
}
