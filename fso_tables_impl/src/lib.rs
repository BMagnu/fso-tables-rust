pub mod curves;
pub mod animations;

use fso_tables::fso_table;

//Reexport the properties that you need to use this crate. Only force people to include the original fso_tables crate if they want to manually add tables or types or anything.
pub use fso_tables::FSOTableFileParser;
pub use fso_tables::FSOTable;
pub use fso_tables::FSOParsingError;

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