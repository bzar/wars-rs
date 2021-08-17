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
    //use model;

    use std::collections::HashMap;

    #[test]
    fn create_game() {
        let map = game::Map {
            name: "Test".into(),
            units: HashMap::new(),
            tiles: HashMap::new(),
            funds: 42
        };

        let players = vec![1, 2, 4, 8];

        let game = game::Game::new(map, &players);

        assert!(game.players[0].funds == 42);
    }
}
