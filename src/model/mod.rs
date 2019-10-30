mod model;
pub use self::model::*;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum UnitType {
    Infantry, ATInfantry, Scout,
    LightTank, MediumTank, HeavyTank,
    LightArtillery, MediumArtillery, HeavyArtillery,
    AAVehicle, SAMVehicle, AttackCopter,
    Interceptor, Bomber,
    APC, TransportCopter, CargoShip,
    GunBoat, AABoat, Cruiser
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum UnitClass {
    Infantry, Vehicle, Aerial, Naval
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Movement {
    Walk, LightVehicle, MediumVehicle, HeavyVehicle, Flying, Ship
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Armor {
    Infantry, LightVehicle, HeavyVehicle,
    LightTank, MediumTank, HeavyTank,
    Interceptor, Copter, Bomber,
    LightShip, MediumShip, HeavyShip
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Weapon {
    Rifle, Machinegun, Bazooka,
    LightCannon, MediumCannon, HeavyCannon,
    LightArtillery, MediumArtillery, HeavyArtillery,
    AACannon, AAMissile, CopterMissile,
    InterceptorMissile, AerialBomb,
    CruiserArtillery, HeavyMachinegun
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Terrain {
    Road, Plains, Forest, Mountains, Water,
    City, Base, Fort, Airport, Port,
    Beach, Bridge, HQ
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum UnitFlag {
    Capture
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum TerrainFlag {
    Capturable, Funds, HQ
}

pub struct WeaponData<'a> {
    pub name: &'a str,
    pub power_map: Box<dyn Fn(Armor) -> Option<u32>>,
    pub range_map: Box<dyn Fn(u32) -> Option<u32>>,
    pub require_deployed: bool
}

pub struct ArmorData<'a> {
    pub name: &'a str
}
pub struct MovementData<'a> {
    pub name: &'a str,
    pub terrain_cost_map: Box<dyn Fn(Terrain) -> Option<u32>>,
}
pub struct TerrainFlagData<'a> {
    pub name: &'a str
}
pub struct UnitFlagData<'a> {
    pub name: &'a str
}

pub struct UnitTypeData<'a> {
    pub name: &'a str,
    pub unit_class: UnitClass,
    pub movement_type: Movement,
    pub movement: u32,
    pub armor_type: Armor,
    pub defense_map: Box<dyn Fn(Terrain) -> Option<u32>>,
    pub weapons: &'a [Weapon],
    pub price: u32,
    pub carry_classes: &'a [UnitClass],
    pub carry_num: u32,
    pub flags: &'a [UnitFlag]
}

pub struct TerrainData<'a> {
    pub name: &'a str,
    pub default_defense: u32,
    pub build_classes: &'a [UnitClass],
    pub repair_classes: &'a [UnitClass],
    pub flags: &'a [TerrainFlag]
}

