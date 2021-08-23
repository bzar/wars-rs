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
}
