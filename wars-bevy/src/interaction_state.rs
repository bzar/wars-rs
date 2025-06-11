use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use wars::{
    game::{Game, Position, TileId, UnitId, UnitType},
    model::UnitClass,
};

use crate::MapAction;

pub struct InteractionStatePlugin;

impl Plugin for InteractionStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InteractionState::default())
            .add_event::<InteractionEvent>();
    }
}

#[derive(Resource, Default)]
pub enum InteractionState {
    #[default]
    Initial,
    SelectDestination {
        unit_id: UnitId,
        destination_options: HashMap<Position, Vec<Position>>,
    },
    SelectAction {
        unit_id: UnitId,
        path: Vec<Position>,
        action_options: HashSet<MapAction>,
        attack_options: HashMap<UnitId, wars::game::Health>,
    },
    SelectAttackTarget {
        unit_id: UnitId,
        path: Vec<Position>,
        attack_options: HashMap<UnitId, wars::game::Health>,
    },
    SelectUnitToBuild {
        tile_id: TileId,
    },
    SelectUnitToUnload {
        carrier_id: UnitId,
        path: Vec<Position>,
    },
    SelectUnloadDestination {
        carrier_id: UnitId,
        path: Vec<Position>,
        unit_id: UnitId,
        unload_options: HashSet<Position>,
    },
}

#[derive(Event, Debug)]
pub enum InteractionEvent {
    EndTurn,
    MoveAndWait(UnitId, Vec<Position>),
    MoveAndAttack(UnitId, Vec<Position>, UnitId),
    MoveAndCapture(UnitId, Vec<Position>),
    MoveAndDeploy(UnitId, Vec<Position>),
    Undeploy(UnitId),
    MoveAndLoadInto(UnitId, Vec<Position>),
    MoveAndUnloadUnitTo(UnitId, Vec<Position>, UnitId, Position),
    BuildUnit(TileId, UnitType),
    SelectDestination(HashSet<Position>),
    CancelSelectDestination,
    SelectAction(HashSet<MapAction>),
    CancelSelectAction,
    SelectAttackTarget(HashMap<UnitId, wars::game::Health>),
    CancelSelectAttackTarget,
    SelectUnloadUnit(Vec<UnitId>),
    CancelSelectUnloadUnit,
    SelectUnloadDestination(HashSet<Position>),
    CancelSelectUnloadDestination,
    SelectUnitToBuild(HashSet<UnitClass>),
    CancelSelectUnitToBuild,
}
impl InteractionState {
    pub fn select_tile(
        &mut self,
        game: &Game,
        tile_id: TileId,
        mut emit: impl FnMut(InteractionEvent),
    ) {
        *self = match self.consume() {
            InteractionState::Initial => select_unit_or_base(game, tile_id, emit),
            InteractionState::SelectDestination {
                unit_id,
                destination_options,
            } => select_destination(game, unit_id, tile_id, destination_options, emit),
            InteractionState::SelectAttackTarget {
                unit_id,
                path,
                attack_options,
            } => select_attack_target(game, unit_id, path, tile_id, attack_options, emit),
            InteractionState::SelectUnloadDestination {
                carrier_id,
                path,
                unit_id,
                unload_options,
            } => select_unload_destination(
                game,
                carrier_id,
                path,
                unit_id,
                tile_id,
                unload_options,
                emit,
            ),
            InteractionState::SelectUnitToBuild { .. } => {
                emit(InteractionEvent::CancelSelectUnitToBuild);
                InteractionState::Initial
            }
            other @ _ => other,
        };
    }

