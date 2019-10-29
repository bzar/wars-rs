use ::model::*;

impl WeaponData {
    pub fn new(name: &str, require_deployed: bool, range_list: Vec<(u32, u32)>, power_list: Vec<(ArmorId, u32)>) -> WeaponData {
        WeaponData {
            name: name.into(),
            require_deployed,
            range_map: range_list.into_iter().collect(),
            power_map: power_list.into_iter().collect()
        }

    }
}

impl UnitData {
    pub fn new(name: &str, unit_class: UnitClassId, movement_type: MovementId, movement: u32, armor_type: ArmorId, defense_list: Vec<(TerrainId, u32)>, weapons: Vec<WeaponId>, price: u32, carry_classes: Vec<UnitClassId>, carry_num: u32, flags: Vec<UnitFlagId>) -> UnitData {
        UnitData {
            name: name.into(),
            unit_class,
            movement_type,
            movement,
            armor_type,
            defense_map: defense_list.into_iter().collect(),
            weapons,
            price,
            carry_classes,
            carry_num,
            flags 
        }
    }
}

impl Configuration {
    pub fn new_wars() -> Configuration {
        let armor_list = vec![
            (0, ArmorData { name: "Infantry".into() }),
            (1, ArmorData { name: "LightVehicle".into() }),
            (2, ArmorData { name: "HeavyVehicle".into() }),
            (3, ArmorData { name: "LightTank".into() }),
            (4, ArmorData { name: "MediumTank".into() }),
            (5, ArmorData { name: "HeavyTank".into() }),
            (6, ArmorData { name: "Interceptor".into() }),
            (7, ArmorData { name: "Copter".into() }),
            (8, ArmorData { name: "Bomber".into() }),
            (9, ArmorData { name: "LightShip".into() }),
            (10, ArmorData { name: "MediumShip".into() }),
            (11, ArmorData { name: "HeavyShip".into() })
        ];
        let weapon_list = vec![
            (0, WeaponData::new(
                "Rifle", false, vec![(1, 100)],
                vec![(0, 50), (1, 30), (2, 20), (3, 20), (4, 10), (5, 5), (7, 15), (9, 10), (10, 7), (11, 4)]
                )),
            (1, WeaponData::new(
                "Machinegun", false, vec![(1, 100)],
                vec![(0, 100), (1, 40), (2, 30), (3, 30), (4, 20), (5, 10), (7, 25), (9, 15), (10, 12), (11, 8)]
                )),
            (15, WeaponData::new(
                "HeavyMachinegun", false, vec![(1, 100)],
                vec![(0, 130), (1, 50), (2, 40), (3, 35), (4, 25), (5, 15), (7, 35), (9, 20), (10, 16), (11, 12)]
                )),
            (2, WeaponData::new(
                "Bazooka", false, vec![(1, 100)],
                vec![(0, 20), (1, 60), (2, 50), (3, 40), (4, 30), (5, 20), (9, 20), (10, 16), (11, 12)]
                )),
            (3, WeaponData::new(
                "LightCannon", false, vec![(1, 100), (2, 50)],
                vec![(0, 30), (1, 60), (2, 50), (3, 50), (4, 35), (5, 30), (9, 25), (10, 20), (11, 15)]
                )),
            (4, WeaponData::new(
                "MediumCannon", false, vec![(1, 100), (2, 50)],
                vec![(0, 40), (1, 80), (2, 70), (3, 60), (4, 50), (5, 40), (9, 30), (10, 25), (11, 20)]
                )),
            (5, WeaponData::new(
                "HeavyCannon", false, vec![(1, 100), (2, 50)],
                vec![(0, 50), (1, 110), (2, 100), (3, 100), (4, 75), (5, 50), (9, 40), (10, 30), (11, 25)]
                )),
            (6, WeaponData::new(
                "LightArtillery", true, vec![(2, 90), (3, 100), (4, 90)],
                vec![(0, 100), (1, 50), (2, 40), (3, 30), (4, 20), (5, 10), (9, 40), (10, 35), (11, 30)]
                )),
            (7, WeaponData::new(
                "MediumArtillery", true, vec![(2, 90), (3, 100), (4, 100), (5, 90)],
                vec![(0, 120), (1, 80), (2, 70), (3, 60), (4, 50), (5, 30), (9, 50), (10, 40), (11, 35)]
                )),
            (8, WeaponData::new(
                "HeavyArtillery", true, vec![(3, 90), (4, 100), (5, 100), (6, 90)],
                vec![(0, 160), (1, 110), (2, 100), (3, 100), (4, 90), (5, 80), (9, 70), (10, 55), (11, 40)]
                )),
            (9, WeaponData::new(
                "AACannon", false, vec![(1, 100)],
                vec![(6, 70), (7, 100), (8, 80)]
                )),
            (10, WeaponData::new(
                "AAMissile", true, vec![(2, 100), (3, 100), (4, 90), (5, 80), (6, 70), (7, 50), (8, 40)],
                vec![(6, 120), (7, 140), (8, 100)]
                )),
            (11, WeaponData::new(
                "Copter_missile", false, vec![(1, 100)],
                vec![(0, 50), (1, 70), (2, 60), (3, 55), (4, 45), (5, 35), (7, 60), (9, 45), (10, 30), (11, 20)]
                )),
            (12, WeaponData::new(
                "Interceptor_missile", false, vec![(1, 100)],
                vec![(6, 50), (7, 100), (8, 80)]
                )),
            (13, WeaponData::new(
                "AerialBomb", false, vec![(1, 100)],
                vec![(0, 160), (1, 140), (2, 130), (3, 120), (4, 100), (5, 90), (9, 80), (10, 70), (11, 60)]
                )),
            (14, WeaponData::new(
                "CruiserArtillery", true, vec![(3, 80), (4, 90), (5, 100), (6, 70)],
                vec![(0, 180), (1, 140), (2, 120), (3, 110), (4, 100), (5, 90), (9, 100), (10, 65), (11, 50)]
                ))
        ];

        let unit_type_list = vec![
            (0, UnitData::new(
                    "Infantry",  0, 0, 3, 0,
                    vec![(1, 10), (2, 50), (5, 50), (6, 50), (7, 65), (8, 50), (9, 50)],
                    vec![0], 100, vec![], 0, vec![0])
            ),
            (1, UnitData::new(
                    "AT-Infantry",  0, 0, 2, 0,
                    vec![(1, 10), (2, 50), (5, 50), (6, 50), (7, 65), (8, 50), (9, 50)],
                    vec![0, 2], 200, vec![],0,vec![0])
            ),
            (14, UnitData::new(
                    "APC", 1, 1, 5, 3,
                    vec![],
                    vec![], 300, vec! [0], 2,vec![])
            ),
            (2, UnitData::new(
                    "Scout vehicle", 1, 1, 6, 1,
                    vec![(1, 10)],
                    vec![1], 400, vec! [], 0,vec![])
            ),
            (9, UnitData::new(
                    "AA vehicle", 1, 1, 5, 1,
                    vec![],
                    vec![9, 1], 500, vec! [], 0,vec![])
            ),
            (3, UnitData::new(
                    "Light tank", 1, 1, 5, 3,
                    vec![(1, 10)],
                    vec![3, 1], 700, vec! [], 0,vec![])
            ),
            (4, UnitData::new(
                    "Medium tank", 1, 2, 4, 4,
                    vec![],
                    vec![4, 15], 1200, vec! [], 0,vec![])
            ),
            (5, UnitData::new(
                    "Heavy tank", 1, 3, 4, 5,
                    vec![],
                    vec![5, 15], 1700, vec! [], 0,vec![])
            ),
            (6, UnitData::new(
                    "Light artillery", 1, 1, 4, 1,
                    vec![],
                    vec![6], 500, vec! [], 0,vec![])
            ),
            (7, UnitData::new(
                    "Medium artillery", 1, 2, 4, 1,
                    vec![],
                    vec![7], 1500, vec! [], 0,vec![])
            ),
            (8, UnitData::new(
                    "Heavy artillery", 1, 3, 4, 2,
                    vec![],
                    vec![8], 2600, vec! [], 0,vec![])
            ),
            (10, UnitData::new(
                    "SAM vehicle", 1, 3, 4, 2,
                    vec![],
                    vec![10], 1000, vec! [], 0,vec![])
            ),
            (15, UnitData::new(
                    "Transport copter", 2, 4, 6, 7,
                    vec! [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 0), (12, 0)],
                    vec![], 500, vec! [0], 2,vec![])
            ),
            (11, UnitData::new(
                    "Attack copter", 2, 4, 7, 7,
                    vec! [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 0), (12, 0)],
                    vec![11, 1], 1000, vec! [], 0,vec![])
            ),
            (12, UnitData::new(
                    "Interceptor", 2, 4, 12, 6,
                    vec! [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 0), (12, 0)],
                    vec![12], 1500, vec! [], 0,vec![])
            ),
            (13, UnitData::new(
                    "Bomber", 2, 4, 9, 8,
                    vec! [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 0), (12, 0)],
                    vec![13], 2200, vec! [], 0,vec![])
            ),
            (16, UnitData::new(
                    "Cargo ship", 3, 5, 4, 10,
                    vec![],
                    vec![], 800, vec! [0, 1], 2,vec![])
            ),
            (17, UnitData::new(
                    "Gunboat", 3, 5, 5, 10,
                    vec![],
                    vec![4, 15], 1000, vec! [], 0,vec![])
            ),
            (18, UnitData::new(
                    "AA boat", 3, 5, 6, 9,
                    vec![],
                    vec![9], 700, vec! [], 0,vec![])
            ),
            (19, UnitData::new(
                    "Cruiser", 3, 5, 4, 11,
                    vec![],
                    vec![14], 3000, vec! [], 0,vec![])
            )
        ];

        let movement_list = vec![];
        let terrain_list = vec![];
        let unit_flag_list = vec![];
        let terrain_flag_list = vec![];

        Configuration {
            weapons: weapon_list.into_iter().collect(),
            armors: armor_list.into_iter().collect(),
            movements: movement_list.into_iter().collect(),
            units: unit_type_list.into_iter().collect(),
            unit_flags: unit_flag_list.into_iter().collect(),
            terrain_flags: terrain_flag_list.into_iter().collect(),
            terrains: terrain_list.into_iter().collect()
        }
    }
}

