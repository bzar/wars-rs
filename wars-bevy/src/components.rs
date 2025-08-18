use bevy::prelude::*;
use std::collections::HashSet;
use wars::model::UNIT_MAX_HEALTH;

#[derive(Component)]
pub struct Tile(pub wars::game::TileId);

#[derive(Component)]
pub struct Prop(pub wars::game::TileId);

#[derive(Component)]
pub struct Unit(pub wars::game::UnitId);

#[derive(Component)]
pub struct Carrier {
    pub load: u32,
    pub capacity: u32,
}

#[derive(Component)]
pub struct CarrierSlot(pub u32);

#[derive(Component)]
pub struct DeployEmblem;

#[derive(Component)]
pub struct Deployed(pub bool);

#[derive(Component)]
pub struct Moved(pub bool);

#[derive(Component)]
pub struct InAttackRange(pub bool);

#[derive(Component)]
pub struct AttackRangeIndicator;

#[derive(Component)]
pub enum CaptureState {
    Capturing(u32),
    Recovering(u32),
    Full,
}

#[derive(Component)]
pub struct CaptureBar;

#[derive(Component)]
pub struct CaptureBarBit(pub u32);

#[derive(Component)]
pub struct UnitMovePreview(pub Vec<wars::game::Position>);

#[derive(Component)]
pub struct UnitMovePreviewProp(pub Entity);

#[derive(Component)]
pub enum TileHighlight {
    Normal,
    Unmovable,
    Movable,
}

#[derive(Component)]
pub enum UnitHighlight {
    Normal,
    Target,
}

#[derive(Component)]
pub enum Health {
    Full,
    Damaged(u32),
}

impl Health {
    pub fn from_value(health: u32) -> Self {
        if health >= UNIT_MAX_HEALTH {
            Self::Full
        } else {
            Self::Damaged(health)
        }
    }
    pub fn value(&self) -> u32 {
        match self {
            Self::Full => UNIT_MAX_HEALTH,
            Self::Damaged(health) => *health,
        }
    }
    pub fn damage(&self, x: u32) -> Self {
        if x > self.value() {
            Self::Damaged(0)
        } else {
            Self::Damaged(self.value() - x)
        }
    }
}
#[derive(Component)]
pub struct HealthOnesDigit;
#[derive(Component)]
pub struct HealthTensDigit;

#[derive(Component)]
pub enum DamageIndicator {
    Hidden,
    Visible(u32),
}
#[derive(Component)]
pub struct DamageOnesDigit;
#[derive(Component)]
pub struct DamageTensDigit;
#[derive(Component)]
pub struct DamageHundredsDigit;

#[derive(Component)]
pub struct Owner(pub u32);

#[derive(Component)]
pub struct EndTurnButton;

#[derive(Component)]
pub struct MenuBar;

#[derive(Component)]
pub struct UnloadMenu;

#[derive(Component)]
pub struct Funds(pub u32);

impl Funds {
    pub fn deduct(&self, amount: u32) -> Self {
        Self(self.0.saturating_sub(amount))
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, enum_iterator::Sequence)]
pub enum Action {
    Wait,
    Attack,
    Capture,
    Deploy,
    Undeploy,
    Load,
    Unload,
    Cancel,
}

#[derive(Event, Debug, Clone, Copy)]
pub enum InputEvent {
    MapSelect(wars::game::TileId),
    MapHover(wars::game::TileId),
    Action(Action),
    UnloadUnit(wars::game::UnitId),
    BuildUnit(wars::game::UnitType),
    EndTurn,
}

#[derive(Event)]
pub struct GameEvent(pub wars::game::Event);

#[derive(Event)]
pub struct GameAction(pub wars::game::Action);

#[derive(Component)]
pub struct ActionMenu;

#[derive(Component)]
pub struct BuildMenu;

#[derive(Component)]
pub struct DisabledButton;

#[derive(Component)]
pub struct PlayerColored;
