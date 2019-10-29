use ::game::*;
use ::util::*;
use std::collections::hash_map;

impl Position {
    pub fn distance_to(&self, &Position(x, y): &Position) -> u32 {
        let &Position(sx, sy) = self;
        let result = if y < sy && x > sx{
            (sy - y) + ((x - sx) - (sy - y)).abs()
        } else if x < sx && y > sy {
            (sx - x) + ((y - sy) - (sx - x)).abs()
        } else {
            (sx - x).abs() + (sy - y).abs()
        };

        assert!(result >= 0);
        result as u32
    }
}
impl Tiles {
    pub fn rect(&self) -> Option<Rect> {
        self.0.values().fold(None, |x, t| {
            match x {
                Some((x0, y0, x1, y1)) => Some((x0.min(t.x), y0.min(t.y), x1.max(t.x), y1.max(t.y))),
                None => Some((t.x, t.y, t.x, t.y))
            }
        })
    }
    pub fn get_unit_tile(&self, unit_id: UnitId) -> ActionResult<(TileId, Tile)> {
        self.0.iter()
            .filter(|&(_, t)| t.unit == Some(unit_id))
            .only().map(|(&id, t)| (id, t.clone())).ok_or(ActionError::UnitNotOnMap)
    }
    pub fn get_at(&self, &Position(x, y): &Position) -> ActionResult<(TileId, Tile)> {
        self.0.iter()
            .filter(|&(_, t)| t.x == x && t.y == y).only()
            .map(|(&id, t)| (id, t.clone()))
            .ok_or(ActionError::InvalidPath)
    }
    pub fn get_path_tiles(&self, path: &[Position]) -> ActionResult<Vec<Tile>> {
        let tiles: Vec<_> = path.iter().map(|&Position(x, y)| {
            self.iter().filter(|t| t.x == x && t.y == y).only().map(|t| t.clone())
        }).collect();

        if tiles.iter().any(|t| t.is_none()) {
            return Err(ActionError::InvalidPath);
        }

        Ok(tiles.into_iter().map(|t| t.unwrap()).collect())
    }
    pub fn iter(&self) -> hash_map::Values<TileId, Tile> {
        self.0.values()
    }
    pub fn update(&mut self, id: TileId, tile: Tile) {
        if let Some(current) = self.0.get_mut(&id) {
            *current = tile;
        }
    }
}

impl Units {
    pub fn iter(&self) -> hash_map::Values<UnitId, Unit> {
        self.0.values()
    }
    pub fn get_ref(&self, id: &UnitId) -> Option<&Unit> {
        self.0.get(id)
    }
    pub fn get(&self, unit_id: UnitId) -> ActionResult<Unit> {
        self.0.get(&unit_id).map(|u| u.clone()).ok_or(ActionError::UnitNotFound)
    }
    pub fn update(&mut self, id: UnitId, tile: Unit) {
        if let Some(current) = self.0.get_mut(&id) {
            *current = tile;
        }
    }
}
impl Game {
    pub fn new(map: Map, players: &[auth::UserId]) -> Game {
        let &max_unit_id = {
            map.units.iter().map(|(id, _)| id).max().unwrap_or(&0)
        };

        let players = {
            players.iter().enumerate().map(|(number, &uid)| Player {
                user_id: uid,
                number: number as u32 + 1,
                funds: map.funds,
                score: 0 }).collect()
        };

        Game {
            state: GameState::Pregame,
            units: Units(map.units),
            tiles: Tiles(map.tiles),
            players,
            in_turn_index: 0,
            round_count: 0,
            turn_count: 0,
            next_unit_id: max_unit_id + 1
        }
    }

    pub fn ascii_representation(&self) -> String {
        if let Some((x0, y0, x1, y1)) = self.tiles.rect() {
            let buffer_height = (2 * (x1 - x0) + 4 * (y1 - y0) + 5) as usize;
            let buffer_width = (6 * (x1 - x0) + 9) as usize;

            let mut buffer = vec![vec![' '; buffer_width]; buffer_height];

            fn to_char(x: u32) -> char {
                match x {
                    0...9 => ('0' as u8 + x as u8) as char,
                    10...35 => ('a' as u8 + x as u8) as char,
                    36...61 => ('A' as u8 + x as u8) as char,
                    _ => '?'
                }
            }
            for t in self.tiles.iter() {
                let x = (6 * (t.x - x0) + 4) as usize;
                let y = (2 * (t.x - x0) + 4 * (t.y - y0) + 2) as usize;
                let unit = t.unit.map(|u| self.units.get_ref(&u).unwrap());
                buffer[y][x] = unit.map(|u| to_char(u.unit_type as u32)).unwrap_or(' ');
                buffer[y-0][x-1] = match unit {
                    Some(&Unit { owner: Some(owner), .. }) => to_char(owner),
                    _ => ' '
                };
                buffer[y+1][x-1] = t.owner.as_ref().map(|&o| to_char(o)).unwrap_or(' ');

                buffer[y+1][x-0] = to_char(t.terrain as u32);
                buffer[y-2][x-2] = '·';
                buffer[y+2][x-2] = '·';
                buffer[y-2][x+2] = '·';
                buffer[y+2][x+2] = '·';
                buffer[y-1][x-3] = '/';
                buffer[y+1][x-3] = '\\';
                buffer[y-1][x+3] = '\\';
                buffer[y+1][x+3] = '/';
                buffer[y-0][x-4] = '·';
                buffer[y-0][x+4] = '·';
                buffer[y-2][x-0] = '-';
                buffer[y+2][x-0] = '-';
            }

            // Map each row vector into a string, filter empty lines
            // collect as a vector of strings, join with newline
            buffer.iter().map(|v| v.iter().collect::<String>())
                .filter(|l| l.chars().any(|c| c != ' '))
                .collect::<Vec<_>>().join("\n")
        } else {
            "<no data>".to_owned()
        }
    }

