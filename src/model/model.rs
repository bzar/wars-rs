use crate::model::*;

pub const MAX_CAPTURE_POINTS: u32 = 200;
pub const CAPTURE_POINT_REGEN_RATE: u32 = 50;
pub const FUNDS_PER_PROPERTY: u32 = 100;

pub fn weapon(x: Weapon) -> WeaponData<'static> {
    use model::Weapon::*;

    let name = match x {
        Rifle => "Rifle",
        Machinegun => "Machinegun",
        Bazooka => "Bazooka",
        LightCannon => "LightCannon",
        MediumCannon => "MediumCannon",
        HeavyCannon => "HeavyCannon",
        LightArtillery => "LightArtillery",
        MediumArtillery => "MediumArtillery",
        HeavyArtillery => "HeavyArtillery",
        AACannon => "AACannon",
        AAMissile => "AAMissile",
        CopterMissile => "CopterMissile",
        InterceptorMissile => "InterceptorMissile",
        AerialBomb => "AerialBomb",
        CruiserArtillery => "CruiserArtillery",
        HeavyMachinegun => "HeavyMachinegun"
    };

    let power_map: Box<dyn Fn(Armor) -> Option<u32>> = {
        use model::Armor::*;

        match x {
            Rifle => Box::new(|a| match a {
                Infantry => Some(50), LightVehicle => Some(30), HeavyVehicle => Some(20),
                LightTank => Some(20), MediumTank => Some(10), HeavyTank => Some(5),
                Interceptor | Bomber => None, Copter => Some(15),
                LightShip => Some(10), MediumShip => Some(7), HeavyShip => Some(4)
            }),
            Machinegun => Box::new(|a| match a {
                Infantry => Some(100), LightVehicle => Some(40), HeavyVehicle => Some(30),
                LightTank => Some(30), MediumTank => Some(20), HeavyTank => Some(10),
                Interceptor => Some(0), Copter => Some(25), Bomber => Some(0),
                LightShip => Some(15), MediumShip => Some(12), HeavyShip => Some(8)
            }),
            HeavyMachinegun => Box::new(|a| match a {
                Infantry => Some(130), LightVehicle => Some(50), HeavyVehicle => Some(40),
                LightTank => Some(35), MediumTank => Some(25), HeavyTank => Some(15),
                Interceptor | Bomber => None, Copter => Some(35),
                LightShip => Some(20), MediumShip => Some(16), HeavyShip => Some(12)
            }),
            Bazooka => Box::new(|a| match a {
                Infantry => Some(20), LightVehicle => Some(60), HeavyVehicle => Some(50),
                LightTank => Some(40), MediumTank => Some(30), HeavyTank => Some(20),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(20), MediumShip => Some(16), HeavyShip => Some(12)
            }),
            LightCannon => Box::new(|a| match a {
                Infantry => Some(30), LightVehicle => Some(60), HeavyVehicle => Some(50),
                LightTank => Some(50), MediumTank => Some(35), HeavyTank => Some(30),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(25), MediumShip => Some(20), HeavyShip => Some(15)
            }),
            MediumCannon => Box::new(|a| match a {
                Infantry => Some(40), LightVehicle => Some(80), HeavyVehicle => Some(70),
                LightTank => Some(60), MediumTank => Some(50), HeavyTank => Some(40),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(30), MediumShip => Some(25), HeavyShip => Some(20)
            }),
            HeavyCannon => Box::new(|a| match a {
                Infantry => Some(50), LightVehicle => Some(110), HeavyVehicle => Some(100),
                LightTank => Some(100), MediumTank => Some(75), HeavyTank => Some(50),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(40), MediumShip => Some(30), HeavyShip => Some(25)
            }),
            LightArtillery => Box::new(|a| match a {
                Infantry => Some(100), LightVehicle => Some(50), HeavyVehicle => Some(40),
                LightTank => Some(30), MediumTank => Some(20), HeavyTank => Some(10),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(40), MediumShip => Some(35), HeavyShip => Some(30)
            }),
            MediumArtillery => Box::new(|a| match a {
                Infantry => Some(120), LightVehicle => Some(80), HeavyVehicle => Some(70),
                LightTank => Some(60), MediumTank => Some(50), HeavyTank => Some(30),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(50), MediumShip => Some(40), HeavyShip => Some(35)
            }),
            HeavyArtillery => Box::new(|a| match a {
                Infantry => Some(160), LightVehicle => Some(110), HeavyVehicle => Some(100),
                LightTank => Some(100), MediumTank => Some(90), HeavyTank => Some(80),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(70), MediumShip => Some(55), HeavyShip => Some(40)
            }),
            AACannon => Box::new(|a| match a {
                Interceptor => Some(70), Copter => Some(100), Bomber => Some(80),
                Infantry | LightVehicle | HeavyVehicle
                    | LightTank | MediumTank | HeavyTank
                    | LightShip | MediumShip | HeavyShip => None
            }),
            AAMissile => Box::new(|a| match a {
                Interceptor => Some(120), Copter => Some(140), Bomber => Some(100),
                Infantry | LightVehicle | HeavyVehicle
                    | LightTank | MediumTank | HeavyTank
                    | LightShip | MediumShip | HeavyShip => None
            }),
            CopterMissile => Box::new(|a| match a {
                Infantry => Some(50), LightVehicle => Some(70), HeavyVehicle => Some(60),
                LightTank => Some(55), MediumTank => Some(45), HeavyTank => Some(35),
                Interceptor | Bomber => None, Copter => Some(60),
                LightShip => Some(45), MediumShip => Some(30), HeavyShip => Some(20)
            }),
            InterceptorMissile => Box::new(|a| match a {
                Interceptor => Some(50), Copter => Some(100), Bomber => Some(80),
                Infantry | LightVehicle | HeavyVehicle
                    | LightTank | MediumTank | HeavyTank
                    | LightShip | MediumShip | HeavyShip => None
            }),
            AerialBomb => Box::new(|a| match a {
                Infantry => Some(160), LightVehicle => Some(140), HeavyVehicle => Some(130),
                LightTank => Some(120), MediumTank => Some(100), HeavyTank => Some(90),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(80), MediumShip => Some(70), HeavyShip => Some(60)
            }),
            CruiserArtillery => Box::new(|a| match a {
                Infantry => Some(180), LightVehicle => Some(140), HeavyVehicle => Some(120),
                LightTank => Some(110), MediumTank => Some(100), HeavyTank => Some(90),
                Interceptor | Copter | Bomber => None,
                LightShip => Some(100), MediumShip => Some(65), HeavyShip => Some(50)
            }),
        }
    };

    let range_map: Box<dyn Fn(u32) -> Option<u32>> = match x {
        LightCannon | MediumCannon | HeavyCannon => Box::new(|r| match r {
            1 => Some(100), 2 => Some(50), _ => None
        }),
        LightArtillery => Box::new(|r| match r {
            2 | 4 => Some(90), 3 => Some(100), _ => None
        }),
        MediumArtillery => Box::new(|r| match r {
            2 | 5 => Some(90), 3 | 4 => Some(100), _ => None
        }),
        HeavyArtillery => Box::new(|r| match r {
            3 | 6 => Some(90), 4 | 5 => Some(100), _ => None
        }),
        CruiserArtillery => Box::new(|r| match r {
            3 => Some(80), 4 => Some(90), 5 => Some(100), 6 => Some(70), _ => None
        }),
        AAMissile => Box::new(|r| match r {
            2 | 3 => Some(100), 4 => Some(90), 5 => Some(80),
            6 => Some(70), 7 => Some(50), 8 => Some(40), _ => None

        }),
        _ => Box::new(|r| if r == 1 { Some(100) } else { None })
    };

    let require_deployed = match x {
        LightArtillery | MediumArtillery | HeavyArtillery | AAMissile | CruiserArtillery => true,
        _ => false
    };

    WeaponData { name, power_map, range_map, require_deployed }
}
pub fn armor(x: Armor) -> ArmorData<'static> {
    use model::Armor::*;
    let name = match x {
        Infantry => "Infantry",
        LightVehicle => "LightVehicle",
        HeavyVehicle => "HeavyVehicle",
        LightTank => "LightTank",
        MediumTank => "MediumTank",
        HeavyTank => "HeavyTank",
        Interceptor => "Interceptor",
        Copter => "Copter",
        Bomber => "Bomber",
        LightShip => "LightShip",
        MediumShip => "MediumShip",
        HeavyShip => "HeavyShip"
    };

    ArmorData { name }
}

