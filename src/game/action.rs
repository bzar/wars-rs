use crate::game::*;
use crate::model::*;

pub fn start(game: &mut Game, emit:  &mut dyn FnMut(Event)) -> ActionResult<()> {
    game.set_state(GameState::InProgress).map_err(|_| ActionError::GameAlreadyStarted)?;
    let in_turn_number = game.in_turn_number().ok_or(ActionError::InternalError)?;
    start_turn(game, in_turn_number, emit)?;
    Ok(())
}


fn start_turn(game: &mut Game, player_number: PlayerNumber, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    emit(Event::StartTurn(player_number));

    let mut player = game.get_player(player_number).ok_or(ActionError::InternalError)?;

    // Generate player funds
    let generated_funds = game.tiles.owned_by_player(player_number)
            .map(|(_, tile)| tile.generated_funds())
            .sum();

    player.funds += generated_funds;
    game.players.update(player)?;
    emit(Event::Funds(player_number, generated_funds));

    // Reset unit capture statuses
    game.units.owned_by_player(player_number)
        .filter(|(_, unit)| unit.capturing)
        .map(|(unit_id, unit)| (unit_id, Unit { capturing: false, ..unit.clone() }))
        .collect::<Vec<_>>()
        .into_iter()
        .try_for_each(|(unit_id, unit)| {
            game.units.update(unit_id, unit)
        })?;

    // Regenerate capture points
    game.tiles.owned_by_player(player_number)
        .filter(|(_, tile)| tile.capture_points < MAX_CAPTURE_POINTS)
        .filter_map(|(tile_id, tile)| Some((tile_id, tile, game.units.get(tile.unit?)?)))
        .filter(|(_, _, unit)| !unit.capturing)
        .map(|(tile_id, tile, _)| {
            let new_tile_capture_points = (tile.capture_points + CAPTURE_POINT_REGEN_RATE).min(MAX_CAPTURE_POINTS);
            (tile_id, Tile { capture_points: new_tile_capture_points, ..tile.clone() })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .try_for_each(|(tile_id, tile)| {
            emit(Event::TileCapturePointRegen(tile_id, tile.capture_points));
            game.tiles.update(tile_id, tile)
        })?;
    
    // Repair units
    game.tiles.owned_by_player(player_number)
        .filter_map(|(_, tile)| tile.unit.map(|unit_id| (unit_id, tile)))
        .filter_map(|(unit_id, tile)| game.units.get(unit_id).map(|unit| (unit_id, unit, tile)))
        .filter(|(_, unit, tile)| unit.is_damaged() && tile.can_repair_unit(&unit))
        .map(|(unit_id, mut unit, tile)| {
            let new_unit_health = (unit.health + tile.repair_rate()).min(UNIT_MAX_HEALTH);
            unit.health = new_unit_health;
            (unit_id, unit)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .try_for_each(|(unit_id, unit)| {
            emit(Event::UnitRepair(unit_id, unit.health));
            game.units.update(unit_id, unit)
        })?;

    Ok(())
}
fn finish_turn(game: &mut Game, player_number: PlayerNumber, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    // Reset moved units
    game.units.iter_with_ids()
        .filter(|(_, u)| u.moved)
        .map(|(i, u)| (*i, Unit { moved: false, ..u.clone() }))
        .collect::<Vec<_>>()
        .into_iter()
        .try_for_each(|(unit_id, unit)| game.units.update(unit_id, unit))?;

    emit(Event::EndTurn(player_number));
    Ok(())
}
pub fn end_turn(game: &mut Game, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let in_turn_number = game.in_turn_number().ok_or(ActionError::InternalError)?;
    finish_turn(game, in_turn_number, emit)?;
    
    // Update player alive statuses
    let players_with_units = game.players_with_units();
    let players_with_build_tiles = game.players_with_build_tiles();

    let updated_players: Vec<_> = game.players.iter().filter_map(|p| {
        let alive = players_with_units.contains(&p.number) || players_with_build_tiles.contains(&p.number);
        if p.alive != alive {
            Some(Player { alive, ..*p})
        } else {
            None
        }
    }).collect();

    updated_players.into_iter().try_for_each(|p| game.players.update(p))?;

    // Check win condition
    if let Some(winner) = game.winner() {
        emit(Event::WinGame(winner));
        game.set_state(GameState::Finished)?;
        return Ok(());
    }

    // Set next player in turn
    let in_turn_number = game.next_player_number().ok_or(ActionError::InternalError)?;
    game.set_player_in_turn(in_turn_number)?;

    start_turn(game, in_turn_number, emit)?;

    Ok(())
}

pub fn surrender(game: &mut Game, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let in_turn_number = game.in_turn_number().ok_or(ActionError::GameNotInProgress)?;

    // Neutralize owned tiles
    game.tiles.owned_by_player(in_turn_number)
        .map(|(tile_id, tile)| (tile_id, Tile { owner: None, ..*tile }))
        .collect::<Vec<_>>()
        .into_iter()
        .try_for_each(|(tile_id, tile)| game.tiles.update(tile_id, tile))?;

    // Neutralize owned units
    game.units.owned_by_player(in_turn_number)
        .map(|(unit_id, unit)| (unit_id, Unit { owner: None, ..unit.clone() }))
        .collect::<Vec<_>>()
        .into_iter()
        .try_for_each(|(unit_id, unit)| game.units.update(unit_id, unit))?;

    emit(Event::Surrender(in_turn_number));

    end_turn(game, emit)
}

pub fn build(game: &mut Game, position: Position, build_type: UnitType, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let mut in_turn_player = game.in_turn_player().ok_or(ActionError::GameNotInProgress)?;
    let (tile_id, mut tile) = game.tiles.get_at(&position)?;

    if tile.owner != Some(in_turn_player.number) {
        return Err(ActionError::OwnerNotInTurn);
    }
    if !tile.can_build(build_type) || tile.unit.is_some() {
        return Err(ActionError::CannotBuild);
    }
    let price = unit_type(build_type).price;
    if in_turn_player.funds < price {
        return Err(ActionError::InsufficientFunds);
    }

    let unit = Unit { unit_type: build_type, moved: true, ..Unit::default() };
    let unit_id = game.units.insert(unit);
    tile.unit = Some(unit_id);
    in_turn_player.funds -= price;

    emit(Event::Build(tile_id, unit_id, build_type, price));

    game.tiles.update(tile_id, tile)?;
    game.players.update(in_turn_player)?;

    Ok(())
}

/// Helper function for actions that move a unit
fn try_move(game: &mut Game, unit_id: UnitId, path: &[Position]) -> ActionResult<(TileId, Tile, TileId, Tile, Unit)> {
    let unit = game.units.get(unit_id).ok_or(ActionError::UnitNotFound)?;

    game.unit_has_turn(&unit)?;
    game.unit_can_move_path(unit_id, path)?;
    game.unit_can_stay_at(unit_id, &path[path.len() - 1])?;

    let (src_tile_id, src_tile) = game.tiles.get_unit_tile(unit_id)?;
    let (dst_tile_id, dst_tile) = game.tiles.get_at(path.last().ok_or(ActionError::InvalidPath)?)?;
    Ok((src_tile_id, src_tile, dst_tile_id, dst_tile, unit))
}

pub fn move_and_wait(game: &mut Game, unit_id: UnitId, path: &[Position], emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let (src_tile_id, mut src_tile, dst_tile_id, mut dst_tile, mut unit) = try_move(game, unit_id, path)?;

    unit.moved = true;
    src_tile.unit = None;
    dst_tile.unit = Some(unit_id);

    game.update_tiles_and_units(
        [(src_tile_id, src_tile), (dst_tile_id, dst_tile)],
        [(unit_id, unit)])?;

    emit(Event::Move(unit_id, path.into()));
    emit(Event::Wait(unit_id));
    Ok(())
}

pub fn move_and_attack(_game: &mut Game) -> ActionResult<()> {
    unimplemented!()
}

pub fn move_and_capture(game: &mut Game, unit_id: UnitId, path: &[Position], emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let (src_tile_id, mut src_tile, dst_tile_id, mut dst_tile, mut unit) = try_move(game, unit_id, path)?;

    if !unit.can_capture() || !dst_tile.is_capturable() || dst_tile.owner == unit.owner {
        return Err(ActionError::CannotCapture)
    }

    unit.moved = true;
    unit.capturing = true;
    src_tile.unit = None;
    dst_tile.unit = Some(unit_id);
    
    emit(Event::Move(unit_id, path.into()));

    if unit.health >= dst_tile.capture_points {
        dst_tile.capture_points = 1;
        dst_tile.owner = unit.owner;
        emit(Event::Captured(unit_id, dst_tile_id));
    } else {
        let new_tile_capture_points = dst_tile.capture_points - unit.health;
        dst_tile.capture_points = new_tile_capture_points;
        emit(Event::Capture(unit_id, dst_tile_id, new_tile_capture_points));
    }

    game.update_tiles_and_units(
        [(src_tile_id, src_tile), (dst_tile_id, dst_tile)],
        [(unit_id, unit)])?;

    Ok(())
}

pub fn move_and_deploy(game: &mut Game, unit_id: UnitId, path: &[Position], emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let (src_tile_id, mut src_tile, dst_tile_id, mut dst_tile, mut unit) = try_move(game, unit_id, path)?;

    if !unit.can_deploy() || unit.deployed {
        return Err(ActionError::CannotDeploy);
    }

    unit.moved = true;
    unit.deployed = true;
    src_tile.unit = None;
    dst_tile.unit = Some(unit_id);

    emit(Event::Move(unit_id, path.into()));
    emit(Event::Deploy(unit_id));

    game.update_tiles_and_units(
        [(src_tile_id, src_tile), (dst_tile_id, dst_tile)],
        [(unit_id, unit)])?;

    Ok(())
}

pub fn undeploy(game: &mut Game, unit_id: UnitId, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let mut unit = game.units.get(unit_id).ok_or(ActionError::UnitNotFound)?;
    game.unit_has_turn(&unit)?;
    if !unit.deployed {
        return Err(ActionError::CannotUndeploy);
    }
    
    unit.deployed = false;
    unit.moved = true;

    emit(Event::Undeploy(unit_id));
    
    game.units.update(unit_id, unit)?;

    Ok(())
}

pub fn move_and_load_into(game: &mut Game, unit_id: UnitId, path: &[Position], emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let mut unit = game.units.get(unit_id).ok_or(ActionError::UnitNotFound)?;
    let carrier_id = game.tiles
        .get_at(path.last().ok_or(ActionError::InvalidPath)?)
        .map(|(_, tile)| tile.unit)?
        .ok_or(ActionError::UnitNotFound)?;
    let mut carrier = game.units.get(carrier_id).ok_or(ActionError::UnitNotFound)?;

    game.unit_has_turn(&unit)?;
    game.unit_can_move_path(unit_id, path)?;

    if !carrier.can_carry(&unit) {
        return Err(ActionError::CannotLoad);
    }

    let (src_tile_id, mut src_tile) = game.tiles.get_unit_tile(unit_id)?;

    src_tile.unit = None;
    unit.moved = true;
    carrier.carried.push(unit_id);

    emit(Event::Move(unit_id, path.into()));
    emit(Event::Load(unit_id, carrier_id));

    game.update_tiles_and_units(
        [(src_tile_id, src_tile)],
        [(unit_id, unit), (carrier_id, carrier)])?;
    
    Ok(())
}

pub fn move_and_unload(game: &mut Game, carrier_id: UnitId, path: &[Position], carried_id: UnitId, unload_position: Position, emit: &mut dyn FnMut(Event)) -> ActionResult<()> {
    let (src_tile_id, mut src_tile, dst_tile_id, mut dst_tile, mut carrier) = try_move(game, carrier_id, path)?;
    let mut carried = game.units.get(carried_id).ok_or(ActionError::UnitNotFound)?;
    let (unload_tile_id, mut unload_tile) = game.tiles.get_at(&unload_position)?;

    if !carrier.carried.contains(&carried_id)
        || unload_position.distance_to(path.last().ok_or(ActionError::InvalidPath)?) != 1
        || !carried.can_move_on_terrain(unload_tile.terrain) {
        return Err(ActionError::CannotUnload);
    }

    carrier.moved = true;
    carrier.carried.retain(|&uid| uid != carried_id);
    carried.moved = true;
    src_tile.unit = None;
    dst_tile.unit = Some(carrier_id);
    unload_tile.unit = Some(carried_id);

    emit(Event::Move(carrier_id, path.into()));
    emit(Event::Unload(carrier_id, carried_id, unload_position));

    game.update_tiles_and_units(
        [(src_tile_id, src_tile), (dst_tile_id, dst_tile), (unload_tile_id, unload_tile)],
        [(carrier_id, carrier), (carried_id, carried)])?;
    
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::game::*;
    use crate::model::*;
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");

    fn tiles_from_array(tiles: &[&[Tile]]) -> HashMap<TileId, Tile> {
        let row_size = tiles.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
        tiles.iter().enumerate()
            .map(|(y, row)| row.iter().enumerate().map(move |(x, tile)| (x as i32, y as i32, tile.clone())))
            .flatten()
            .map(|(x, y, tile)| ((x + y * row_size) as usize, Tile { x, y, ..tile }))
            .collect()
    }
    fn path(coords: &[(i32, i32)]) -> Vec<Position> {
        coords.iter().map(From::from).collect()
    }
    #[test]
    fn test_move_and_wait() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let mut game = Game::new(map, &[0, 1]);
        assert!(start(&mut game, &mut |_| ()) == Ok(()));

        let mut events = Vec::new();
        let emit = &mut |e| events.push(e);
        let unit_path = path(&[(0,13), (1,12), (1, 11), (2, 10)]);
        let result = move_and_wait(&mut game, 219, &unit_path, emit);
        assert!(result.is_ok());
        assert_eq!(events, vec![
                   Event::Move(219, unit_path),
                   Event::Wait(219)]);
    }
    #[test]
    fn test_end_turn() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let mut game = Game::new(map, &[0, 1]);
        assert!(start(&mut game, &mut |_| ()) == Ok(()));

        let mut events = Vec::new();
        let emit = &mut |e| events.push(e);
        end_turn(&mut game, emit).unwrap();
        assert_eq!(events, vec![
                   Event::EndTurn(1),
                   Event::StartTurn(2),
                   Event::Funds(2, 600)]);
    }
    #[test]
    fn test_capture() {
        let base = Tile { terrain: model::Terrain::Base, ..Tile::default() };
        let units = [Unit { owner: Some(1), unit_type: UnitType::Infantry, ..Unit::default() }]
            .iter().cloned().enumerate().collect();
        let tiles = tiles_from_array(&[&[Tile { owner: Some(1), ..base }, Tile { owner: Some(2), unit: Some(0usize), ..base}],
                                       &[Tile { owner: Some(1), ..base }, Tile { owner: Some(2), ..base}]]);
        let map = Map { name: "Test".into(), units, tiles, funds: 0 };
        let mut game = Game::new(map, &[1, 2]);
        start(&mut game, &mut |_| ()).unwrap();

        let mut events = Vec::new();
        let emit = &mut |e| events.push(e);
        let unit_path = path(&[(1,0)]);
        move_and_capture(&mut game, 0usize, &unit_path, emit).unwrap();
        end_turn(&mut game, &mut |_| ()).unwrap();
        end_turn(&mut game, &mut |_| ()).unwrap();
        move_and_capture(&mut game, 0usize, &unit_path, emit).unwrap();

        assert_eq!(events, vec![
                   Event::Move(0, unit_path.clone()),
                   Event::Capture(0, 1, 100),
                   Event::Move(0, unit_path),
                   Event::Captured(0, 1),
                   ]);
    }

    #[test]
    fn test_deploy_undeploy() {
        let base = Tile { terrain: model::Terrain::Base, ..Tile::default() };
        let units = [Unit { owner: Some(1), unit_type: UnitType::LightArtillery, ..Unit::default() }]
            .iter().cloned().enumerate().collect();
        let tiles = tiles_from_array(&[&[Tile { owner: Some(1), ..base }, Tile { owner: Some(2), unit: Some(0usize), ..base}],
                                       &[Tile { owner: Some(1), ..base }, Tile { owner: Some(2), ..base}]]);
        let map = Map { name: "Test".into(), units, tiles, funds: 0 };
        let mut game = Game::new(map, &[1, 2]);
        start(&mut game, &mut |_| ()).unwrap();

        let mut events = Vec::new();
        let emit = &mut |e| events.push(e);
        let unit_id = 0;
        let unit_path = path(&[(1, 0), (1, 1)]);

        move_and_deploy(&mut game, unit_id, &unit_path, emit).unwrap();
        end_turn(&mut game, &mut |_| ()).unwrap();
        end_turn(&mut game, &mut |_| ()).unwrap();
        undeploy(&mut game, unit_id, emit).unwrap();

        assert_eq!(events, vec![
                   Event::Move(unit_id, unit_path),
                   Event::Deploy(unit_id),
                   Event::Undeploy(unit_id),
                   ]);

    }

    #[test]
    fn test_load_unload() {
        let plains = Tile { terrain: model::Terrain::Plains, ..Tile::default() };
        let units = [
            Unit { owner: Some(1), unit_type: UnitType::Infantry, ..Unit::default() },
            Unit { owner: Some(1), unit_type: UnitType::APC, ..Unit::default() },
            Unit { owner: Some(2), ..Unit::default() }]
            .iter().cloned().enumerate().collect();
        let tiles = tiles_from_array(&[&[Tile {unit: Some(0), ..plains }, Tile {..plains}],
                                       &[Tile {unit: Some(1), ..plains }, Tile {..plains}],
                                       &[Tile {..plains }, Tile {..plains}],
                                       &[Tile {..plains }, Tile {unit: Some(2), ..plains}],
        ]);
        let map = Map { name: "Test".into(), units, tiles, funds: 0 };
        let mut game = Game::new(map, &[1, 2]);
        start(&mut game, &mut |_| ()).unwrap();

        let unit_id = 0;
        let carrier_id = 1;
        let mut events = Vec::new();
        let emit = &mut |e| events.push(e);

        move_and_load_into(&mut game, unit_id, &path(&[(0,0), (0,1)]), emit).unwrap();
        move_and_unload(&mut game, carrier_id, &path(&[(0,1), (1,1)]), unit_id, Position(1, 0), emit).unwrap();

        assert_eq!(events, vec![
                   Event::Move(unit_id, path(&[(0,0), (0,1)])),
                   Event::Load(unit_id, carrier_id),
                   Event::Move(carrier_id, path(&[(0,1), (1,1)])),
                   Event::Unload(carrier_id, unit_id, Position(1,0)),
                   ]);

    }

    #[test]
    fn test_build() {
        let base = Tile { terrain: model::Terrain::Base, ..Tile::default() };
        let units = [].iter().cloned().enumerate().collect();
        let tiles = tiles_from_array(&[&[Tile { owner: Some(1), ..base }, Tile { owner: Some(2), ..base}],
                                       &[Tile { owner: Some(1), ..base }, Tile { owner: Some(2), ..base}]]);
        let map = Map { name: "Test".into(), units, tiles, funds: 0 };
        let mut game = Game::new(map, &[1, 2]);
        start(&mut game, &mut |_| ()).unwrap();

        let mut events = Vec::new();
        let emit = &mut |e| events.push(e);

        build(&mut game, Position(0, 0), UnitType::Infantry, emit).unwrap();
        build(&mut game, Position(0, 0), UnitType::Bomber, emit).expect_err("Base shouldn't be able to build bombers");
        build(&mut game, Position(0, 0), UnitType::HeavyTank, emit).expect_err("Should not have enough funds");
        end_turn(&mut game, &mut |_| ()).unwrap();
        build(&mut game, Position(1, 0), UnitType::ATInfantry, emit).unwrap();
        assert_eq!(game.in_turn_player().unwrap().funds, 0);

        assert_eq!(events, vec![
                   Event::Build(0, 0, UnitType::Infantry, 100),
                   Event::Build(1, 1, UnitType::ATInfantry, 200),
                   ]);

    }

}
