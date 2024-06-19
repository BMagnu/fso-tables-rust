use fso_tables::fso_table;

#[fso_table(table_start="#Curves", table_end="#End", toplevel)]
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

#[fso_table]
pub struct CurveKeyframe {
	#[unnamed]
	#[gobble=":"]
	pub pos: (f32, f32),
	#[unnamed]
	pub segment: CurveSegment
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