use std::collections::HashSet;

pub fn random_bot(
    game: &mut wars::game::Game,
    emit: &mut dyn FnMut(wars::game::Event),
) -> Result<(), wars::game::ActionError> {
    let player_number = game.in_turn_number();

    let mut my_units: Vec<(wars::game::UnitId, wars::game::Unit)> = game
        .units
        .iter_with_ids()
        .filter_map(|(uid, u)| (u.owner == player_number).then_some((*uid, u.clone())))
        .collect();

    let being_carried: HashSet<wars::game::UnitId> = my_units
        .iter()
        .flat_map(|(_, unit)| unit.carried.iter().copied())
        .collect();

    fastrand::shuffle(&mut my_units);

    for (unit_id, unit) in my_units {
        if being_carried.contains(&unit_id) {
            continue;
        }
        if let Some(movement_options) = game.unit_move_options(unit_id) {
            if let Some(path) = fastrand::choice(movement_options.values()) {
                let destination = path.last().expect("Invalid path");
                let (tile_id, tile) = game.tiles.get_at(destination)?;

                if game.unit_can_stay_at(unit_id, destination).is_ok() {
                    let attack_options = game.unit_attack_options(unit_id, destination);

                    if !attack_options.is_empty() && fastrand::bool() {
                        let target_id = fastrand::choice(attack_options.keys()).unwrap();
                        wars::game::action::move_and_attack(game, unit_id, path, *target_id, emit)?;
                    } else if game.unit_can_capture_tile(unit_id, tile_id).is_ok()
                        && fastrand::bool()
                    {
                        wars::game::action::move_and_capture(game, unit_id, path, emit)?;
                    } else if unit.can_deploy() && fastrand::bool() {
                        if unit.deployed {
                            wars::game::action::undeploy(game, unit_id, emit)?;
                        } else {
                            wars::game::action::move_and_deploy(game, unit_id, path, emit)?;
                        }
                    } else if !unit.carried.is_empty() {
                        let carried_id = fastrand::choice(unit.carried).unwrap();
                        if let Some(unload_targets) =
                            game.unit_unload_options(unit_id, destination, carried_id)
                        {
                            if let Some(unload_position) = fastrand::choice(unload_targets) {
                                wars::game::action::move_and_unload(
                                    game,
                                    unit_id,
                                    path,
                                    carried_id,
                                    unload_position,
                                    emit,
                                )?;
                            }
                        }
                    } else {
                        wars::game::action::move_and_wait(game, unit_id, path, emit)?;
                    }
                } else if game.unit_can_load_into_carrier_at(unit_id, destination) {
                    wars::game::action::move_and_load_into(game, unit_id, path, emit)?;
                }
            }
        }
    }

    let mut my_bases: Vec<_> = game
        .tiles
        .iter_with_ids()
        .filter(|(_, t)| t.owner == player_number && !t.terrain_data().build_classes.is_empty())
        .map(|(tid, t)| (*tid, t.clone()))
        .collect();

    fastrand::shuffle(&mut my_bases);
    for (_, tile) in my_bases {
        if tile.unit.is_some() {
            continue;
        }
        let funds = game.in_turn_player().map(|p| p.funds).unwrap_or(0);
        let build_options: Vec<_> = enum_iterator::all::<wars::game::UnitType>()
            .map(|unit_type| (unit_type, wars::model::unit_type(unit_type)))
            .filter(|(_, info)| tile.terrain_data().build_classes.contains(&info.unit_class))
            .filter(|(_, info)| info.price < funds)
            .collect();
        if let Some((build_type, _)) = fastrand::choice(build_options) {
            wars::game::action::build(game, tile.position(), build_type, emit)?;
        }
    }
    wars::game::action::end_turn(game, emit)
}