#[cfg(test)]
mod test {
    use ::model::*;

    #[test]
    fn generate_wars_configuration() {
        Configuration::new_wars();

    }
}
/*





[ MovementType { id: 0, name: "Walk", effectMap: { "3": 2, "4": 3 } },
  MovementType {
    id: 1,
    name: "LightVehicle",
    effectMap: { "2": 2, "3": null, "4": null } },
  MovementType {
    id: 2,
    name: "MediumVehicle",
    effectMap: { "1": 1, "2": 3, "3": null, "4": null } },
  MovementType {
    id: 3,
    name: "HeavyVehicle",
    effectMap: { "1": 2, "2": 4, "3": null, "4": null, "10": 2 } },
  MovementType { id: 4, name: "Flying", effectMap: {} },
  MovementType {
    id: 5,
    name: "Ship",
    effectMap: 
     { "0": null,
       "1": null,
       "2": null,
       "3": null,
       "4": 1,
       "5": null,
       "6": null,
       "7": null,
       "8": null,
       "9": 1,
       "10": 1,
       "11": 1,
       "12": null } } ]


[ TerrainType {
    id: 0,
    name: "Road",
    defense: 0,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 1,
    name: "Plains",
    defense: 0,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 2,
    name: "Forest",
    defense: 20,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 3,
    name: "Mountains",
    defense: 60,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 4,
    name: "Water",
    defense: 0,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 5,
    name: "City",
    defense: 40,
    buildTypes: [],
    repairTypes: [ 0, 1 ],
    flags: [ 0, 1 ] },
  TerrainType {
    id: 6,
    name: "Base",
    defense: 45,
    buildTypes: [ 0, 1 ],
    repairTypes: [ 0, 1 ],
    flags: [ 0, 1 ] },
  TerrainType {
    id: 7,
    name: "Fort",
    defense: 20,
    buildTypes: [],
    repairTypes: [ 0 ],
    flags: [ 0 ] },
  TerrainType {
    id: 8,
    name: "Airport",
    defense: 45,
    buildTypes: [ 2 ],
    repairTypes: [ 0, 2 ],
    flags: [ 0 ] },
  TerrainType {
    id: 9,
    name: "Port",
    defense: 45,
    buildTypes: [ 3 ],
    repairTypes: [ 0, 3 ],
    flags: [ 0 ] },
  TerrainType {
    id: 10,
    name: "Beach",
    defense: 0,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 11,
    name: "Bridge",
    defense: 0,
    buildTypes: [],
    repairTypes: [],
    flags: [] },
  TerrainType {
    id: 12,
    name: "HQ",
    defense: 60,
    buildTypes: [],
    repairTypes: [ 0, 1 ],
    flags: [ 0, 2 ] } ]

[ UnitClass { id: 0, name: "Infantry" },
  UnitClass { id: 1, name: "Vehicle" },
  UnitClass { id: 2, name: "Aerial" },
  UnitClass { id: 3, name: "Naval" } ]

  [ UnitFlag { id: 0, name: "Capture" } ]

  [ TerrainFlag { id: 0, name: "Capturable" },
  TerrainFlag { id: 1, name: "Funds" },
  TerrainFlag { id: 2, name: "HQ" } ]
*/

