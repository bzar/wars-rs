use crate::game::*;
use crate::model::*;

impl Default for Unit {
    fn default() -> Unit {
        Unit {
            unit_type: model::UnitType::Infantry,
            health: UNIT_MAX_HEALTH,
            carried: Vec::new(),
            owner: None,
            deployed: false,
            moved: false,
            capturing: false
        }
    }
}

impl Unit {
    pub fn is_damaged(&self) -> bool {
        self.health < model::UNIT_MAX_HEALTH
    }
    pub fn unit_type_data(&self) -> UnitTypeData {
        unit_type(self.unit_type)
    }
    pub fn has_unit_flag(&self, flag: UnitFlag) -> bool {
        self.unit_type_data().flags.contains(&flag)
    }
    pub fn can_capture(&self) -> bool {
        self.has_unit_flag(UnitFlag::Capture)
    }
    pub fn can_deploy(&self) -> bool {
        self.unit_type_data().weapons.iter().any(|w| weapon(*w).require_deployed)
    }
    pub fn can_carry(&self, target: &Unit) -> bool {
        self.unit_type_data().carry_num > self.carried.len() as u32
            && self.unit_type_data().carry_classes.contains(&target.unit_type_data().unit_class)
    }
    pub fn can_move_on_terrain(&self, terrain_type: Terrain) -> bool {
        (movement(self.unit_type_data().movement_type).terrain_cost_map)(terrain_type).is_some()
    }
    pub fn defense_in_terrain(&self, terrain_type: Terrain) -> u32 {
        (self.unit_type_data().defense_map)(terrain_type).unwrap_or_else(|| terrain(terrain_type).default_defense)
    }
}