    pub fn unit_can_move_path(&self, unit_id: UnitId, path: &[Position]) -> ActionResult<()> {
        if path.is_empty() {
            return Err(ActionError::InvalidPath);
        }

        let unit = self.units.get(unit_id)?;
        let tiles = self.tiles.get_path_tiles(path)?;

        if tiles[0].unit != Some(unit_id) {
            return Err(ActionError::InvalidPath);
        }

        if path.len() == 1 {
            // Special case: unit stays where it is
            return Ok(());
        }

        let (_, distance) = path.iter().fold((None, 0), |(prev, total), pos| match prev {
            Some(prev) => (Some(pos), total + prev.distance_to(pos)),
            None => (Some(pos), 0)
        });

        if distance + 1 != path.len() as u32 {
            return Err(ActionError::InvalidPath);
        }

        let unit_data = model::unit_type(unit.unit_type);
        let movement_type = model::movement(unit_data.movement_type);

        let cost = tiles.iter().skip(1) // Source tile cost not included
            .map(|t| (movement_type.terrain_cost_map)(t.terrain))
            .fold(Some(0), |cost, tile_cost| match (cost, tile_cost) {
                (None, _) | (_, None) => None,
                (Some(a), Some(b)) => Some(a + b)
            }).ok_or(ActionError::InvalidPath)?;

        if cost > unit_data.movement {
            return Err(ActionError::InvalidPath);
        }

        let path_has_blocking_units = tiles.iter()
            .any(|t| t.unit
                 .map(|u_id| self.units.get_ref(&u_id)
                      .map(|u| u.owner != unit.owner).unwrap_or(false))
                 .unwrap_or(false));

        if path_has_blocking_units {
            return Err(ActionError::InvalidPath);
        }

        Ok(()) 
    }

    pub fn unit_can_stay_at(&self, unit_id: UnitId, coords: &Position) -> ActionResult<()> {
        let (_, tile) = self.tiles.get_at(coords)?;

        if tile.unit.is_some() && tile.unit != Some(unit_id) {
            return Err(ActionError::InvalidPath);
        }

        Ok(())
    }
    pub fn in_turn_number(&self) -> Option<PlayerNumber> {
        match self.state {
            GameState::InProgress => Some(self.players[self.in_turn_index].number),
            _ => None
        }
    }
    pub fn unit_has_turn(&self, unit: &Unit) -> ActionResult<()> {
        if unit.moved {
            return Err(ActionError::UnitAlreadyMoved);
        }

        if unit.owner != self.in_turn_number() {
            return Err(ActionError::OwnerNotInTurn);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use ::game::*;
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");

    #[test]
    fn third_party_map_rect() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let game = Game::new(map, &[0, 1]);
        let (x0, y0, x1, y1) = game.tiles.rect().unwrap();
        assert!(x0 == 0 && y0 == -7 && x1 == 14 && y1 == 14,
                "({}, {}, {}, {}) != (0, -7, 14, 14)", x0, y0, x1, y1);
    }
    #[test]
    fn third_party_ascii() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let game = Game::new(map, &[0, 1]);
        println!("{}", game.ascii_representation());
    }

    #[test]
    fn position_distance() {
        assert!(Position(0,0).distance_to(&Position(0,1)) == 1);
        assert!(Position(0,0).distance_to(&Position(0,-1)) == 1);
        assert!(Position(0,0).distance_to(&Position(1,0)) == 1);
        assert!(Position(0,0).distance_to(&Position(1,0)) == 1);
        assert!(Position(0,0).distance_to(&Position(1,-1)) == 1);
        assert!(Position(0,0).distance_to(&Position(-1,1)) == 1);
    }
}
