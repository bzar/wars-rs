use crate::game::*;
use crate::util::*;
use serde_json;

impl Map {
    pub fn from_json(data: &str) -> Result<Map, serde_json::Error> {
        let map_data: JsonMap = serde_json::from_str(data)?;
        Ok(map_data.into_map())
    }
    pub fn player_numbers(&self) -> Vec<u32> {
        let tile_owners = self.tiles.values().filter_map(|t| t.owner);
        let unit_owners = self.units.values().filter_map(|u| u.owner);
        tile_owners.chain(unit_owners).unique().collect()
    }
}

#[derive(Serialize, Deserialize)]
struct JsonMap {
    name: String,
    funds: u32,
    #[serde(rename="mapData")]
    map_data: Vec<JsonMapTile>
}

#[derive(Serialize, Deserialize)]
struct JsonMapTile {
    x: i32,
    y: i32,
    #[serde(rename="type")]
    tile_type: u32,
    #[serde(rename="subtype")]
    tile_subtype: u32,
    owner: u32,
    unit: Option<JsonMapUnit>
}

impl JsonMapTile {
    fn as_tile(&self, unit_id: UnitId) -> Tile {
        let terrain = {
            use model::Terrain::*;
            [Road, Plains, Forest, Mountains, Water,
            City, Base, Fort, Airport, Port,Beach, Bridge, HQ][self.tile_type as usize]
        };

        Tile {
            terrain,
            terrain_subtype_id: self.tile_subtype,
            owner: if self.owner != 0 { Some(self.owner) } else { None },
            x: self.x,
            y: self.y,
            unit: self.unit.as_ref().map(|_| unit_id),
            ..Tile::default()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JsonMapUnit {
    #[serde(rename="type")]
    unit_type: u32,
    owner: u32
}

impl JsonMapUnit {
    fn as_unit(&self) -> Unit {
        let unit_type = {
            use model::UnitType::*;
            [Infantry, ATInfantry, Scout, LightTank, MediumTank, HeavyTank,
            LightArtillery, MediumArtillery, HeavyArtillery, AAVehicle, SAMVehicle,
            AttackCopter, Interceptor, Bomber, APC, TransportCopter,
            CargoShip,GunBoat, AABoat, Cruiser][self.unit_type as usize]
        };
        Unit {
            unit_type,
            owner: if self.owner != 0 { Some(self.owner) } else { None },
            ..Unit::default()
        }
    }
}
impl JsonMap {
    fn into_map(self) -> Map {
        let tiles = {
            self.map_data.iter().enumerate()
                .map(|(i, t)| (i, t.as_tile(i)))
                .collect()
        };
        let units = {
            self.map_data.iter().enumerate()
                .filter_map(|(i, t)| t.unit.as_ref().map(|u| (i, u.as_unit())))
                .collect()
        };
        
        Map { name: self.name, funds: self.funds, units, tiles }
    }
}
#[cfg(test)]
mod test {
    use crate::game::*;
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");

    #[test]
    fn read_json() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        assert!(map.name == "Third party");
        assert!(map.funds == 0);

        // There should be a light tank at (14, 1)
        let tile_with_unit = map.tiles.values()
            .filter(|t| t.x == 14 && t.y == 1)
            .next().unwrap();
        
        assert!(tile_with_unit.unit.is_some());

        let unit_id = tile_with_unit.unit.unwrap();
        let unit = map.units.iter()
            .filter(|&(&i, _)| i == unit_id)
            .map(|(_, u)| u)
            .next().unwrap();

        assert!(unit.unit_type == model::UnitType::LightTank && unit.owner.is_none(), "{:?} == 1 && {:?} == None", unit.unit_type, unit.owner);
    }
}
