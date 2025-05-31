use crate::game::{Credits, Health, Position, Tile, Unit};
use crate::model::*;

impl Default for Tile {
    fn default() -> Tile {
        use crate::model;
        Tile {
            terrain: model::Terrain::Road,
            terrain_subtype_id: 0,
            owner: None,
            x: 0,
            y: 0,
            unit: None,
            capture_points: model::MAX_CAPTURE_POINTS,
        }
    }
}

impl Tile {
    pub fn has_terrain_flag(&self, flag: TerrainFlag) -> bool {
        terrain(self.terrain).flags.contains(&flag)
    }
    pub fn can_repair_unit_class(&self, unit_class: UnitClass) -> bool {
        terrain(self.terrain).repair_classes.contains(&unit_class)
    }
    pub fn can_repair_unit(&self, unit: &Unit) -> bool {
        self.can_repair_unit_class(unit.unit_type_data().unit_class)
    }

    pub fn terrain_data(&self) -> TerrainData {
        terrain(self.terrain)
    }
    pub fn repair_rate(&self) -> Health {
        UNIT_MAX_REPAIR_RATE * self.capture_points / MAX_CAPTURE_POINTS
    }
    pub fn generated_funds(&self) -> Credits {
        if self.has_terrain_flag(TerrainFlag::Funds) {
            FUNDS_PER_PROPERTY * self.capture_points / MAX_CAPTURE_POINTS
        } else {
            0
        }
    }
    pub fn is_capturable(&self) -> bool {
        self.has_terrain_flag(TerrainFlag::Capturable)
    }
    pub fn can_build(&self, target_type: UnitType) -> bool {
        self.terrain_data()
            .build_classes
            .contains(&unit_type(target_type).unit_class)
    }
    pub fn position(&self) -> Position {
        Position(self.x, self.y)
    }
    pub fn max_capture_points(&self) -> u32 {
        MAX_CAPTURE_POINTS
    }
}