    pub fn select_action(
        &mut self,
        game: &Game,
        action: MapAction,
        mut emit: impl FnMut(InteractionEvent),
    ) {
        *self = match self.consume() {
            InteractionState::SelectAction {
                unit_id,
                path,
                action_options,
                attack_options,
            } => select_action(
                game,
                unit_id,
                path,
                action,
                action_options,
                attack_options,
                emit,
            ),
            InteractionState::SelectDestination { .. } if action == MapAction::Cancel => {
                emit(InteractionEvent::CancelSelectDestination);
                InteractionState::Initial
            }
            InteractionState::SelectAttackTarget { .. } if action == MapAction::Cancel => {
                emit(InteractionEvent::CancelSelectAttackTarget);
                InteractionState::Initial
            }
            InteractionState::SelectUnitToBuild { .. } if action == MapAction::Cancel => {
                emit(InteractionEvent::CancelSelectUnitToBuild);
                InteractionState::Initial
            }
            InteractionState::SelectUnitToUnload { .. } if action == MapAction::Cancel => {
                emit(InteractionEvent::CancelSelectUnloadUnit);
                InteractionState::Initial
            }
            InteractionState::SelectUnloadDestination { .. } if action == MapAction::Cancel => {
                emit(InteractionEvent::CancelSelectUnloadDestination);
                InteractionState::Initial
            }
            other @ _ => other,
        };
    }

    pub fn select_unit_to_unload(
        &mut self,
        game: &Game,
        unit_id: UnitId,
        emit: impl FnMut(InteractionEvent),
    ) {
        *self = match self.consume() {
            InteractionState::SelectUnitToUnload { carrier_id, path } => {
                select_unit_to_unload(game, carrier_id, path, unit_id, emit)
            }
            other @ _ => other,
        };
    }

    pub fn select_unit_type_to_build(
        &mut self,
        game: &Game,
        unit_type: UnitType,
        emit: impl FnMut(InteractionEvent),
    ) {
        *self = match self.consume() {
            InteractionState::SelectUnitToBuild { tile_id } => {
                select_unit_type_to_build(game, tile_id, unit_type, emit)
            }
            other @ _ => other,
        };
    }

    pub fn end_turn(&mut self, mut emit: impl FnMut(InteractionEvent)) {
        emit(InteractionEvent::EndTurn);
    }

    fn consume(&mut self) -> InteractionState {
        let mut state = InteractionState::Initial;
        std::mem::swap(self, &mut state);
        state
    }
}

fn select_unit_or_base(
    game: &Game,
    tile_id: TileId,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    let tile = game.tiles.get(tile_id).expect("Tile does not exist");
    if let Some(unit_id) = tile.unit {
        let unit = game.units.get(unit_id).expect("Unit does not exist");
        if unit.owner == game.in_turn_number() && !unit.moved {
            if let Some(destination_options) = game.unit_move_options(unit_id) {
                emit(InteractionEvent::SelectDestination(
                    destination_options.keys().cloned().collect(),
                ));
                return InteractionState::SelectDestination {
                    unit_id,
                    destination_options,
                };
            }
        }
    } else if !tile.terrain_data().build_classes.is_empty() && tile.owner == game.in_turn_number() {
        emit(InteractionEvent::SelectUnitToBuild(
            tile.terrain_data().build_classes.iter().copied().collect(),
        ));
        return InteractionState::SelectUnitToBuild { tile_id };
    }
    InteractionState::Initial
}

fn select_destination(
    game: &Game,
    unit_id: UnitId,
    tile_id: TileId,
    mut destination_options: HashMap<Position, Vec<Position>>,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    let unit = game.units.get_ref(&unit_id).expect("Unit does not exist");
    let tile = game.tiles.get(tile_id).expect("Tile does not exist");
    let position = tile.position();

    let Some(path) = destination_options.remove(&position) else {
        emit(InteractionEvent::CancelSelectDestination);
        return InteractionState::Initial;
    };

    let mut action_options = HashSet::from([MapAction::Cancel]);

    if game.unit_can_stay_at(unit_id, &position).is_ok() {
        action_options.insert(MapAction::Wait);

        if unit.can_deploy() && !unit.deployed {
            action_options.insert(MapAction::Deploy);
        }
    }

    let attack_options = game.unit_attack_options(unit_id, &position);

    if !attack_options.is_empty() {
        action_options.insert(MapAction::Attack);
    }

    if game.unit_can_load_into_carrier_at(unit_id, &position) {
        action_options.insert(MapAction::Load);
    }

    if game.unit_can_capture_tile(unit_id, tile_id).is_ok() {
        action_options.insert(MapAction::Capture);
    }
    if unit.carried.iter().any(|u| {
        game.unit_unload_options(unit_id, &position, *u)
            .is_some_and(|os| !os.is_empty())
    }) {
        action_options.insert(MapAction::Unload);
    }
    emit(InteractionEvent::SelectAction(action_options.clone()));
    InteractionState::SelectAction {
        unit_id,
        path,
        action_options,
        attack_options,
    }
}
fn select_attack_target(
    game: &Game,
    unit_id: UnitId,
    path: Vec<Position>,
    tile_id: TileId,
    attack_options: HashMap<UnitId, wars::game::Health>,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    let tile = game.tiles.get(tile_id).expect("Tile does not exist");
    let Some(target_id) = tile.unit else {
        return InteractionState::Initial;
    };

    if !attack_options.contains_key(&target_id) {
        return InteractionState::Initial;
    };
    emit(InteractionEvent::MoveAndAttack(unit_id, path, target_id));
    InteractionState::Initial
}

