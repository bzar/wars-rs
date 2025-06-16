use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use wars::{
    game::{Game, Position, TileId, UnitId, UnitType},
    model::UnitClass,
};

use crate::{Action, InputEvent};

pub struct InteractionStatePlugin;

impl Plugin for InteractionStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InteractionState::None)
            .add_systems(Startup, setup);
    }
}

#[derive(Resource)]
pub enum InteractionState {
    None,
    SelectUnitOrBase(HashSet<UnitId>, HashSet<TileId>),
    SelectDestination {
        unit_id: UnitId,
        destination_options: HashMap<Position, Vec<Position>>,
    },
    SelectAction {
        unit_id: UnitId,
        path: Vec<Position>,
        action_options: HashSet<Action>,
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

#[derive(Debug)]
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
    SelectUnitOrBase(HashSet<UnitId>, HashSet<TileId>),
    SelectDestination(HashSet<Position>),
    CancelSelectDestination,
    SelectAction(HashSet<Action>),
    SelectedAction(Action),
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid state")]
    InvalidState,
    #[error("Action error")]
    ActionError(#[from] wars::game::ActionError),
}
pub type InteractionResult<T = ()> = Result<T, Error>;

impl InteractionState {
    pub fn from_game(game: &Game) -> Self {
        let units = game
            .units
            .iter_with_ids()
            .filter_map(|(id, unit)| {
                (unit.owner == game.in_turn_number() && !unit.moved).then_some(*id)
            })
            .collect();
        let tiles = game
            .tiles
            .iter_with_ids()
            .filter_map(|(id, tile)| {
                (tile.owner == game.in_turn_number()
                    && !tile.terrain_data().build_classes.is_empty()
                    && tile.unit.is_none())
                .then_some(*id)
            })
            .collect();
        Self::SelectUnitOrBase(units, tiles)
    }
    pub fn handle(
        &mut self,
        event: InputEvent,
        game: &mut Game,
        emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionResult {
        match event {
            InputEvent::MapSelect(tile_id) => self.select_tile(game, tile_id, emit),
            InputEvent::Action(action) => self.select_action(game, action, emit),
            InputEvent::UnloadUnit(unit_id) => self.select_unit_to_unload(game, unit_id, emit),
            InputEvent::BuildUnit(unit_type) => {
                self.select_unit_type_to_build(game, unit_type, emit)
            }
            InputEvent::EndTurn => self.end_turn(game, emit),
        }
    }
    pub fn select_tile(
        &mut self,
        game: &mut Game,
        tile_id: TileId,
        mut emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionResult {
        *self = match self.consume() {
            InteractionState::SelectUnitOrBase(units, tiles) => {
                select_unit_or_base(game, tile_id, units, tiles, emit)?
            }
            InteractionState::SelectDestination {
                unit_id,
                destination_options,
            } => select_destination(game, unit_id, tile_id, destination_options, emit)?,
            InteractionState::SelectAttackTarget {
                unit_id,
                path,
                attack_options,
            } => select_attack_target(game, unit_id, path, tile_id, attack_options, emit)?,
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
            )?,
            InteractionState::SelectUnitToBuild { .. } => {
                emit(InteractionEvent::CancelSelectUnitToBuild, game);
                InteractionState::reset(game, emit)
            }
            InteractionState::None => return Err(Error::InvalidState),
            other @ _ => other,
        };
        Ok(())
    }

    pub fn select_action(
        &mut self,
        game: &mut Game,
        action: Action,
        mut emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionResult {
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
            )?,
            InteractionState::SelectDestination { .. } if action == Action::Cancel => {
                emit(InteractionEvent::CancelSelectDestination, game);
                InteractionState::reset(game, emit)
            }
            InteractionState::SelectAttackTarget { .. } if action == Action::Cancel => {
                emit(InteractionEvent::CancelSelectAttackTarget, game);
                InteractionState::reset(game, emit)
            }
            InteractionState::SelectUnitToBuild { .. } if action == Action::Cancel => {
                emit(InteractionEvent::CancelSelectUnitToBuild, game);
                InteractionState::reset(game, emit)
            }
            InteractionState::SelectUnitToUnload { .. } if action == Action::Cancel => {
                emit(InteractionEvent::CancelSelectUnloadUnit, game);
                InteractionState::reset(game, emit)
            }
            InteractionState::SelectUnloadDestination { .. } if action == Action::Cancel => {
                emit(InteractionEvent::CancelSelectUnloadDestination, game);
                InteractionState::reset(game, emit)
            }
            InteractionState::None => return Err(Error::InvalidState),
            other @ _ => other,
        };
        Ok(())
    }

    pub fn select_unit_to_unload(
        &mut self,
        game: &mut Game,
        unit_id: UnitId,
        emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionResult {
        *self = match self.consume() {
            InteractionState::SelectUnitToUnload { carrier_id, path } => {
                select_unit_to_unload(game, carrier_id, path, unit_id, emit)?
            }
            InteractionState::None => return Err(Error::InvalidState),
            other @ _ => other,
        };
        Ok(())
    }

    pub fn select_unit_type_to_build(
        &mut self,
        game: &mut Game,
        unit_type: UnitType,
        emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionResult {
        *self = match self.consume() {
            InteractionState::SelectUnitToBuild { tile_id } => {
                select_unit_type_to_build(game, tile_id, unit_type, emit)?
            }
            InteractionState::None => return Err(Error::InvalidState),
            other @ _ => other,
        };
        Ok(())
    }

    pub fn end_turn(
        &mut self,
        game: &mut Game,
        mut emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionResult {
        emit(InteractionEvent::EndTurn, game);
        *self = InteractionState::reset(game, emit);
        Ok(())
    }

    pub fn reset(
        game: &mut Game,
        mut emit: impl FnMut(InteractionEvent, &mut Game),
    ) -> InteractionState {
        let state = InteractionState::from_game(game);
        let InteractionState::SelectUnitOrBase(ref units, ref tiles) = state else {
            panic!("InteractionState::from_game produced the wrong state")
        };
        emit(
            InteractionEvent::SelectUnitOrBase(units.clone(), tiles.clone()),
            game,
        );
        state
    }
    fn consume(&mut self) -> InteractionState {
        let mut state = InteractionState::None;
        std::mem::swap(self, &mut state);
        state
    }
}

fn select_unit_or_base(
    game: &mut Game,
    tile_id: TileId,
    units: HashSet<UnitId>,
    tiles: HashSet<TileId>,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    let tile = game
        .tiles
        .get(tile_id)
        .ok_or(wars::game::ActionError::TileNotFound)?;
    if let Some(unit_id) = tile.unit {
        if units.contains(&unit_id) {
            if let Some(destination_options) = game.unit_move_options(unit_id) {
                emit(
                    InteractionEvent::SelectDestination(
                        destination_options.keys().cloned().collect(),
                    ),
                    game,
                );
                return Ok(InteractionState::SelectDestination {
                    unit_id,
                    destination_options,
                });
            }
        }
    } else if tiles.contains(&tile_id) {
        emit(
            InteractionEvent::SelectUnitToBuild(
                tile.terrain_data().build_classes.iter().copied().collect(),
            ),
            game,
        );
        return Ok(InteractionState::SelectUnitToBuild { tile_id });
    }
    Ok(InteractionState::SelectUnitOrBase(units, tiles))
}

fn select_destination(
    game: &mut Game,
    unit_id: UnitId,
    tile_id: TileId,
    mut destination_options: HashMap<Position, Vec<Position>>,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    let unit = game
        .units
        .get_ref(&unit_id)
        .ok_or(wars::game::ActionError::UnitNotFound)?;
    let tile = game
        .tiles
        .get(tile_id)
        .ok_or(wars::game::ActionError::TileNotFound)?;
    let position = tile.position();

    let Some(path) = destination_options.remove(&position) else {
        emit(InteractionEvent::CancelSelectDestination, game);
        return Ok(InteractionState::reset(game, emit));
    };

    let mut action_options = HashSet::from([Action::Cancel]);

    if game.unit_can_stay_at(unit_id, &position).is_ok() {
        action_options.insert(Action::Wait);

        if unit.can_deploy() && !unit.deployed {
            action_options.insert(Action::Deploy);
        }
    }

    let attack_options = game.unit_attack_options(unit_id, &position);

    if !attack_options.is_empty() {
        action_options.insert(Action::Attack);
    }

    if game.unit_can_load_into_carrier_at(unit_id, &position) {
        action_options.insert(Action::Load);
    }

    if game.unit_can_capture_tile(unit_id, tile_id).is_ok() {
        action_options.insert(Action::Capture);
    }
    if unit.carried.iter().any(|u| {
        game.unit_unload_options(unit_id, &position, *u)
            .is_some_and(|os| !os.is_empty())
    }) {
        action_options.insert(Action::Unload);
    }
    emit(InteractionEvent::SelectAction(action_options.clone()), game);
    Ok(InteractionState::SelectAction {
        unit_id,
        path,
        action_options,
        attack_options,
    })
}
fn select_attack_target(
    game: &mut Game,
    unit_id: UnitId,
    path: Vec<Position>,
    tile_id: TileId,
    attack_options: HashMap<UnitId, wars::game::Health>,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    let tile = game
        .tiles
        .get(tile_id)
        .ok_or(wars::game::ActionError::TileNotFound)?;
    let Some(target_id) = tile.unit else {
        emit(InteractionEvent::CancelSelectAttackTarget, game);
        return Ok(InteractionState::reset(game, emit));
    };

    if !attack_options.contains_key(&target_id) {
        emit(InteractionEvent::CancelSelectAttackTarget, game);
        return Ok(InteractionState::reset(game, emit));
    };
    emit(
        InteractionEvent::MoveAndAttack(unit_id, path, target_id),
        game,
    );
    Ok(InteractionState::reset(game, emit))
}

fn select_unload_destination(
    game: &mut Game,
    carrier_id: UnitId,
    path: Vec<Position>,
    unit_id: UnitId,
    tile_id: TileId,
    unload_options: HashSet<Position>,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    let tile = game
        .tiles
        .get(tile_id)
        .ok_or(wars::game::ActionError::TileNotFound)?;
    let position = tile.position();
    if !unload_options.contains(&position) {
        return Ok(InteractionState::SelectUnloadDestination {
            carrier_id,
            path,
            unit_id,
            unload_options,
        });
    }
    emit(
        InteractionEvent::MoveAndUnloadUnitTo(carrier_id, path, unit_id, position),
        game,
    );
    Ok(InteractionState::reset(game, emit))
}
fn select_action(
    game: &mut Game,
    unit_id: UnitId,
    path: Vec<Position>,
    action: Action,
    action_options: HashSet<Action>,
    attack_options: HashMap<UnitId, wars::game::Health>,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    if !action_options.contains(&action) {
        return Err(wars::game::ActionError::InternalError.into());
    }
    emit(InteractionEvent::SelectedAction(action), game);
    match action {
        Action::Wait => {
            emit(InteractionEvent::MoveAndWait(unit_id, path), game);
            return Ok(InteractionState::reset(game, emit));
        }
        Action::Attack => {
            emit(
                InteractionEvent::SelectAttackTarget(attack_options.clone()),
                game,
            );
            Ok(InteractionState::SelectAttackTarget {
                unit_id,
                path,
                attack_options,
            })
        }
        Action::Capture => {
            emit(InteractionEvent::MoveAndCapture(unit_id, path), game);
            Ok(InteractionState::reset(game, emit))
        }
        Action::Deploy => {
            emit(InteractionEvent::MoveAndDeploy(unit_id, path), game);
            Ok(InteractionState::reset(game, emit))
        }
        Action::Undeploy => {
            emit(InteractionEvent::Undeploy(unit_id), game);
            Ok(InteractionState::reset(game, emit))
        }
        Action::Load => {
            emit(InteractionEvent::MoveAndLoadInto(unit_id, path), game);
            Ok(InteractionState::reset(game, emit))
        }
        Action::Unload => {
            let unit = game.units.get_ref(&unit_id).expect("Unit does not exist");
            emit(
                InteractionEvent::SelectUnloadUnit(unit.carried.clone()),
                game,
            );

            Ok(InteractionState::SelectUnitToUnload {
                carrier_id: unit_id,
                path,
            })
        }
        Action::Cancel => {
            emit(InteractionEvent::CancelSelectAction, game);
            Ok(InteractionState::reset(game, emit))
        }
    }
}

fn select_unit_to_unload(
    game: &mut Game,
    carrier_id: UnitId,
    path: Vec<Position>,
    unit_id: UnitId,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    let position = path.last().expect("Invalid path");
    let unload_options = game
        .unit_unload_options(carrier_id, position, unit_id)
        .ok_or(wars::game::ActionError::CannotUnload)?;
    emit(
        InteractionEvent::SelectUnloadDestination(unload_options.clone()),
        game,
    );
    Ok(InteractionState::SelectUnloadDestination {
        carrier_id,
        path,
        unit_id,
        unload_options,
    })
}
fn select_unit_type_to_build(
    game: &mut Game,
    tile_id: TileId,
    unit_type: UnitType,
    mut emit: impl FnMut(InteractionEvent, &mut Game),
) -> InteractionResult<InteractionState> {
    let tile = game
        .tiles
        .get(tile_id)
        .ok_or(wars::game::ActionError::TileNotFound)?;
    if !tile.can_build(unit_type) {
        return Err(wars::game::ActionError::CannotBuild.into());
    }

    emit(InteractionEvent::BuildUnit(tile_id, unit_type), game);
    Ok(InteractionState::reset(game, emit))
}

fn setup(game: Res<crate::Game>, mut interaction_state: ResMut<InteractionState>) {
    *interaction_state = InteractionState::from_game(&game);
}
