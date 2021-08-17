use crate::game::*;

impl Default for Unit {
    fn default() -> Unit {
        Unit {
            unit_type: model::UnitType::Infantry,
            health: 0,
            carried: Vec::new(),
            owner: None,
            deployed: false,
            moved: false,
            capturing: false
        }
    }
}

