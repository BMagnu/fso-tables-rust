use fso_tables::{fso_table, FSOBuilder, FSOParser, FSOParsingError, FSOTable};
use crate::{Angles, Vec3D};

#[fso_table(toplevel)]
pub struct AnimationTable {
	#[unnamed]
	pub animations: AnimationSubtable,
	#[unnamed]
	pub moveables: Option<MoveablesSubtable>
}

#[fso_table(table_start="#Animations", table_end="#End")]
pub struct AnimationSubtable {
	#[unnamed]
	pub animations: Vec<Animation>
}

#[fso_table(table_start="#Moveables", table_end="#End")]
pub struct MoveablesSubtable {
	#[unnamed]
	pub moveables: Vec<Moveable>
}

#[fso_table]
pub struct Animation {
	pub name: String,
	#[fso_name="$Type:"]
	pub triggered_by: AnimationTrigger,
	pub flags: Option<Vec<AnimationFlag>>,
	#[unnamed]
	pub segment: AnimationSegment
}

pub struct Moveable;
impl FSOTable for Moveable {
	fn parse<'a, Parser: FSOParser<'a>>(state: &'a Parser) -> Result<Self, FSOParsingError> { Err(FSOParsingError{ line: state.line(), reason: "Unimplemented!".to_string() })}
	fn spew(&self, _state: &mut impl FSOBuilder) { }
}

//This is a bit ugly, but it's an animation table only issue, so do it manually here...
#[fso_table]
pub enum AnimationTrigger {
	#[fso_name="initial"]
	Initial,
	#[fso_name="on-spawn"]
	OnSpawn,
	#[fso_name="docking-stage-1"]
	DockingStage1 {trigger: Option<AnimationTriggerDocking>},
	#[fso_name="docking-stage-2"]
	DockingStage2 {trigger: Option<AnimationTriggerDocking>},
	#[fso_name="docking-stage-3"]
	DockingStage3 {trigger: Option<AnimationTriggerDocking>},
	#[fso_name="docked"]
	Docked {trigger: Option<AnimationTriggerDocking>},
	#[fso_name="primary-bank"]
	PrimaryBank {trigger: Option<AnimationTriggerWeaponBank>},
	#[fso_name="primary-fired"]
	PrimaryFired {trigger: Option<AnimationTriggerWeaponBank>},
	#[fso_name="secondary-bank"]
	SecondaryBank {trigger: Option<AnimationTriggerWeaponBank>},
	#[fso_name="secondary-fired"]
	SecondaryFired {trigger: Option<AnimationTriggerWeaponBank>},
	#[fso_name="fighterbay"]
	Fighterbay {trigger: Option<AnimationTriggerFighterbay>},
	#[fso_name="afterburner"]
	Afterburner,
	#[fso_name="turret-firing"]
	TurretFiring {trigger: Option<AnimationTriggerTurret>},
	#[fso_name="turret-fired"]
	TurretFired {trigger: Option<AnimationTriggerTurret>},
	#[fso_name="scripted"]
	Scripted {trigger: Option<AnimationTriggerScripted>}
}

#[fso_table(prefix="+")]
pub struct AnimationTriggerDocking {
	//Docking Port Name or Number
	pub triggered_by: String
}

#[fso_table(prefix="+")]
pub struct AnimationTriggerWeaponBank {
	//Weapon Bank Number
	pub triggered_by: u32
}

#[fso_table(prefix="+")]
pub struct AnimationTriggerFighterbay {
	//Figherbay Name or Number
	pub triggered_by: String
}

#[fso_table(prefix="+")]
pub struct AnimationTriggerTurret {
	//Turret Subsys Name
	pub triggered_by: String
}

#[fso_table(prefix="+")]
pub struct AnimationTriggerScripted {
	pub triggered_by: String
}

#[fso_table(flagset)]
pub enum AnimationFlag {
	AutoReverse,
	ResetAtCompletion,
	#[fso_name="loop"]
	Looping,
	RandomStartingPhase,
	PauseOnReverse,
	SeamlessWithStartup{ startup_time: f32 }
}

#[fso_table(prefix="+", suffix=":")]
pub enum AnimationTarget {
	Submodel{ submodel_name: String },
	TurretBase { subsystem_name: String },
	TurretArm { subsystem_name: String },
}

#[fso_table(prefix="$", suffix=":")]
pub enum AnimationSegment {
	SetOrientation { segment: AnimationSegmentSetOrientation },
	SetAngle { segment: AnimationSegmentSetAngle },
	Rotation { segment: AnimationSegmentRotation },
	AxisRotation { segment: AnimationSegmentAxisRotation },
	Translation { segment: AnimationSegmentTranslation },
	InverseKinematics { segment: AnimationSegmentIK },
	SoundDuring { segment: AnimationSegmentSoundDuring },
	Wait { segment: AnimationSegmentWait },
	SegmentSequential { segment: AnimationSegmentList },
	SegmentParallel { segment: AnimationSegmentList }
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentSetOrientation {
	pub angle: Angles,
	#[existence]
	pub absolute: bool,
	#[unnamed]
	pub submodel: Option<AnimationTarget>
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentSetAngle {
	pub angle: f32,
	#[unnamed]
	pub submodel: Option<AnimationTarget>
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentRotation {
	pub angle: Option<Angles>,
	#[existence]
	pub absolute: bool,
	pub velocity: Option<Angles>,
	pub time: Option<f32>,
	pub acceleration: Option<Angles>,
	#[unnamed]
	pub submodel: Option<AnimationTarget>
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentAxisRotation {
	pub axis: Vec3D,
	pub angle: Option<f32>,
	pub velocity: Option<f32>,
	pub time: Option<f32>,
	pub acceleration: Option<f32>,
	#[unnamed]
	pub submodel: Option<AnimationTarget>
}
#[fso_table(prefix="+")]
pub struct AnimationSegmentTranslation {
	pub vector: Option<Vec3D>,
	#[existence]
	pub absolute: bool,
	pub velocity: Option<Vec3D>,
	pub time: Option<f32>,
	pub acceleration: Option<Vec3D>,
	pub coordinate_system: Option<AnimationTranslationCoordinateSystem>,
	#[unnamed]
	pub submodel: Option<AnimationTarget>
}

#[fso_table]
pub enum AnimationTranslationCoordinateSystem {
	Parent,
	#[fso_name="Local at start"]
	LocalAtStart,
	#[fso_name="Local current"]
	LocalCurrent,
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentIK {
	pub target_position: Option<Vec3D>,
	pub target_orientation: Option<Angles>,
	pub time: f32,
	#[unnamed]
	pub links: Vec<AnimationSegmentIKChainLink>
}

#[fso_table(table_start="$Chain Link:", prefix="+")]
pub struct AnimationSegmentIKChainLink {
	#[unnamed]
	pub submodel: AnimationTarget,
	pub acceleration: Option<Vec3D>,
	pub constraint: Option<AnimationSegmentIKConstraint>
}

#[fso_table]
pub enum AnimationSegmentIKConstraint {
	Window {size: AnimationSegmentIKConstraintWindow},
	Hinge {axis: AnimationSegmentIKConstraintHinge}
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentIKConstraintWindow {
	pub window_size: Angles
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentIKConstraintHinge {
	pub axis: Vec3D
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentSoundDuring {
	#[unnamed]
	pub submodel: Option<AnimationTarget>,
	pub start: Option<String>,
	#[fso_name="+Loop:"]
	pub loop_sound: Option<String>,
	pub end: Option<String>,
	pub radius: f32,
	pub position: Option<Vec3D>,
	#[existence]
	pub flip_when_reversed: bool,
	#[existence]
	#[fso_name="+Don't Interrupt Playing Sounds"]
	pub dont_interrupt: bool,
	#[unnamed]
	pub segment: Box<AnimationSegment>
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentWait {
	pub time: f32
}

#[fso_table(prefix="+")]
pub struct AnimationSegmentList {
	#[unnamed]
	pub submodel: Option<AnimationTarget>,
	#[gobble="+End Segment"]
	#[unnamed]
	pub segments: Vec<AnimationSegment>
}