fn select_unload_destination(
    game: &Game,
    carrier_id: UnitId,
    path: Vec<Position>,
    unit_id: UnitId,
    tile_id: TileId,
    unload_options: HashSet<Position>,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    let tile = game.tiles.get(tile_id).expect("Tile does not exist");
    let position = tile.position();
    if !unload_options.contains(&position) {
        return InteractionState::SelectUnloadDestination {
            carrier_id,
            path,
            unit_id,
            unload_options,
        };
    }
    emit(InteractionEvent::MoveAndUnloadUnitTo(
        carrier_id, path, unit_id, position,
    ));
    InteractionState::Initial
}
fn select_action(
    game: &Game,
    unit_id: UnitId,
    path: Vec<Position>,
    action: MapAction,
    action_options: HashSet<MapAction>,
    attack_options: HashMap<UnitId, wars::game::Health>,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    if !action_options.contains(&action) {
        panic!("Action is not permitted here");
    }
    match action {
        MapAction::Wait => {
            emit(InteractionEvent::MoveAndWait(unit_id, path));
            InteractionState::Initial
        }
        MapAction::Attack => {
            emit(InteractionEvent::SelectAttackTarget(attack_options.clone()));
            InteractionState::SelectAttackTarget {
                unit_id,
                path,
                attack_options,
            }
        }
        MapAction::Capture => {
            emit(InteractionEvent::MoveAndCapture(unit_id, path));
            InteractionState::Initial
        }
        MapAction::Deploy => {
            emit(InteractionEvent::MoveAndDeploy(unit_id, path));
            InteractionState::Initial
        }
        MapAction::Undeploy => {
            emit(InteractionEvent::Undeploy(unit_id));
            InteractionState::Initial
        }
        MapAction::Load => {
            emit(InteractionEvent::MoveAndLoadInto(unit_id, path));
            InteractionState::Initial
        }
        MapAction::Unload => {
            let unit = game.units.get_ref(&unit_id).expect("Unit does not exist");
            emit(InteractionEvent::SelectUnloadUnit(unit.carried.clone()));

            InteractionState::SelectUnitToUnload {
                carrier_id: unit_id,
                path,
            }
        }
        MapAction::Cancel => {
            emit(InteractionEvent::CancelSelectAction);
            InteractionState::Initial
        }
    }
}

fn select_unit_to_unload(
    game: &Game,
    carrier_id: UnitId,
    path: Vec<Position>,
    unit_id: UnitId,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    let position = path.last().expect("Invalid path");
    let unload_options = game
        .unit_unload_options(carrier_id, position, unit_id)
        .expect("Unit does not have valid unload options");
    emit(InteractionEvent::SelectUnloadDestination(
        unload_options.clone(),
    ));
    InteractionState::SelectUnloadDestination {
        carrier_id,
        path,
        unit_id,
        unload_options,
    }
}
fn select_unit_type_to_build(
    game: &Game,
    tile_id: TileId,
    unit_type: UnitType,
    mut emit: impl FnMut(InteractionEvent),
) -> InteractionState {
    let tile = game.tiles.get(tile_id).expect("Tile does not exist");
    if !tile.can_build(unit_type) {
        panic!("Tile can not build unit type");
    }

    emit(InteractionEvent::BuildUnit(tile_id, unit_type));
    InteractionState::Initial
}