pub fn movement(x: Movement) -> MovementData<'static> {
    use model::Movement::*;
    use model::Terrain::*;
    let name = match x {
        Walk => "Walk",
        LightVehicle => "Vehicle",
        MediumVehicle => "MediumVehicle",
        HeavyVehicle => "HeavyVehicle",
        Flying => "Flying",
        Ship => "Ship"
    };


    let terrain_cost_map: Box<dyn Fn(Terrain) -> Option<u32>> = match x {
        Walk => Box::new(|t| match t {
            Mountains => Some(2),
            Water => Some(3),
            _ => Some(1)
        }),
        LightVehicle => Box::new(|t| match t {
            Forest => Some(2),
            Mountains | Water => None,
            _ => Some(1)
        }),
        MediumVehicle => Box::new(|t| match t {
            Forest => Some(3),
            Mountains | Water => None,
            _ => Some(1)
        }),
        HeavyVehicle => Box::new(|t| match t {
            Plains | Beach => Some(2),
            Forest => Some(4),
            Mountains | Water => None,
            _ => Some(1)
        }),
        Flying => Box::new(|_| Some(1)),
        Ship => Box::new(|t| match t {
            Water | Beach | Port => Some(1),
            _ => None
        })
    };

    MovementData { name, terrain_cost_map }
}

