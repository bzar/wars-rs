extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

pub mod auth;
pub mod model;
pub mod game;
mod util;

#[cfg(test)]
mod test {
    use crate::game;
    //use auth;
    use crate::model;

    use std::collections::HashMap;

    #[test]
    fn create_game() {
        let base = game::Tile { terrain: model::Terrain::Base, ..game::Tile::default() };
        let map = game::Map {
            name: "Test".into(),
            units: HashMap::new(),
            tiles: [
                (0usize, game::Tile { owner: Some(1), x: 0, ..base }),
                (1usize, game::Tile { owner: Some(2), x: 1, ..base }),
            ].iter().cloned().collect(),
            funds: 42
        };

        let players = vec![1, 2];

        let mut game = game::Game::new(map, &players);
        assert_eq!(game::action::start(&mut game, |_| ()), Ok(()));
        assert_eq!(game.in_turn_player().unwrap().funds, 42 + model::FUNDS_PER_PROPERTY);
    }
}
