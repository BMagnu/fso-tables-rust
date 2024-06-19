pub mod curves;
pub mod animations;

use fso_tables::fso_table;

#[fso_table(inline)]
pub struct Vec3D {
	#[unnamed]
	pub x: f32,
	#[unnamed]
	pub y: f32,
	#[unnamed]
	pub z: f32
}

#[fso_table(inline)]
pub struct Angles {
	#[unnamed]
	pub pitch: f32,
	#[unnamed]
	pub heading: f32,
	#[unnamed]
	pub bank: f32
}