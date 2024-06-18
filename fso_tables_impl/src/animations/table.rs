use fso_tables::fso_table;

#[fso_table(table_start="#Animations", table_end="#End")]
pub struct AnimationTable {
	#[unnamed]
	pub curves: Vec<Animation>
}

#[fso_table]
pub struct Animation {
	pub name: String,
	#[fso_name="$Type:"]
	pub triggered_by: AnimationTrigger,
	pub flags: Vec<AnimationFlag>,
	#[unnamed]
	pub segment: AnimationSegment
}

//This is a bit ugly, but it's an animation table only issue, so do it manually here...
#[fso_table]
pub enum AnimationTrigger {
	#[fso_name="initial"]
	Initial,
	#[fso_name="on-spawn"]
	OnSpawn,
	#[fso_name="docking-stage-1"]
	DockingStage1 {trigger: AnimationTriggerDocking},
	#[fso_name="docking-stage-2"]
	DockingStage2 {trigger: AnimationTriggerDocking},
	#[fso_name="docking-stage-3"]
	DockingStage3 {trigger: AnimationTriggerDocking},
	#[fso_name="docked"]
	Docked {trigger: AnimationTriggerDocking},
	#[fso_name="primary-bank"]
	PrimaryBank {trigger: AnimationTriggerWeaponBank},
	#[fso_name="primary-fired"]
	PrimaryFired {trigger: AnimationTriggerWeaponBank},
	#[fso_name="secondary-bank"]
	SecondaryBank {trigger: AnimationTriggerWeaponBank},
	#[fso_name="secondary-fired"]
	SecondaryFired {trigger: AnimationTriggerWeaponBank},
	#[fso_name="fighterbay"]
	Fighterbay {trigger: AnimationTriggerFighterbay},
	#[fso_name="afterburner"]
	Afterburner,
	#[fso_name="turret-firing"]
	TurretFiring {trigger: AnimationTriggerTurret},
	#[fso_name="turret-fired"]
	TurretFired {trigger: AnimationTriggerTurret},
	#[fso_name="scripted"]
	Scripted {trigger: AnimationTriggerScripted}
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

#[fso_table(prefix="$", suffix=":")]
pub enum AnimationSegment {
	SetOrientation
}