pub fn unit_type(x: UnitType) -> UnitTypeData<'static> {
    use model::UnitType::*;

    let name = match x {
        Infantry => "Infantry",
        ATInfantry => "ATInfantry",
        Scout => "Scout",
        LightTank => "LightTank",
        MediumTank => "MediumTank",
        HeavyTank => "HeavyTank",
        LightArtillery => "LightArtillery",
        MediumArtillery => "MediumArtillery",
        HeavyArtillery => "HeavyArtillery",
        AAVehicle => "AAVehicle",
        SAMVehicle => "SAMVehicle",
        AttackCopter => "AttackCopter",
        Interceptor => "Interceptor",
        Bomber => "Bomber",
        APC => "APC",
        TransportCopter => "TransportCopter",
        CargoShip => "CargoShip",
        GunBoat => "GunBoat",
        AABoat => "AABoat",
        Cruiser => "Cruiser"
    };
    let unit_class = match x {
        Infantry | ATInfantry => UnitClass::Infantry,
        Scout
            | LightTank
            | MediumTank
            | HeavyTank
            | LightArtillery
            | MediumArtillery
            | HeavyArtillery
            | AAVehicle
            | SAMVehicle => UnitClass::Vehicle,
        AttackCopter
            | Interceptor
            | Bomber
            | APC
            | TransportCopter => UnitClass::Aerial,
        CargoShip
            | GunBoat
            | AABoat
            | Cruiser => UnitClass::Naval
    };
    let movement_type = match x {
        Infantry | ATInfantry => Movement::Walk,
        Scout
            | LightTank
            | LightArtillery
            | APC
            | AAVehicle => Movement::LightVehicle,
        MediumTank
            | HeavyTank
            | MediumArtillery
            | HeavyArtillery
            | SAMVehicle => Movement::HeavyVehicle,
        AttackCopter
            | Interceptor
            | Bomber
            | TransportCopter => Movement::Flying,
        CargoShip
            | GunBoat
            | AABoat
            | Cruiser => Movement::Ship
    };
    let movement = match x {
        ATInfantry => 2,
        Infantry => 3,
        MediumTank
            | HeavyTank
            | LightArtillery
            | MediumArtillery
            | HeavyArtillery
            | SAMVehicle
            | CargoShip
            | Cruiser => 4,
        LightTank | APC | AAVehicle | GunBoat => 5,
        Scout | TransportCopter | AABoat => 6,
        AttackCopter => 7,
        Bomber => 9,
        Interceptor => 12
    };
    let armor_type = match x {
        Infantry | ATInfantry => Armor::Infantry,
        Scout | LightArtillery | MediumArtillery | AAVehicle => Armor::LightVehicle,
        LightTank => Armor::LightTank,
        MediumTank => Armor::MediumTank,
        HeavyTank => Armor::HeavyTank,
        HeavyArtillery | SAMVehicle => Armor::HeavyVehicle,
        AttackCopter | TransportCopter => Armor::Copter,
        Interceptor => Armor::Interceptor,
        Bomber => Armor::Bomber,
        APC => Armor::LightTank,
        CargoShip | GunBoat => Armor::MediumShip,
        AABoat => Armor::LightShip,
        Cruiser => Armor::HeavyShip
    };
    let defense_map: Box<dyn Fn(Terrain) -> Option<u32>> = match x {
        Infantry | ATInfantry => Box::new(|t| match t {
            Terrain::Plains => Some(10),
            Terrain::Forest
                | Terrain::City
                | Terrain::Base
                | Terrain::Airport
                | Terrain::Port => Some(50),
            Terrain::Mountains => Some(65),
            _ => None
        }),
        Scout | LightTank => Box::new(|t| match t {
            Terrain::Plains => Some(10),
            _ => None
        }),
        AttackCopter | Interceptor | Bomber => Box::new(|_| Some(0)),
        _ => Box::new(|_| None)
    };

    let weapons: &[Weapon] = match x {
        Infantry => &[Weapon::Rifle],
        ATInfantry => &[Weapon::Rifle, Weapon::Bazooka],
        Scout => &[Weapon::Machinegun],
        LightTank => &[Weapon::Machinegun, Weapon::LightCannon],
        MediumTank => &[Weapon::HeavyMachinegun, Weapon::MediumCannon],
        HeavyTank => &[Weapon::HeavyMachinegun, Weapon::HeavyCannon],
        LightArtillery => &[Weapon::LightArtillery],
        MediumArtillery => &[Weapon::MediumArtillery],
        HeavyArtillery => &[Weapon::HeavyArtillery],
        AAVehicle => &[Weapon::Machinegun, Weapon::AACannon],
        SAMVehicle => &[Weapon::AAMissile],
        AttackCopter => &[Weapon::CopterMissile],
        Interceptor => &[Weapon::Machinegun, Weapon::CopterMissile],
        Bomber => &[Weapon::AerialBomb],
        APC | TransportCopter | CargoShip => &[],
        GunBoat => &[Weapon::HeavyMachinegun, Weapon::MediumCannon],
        AABoat => &[Weapon::AACannon],
        Cruiser => &[Weapon::CruiserArtillery]
    };
    let price = match x {
        Infantry => 100,
        ATInfantry => 200,
        Scout => 400,
        LightTank => 700,
        MediumTank => 1200,
        HeavyTank => 1700,
        LightArtillery => 500,
        MediumArtillery => 1500,
        HeavyArtillery => 2600,
        AAVehicle => 500,
        SAMVehicle => 1000,
        AttackCopter => 1000,
        Interceptor => 1500,
        Bomber => 2200,
        APC => 300,
        TransportCopter => 500,
        CargoShip => 800,
        GunBoat => 1000,
        AABoat => 700,
        Cruiser => 3000
    };
    let carry_classes: &[UnitClass] = match x {
        APC | CargoShip  => &[UnitClass::Infantry, UnitClass::Vehicle],
        TransportCopter => &[UnitClass::Infantry],
        _ => &[]
    };
    let carry_num = match x {
        APC | TransportCopter | CargoShip => 2,
        _ => 0
    };
    let flags: &[UnitFlag] = match x {
        Infantry | ATInfantry => &[UnitFlag::Capture],
        _ => &[]
    };

    UnitTypeData {
        name, unit_class, movement_type, movement, armor_type, defense_map,
        weapons, price, carry_classes, carry_num, flags
    }

}

