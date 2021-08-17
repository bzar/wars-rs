use crate::game::*;

pub fn start(game: &mut Game) -> ActionResult<()> {
    match game.state {
        GameState::Pregame => {
            game.state = GameState::InProgress;
            Ok(())
        },
        _ => Err(ActionError::GameAlreadyStarted)
    }
}

pub fn move_and_wait<F>(game: &mut Game, unit_id: UnitId, path: &[Position], mut emit: F) -> ActionResult<usize> 
    where F: FnMut(Event) {
    let mut unit = game.units.get(unit_id)?;

    game.unit_has_turn(&unit)?;
    game.unit_can_move_path(unit_id, path)?;
    game.unit_can_stay_at(unit_id, &path[path.len() - 1])?;

    if path.len() > 1 {
        let (src_tile_id, mut src_tile) = game.tiles.get_unit_tile(unit_id)?;
        let (dst_tile_id, mut dst_tile) = game.tiles.get_at(&path[path.len() - 1])?;

        unit.moved = true;
        src_tile.unit = None;
        dst_tile.unit = Some(unit_id);

        game.tiles.update(src_tile_id, src_tile);
        game.tiles.update(dst_tile_id, dst_tile);
        game.units.update(unit_id, unit);
        emit(Event::Move(unit_id, path.into()));
    }

    emit(Event::Wait(unit_id));
    Ok(path.len())
}

#[cfg(test)]
mod test {
    use crate::game::*;
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");

    #[test]
    fn third_party_first_turn() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let mut game = Game::new(map, &[0, 1]);
        assert!(start(&mut game) == Ok(()));

        let mut events = Vec::new();
        let path: Vec<_> = [(0,13), (1,12), (1, 11), (2, 10)]
            .iter().map(|&(x, y)| Position(x, y)).collect();
        let result = move_and_wait(&mut game, 219, &path, |e| {
            events.push(e);
        });
        assert!(result == Ok(4));
        assert!(events[0] == Event::Move(219, path));
        assert!(events[1] == Event::Wait(219));
    }
}