pub fn unit_flag(x: UnitFlag) -> UnitFlagData<'static> {
    use model::UnitFlag::*;
    let name = match x {
        Capture => "Capture",
    };

    UnitFlagData { name }
}

pub fn terrain_flag(x: TerrainFlag) -> TerrainFlagData<'static> {
    use model::TerrainFlag::*;
    let name = match x {
        Capturable => "Capturable",
        Funds => "Funds",
        HQ => "HQ"
    };

    TerrainFlagData { name }
}

pub fn terrain(x: Terrain) -> TerrainData<'static> {
    use model::Terrain::*;

    let name = match x {
        Road => "Road",
        Plains => "Plains",
        Forest => "Forest",
        Mountains => "Mountains",
        Water => "Water",
        City => "City",
        Base => "Base",
        Fort => "Fort",
        Airport => "Airport",
        Port => "Port",
        Beach => "Beach",
        Bridge => "Bridge",
        HQ => "HQ" 
    };

    let default_defense = match x {
        Road | Plains | Water | Beach | Bridge => 0,
        Forest => 20,
        Mountains | HQ => 60,
        City => 40,
        Base | Airport | Port => 45,
        Fort => 20
    };

    let build_classes: &[UnitClass] = match x {
        Base => &[UnitClass::Infantry, UnitClass::Vehicle],
        Airport => &[UnitClass::Aerial],
        Port => &[UnitClass::Naval],
        _ => &[]
    };

    let repair_classes: &[UnitClass] = match x {
        City | Base | HQ => &[UnitClass::Infantry, UnitClass::Vehicle],
        Airport => &[UnitClass::Aerial, UnitClass::Infantry],
        Port => &[UnitClass::Naval, UnitClass::Infantry],
        _ => &[]
    };

    let flags: &[TerrainFlag] = match x {
        City | Base => &[TerrainFlag::Capturable, TerrainFlag::Funds],
        Port | Airport | Fort => &[TerrainFlag::Capturable],
        HQ => &[TerrainFlag::Capturable, TerrainFlag::HQ],
        _ => &[]
    };

    TerrainData { name, default_defense, build_classes, repair_classes, flags }
}

#[cfg(test)]
mod test {
    use crate::model::*;

    #[test]
    fn generate_wars_configuration() {
        let mut ws = unit_type(UnitType::HeavyTank).weapons.iter();
        assert!(ws.next() == Some(&Weapon::HeavyMachinegun));
        assert!(ws.next() == Some(&Weapon::HeavyCannon));
        assert!(ws.next() == None);

        let mut tfs = terrain(Terrain::City).flags.iter();
        assert!(tfs.next() == Some(&TerrainFlag::Capturable));
        assert!(tfs.next() == Some(&TerrainFlag::Funds));
        assert!(tfs.next() == None);
    }
}
