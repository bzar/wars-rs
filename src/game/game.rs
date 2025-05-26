use crate::game::*;
use crate::model::*;
use crate::util::*;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};

impl Position {
    pub fn distance_to(&self, &Position(x, y): &Position) -> u32 {
        let &Position(sx, sy) = self;
        let result = if y < sy && x > sx {
            (sy - y) + ((x - sx) - (sy - y)).abs()
        } else if x < sx && y > sy {
            (sx - x) + ((y - sy) - (sx - x)).abs()
        } else {
            (sx - x).abs() + (sy - y).abs()
        };

        assert!(result >= 0);
        result as u32
    }
    pub fn adjacent(&self) -> impl Iterator<Item = Self> {
        let Position(x, y) = self;
        [(0, 1), (0, -1), (1, 0), (-1, 0), (-1, 1), (1, -1)]
            .into_iter()
            .map(move |(dx, dy)| Position(x + dx, y + dy))
    }
}

impl From<&(i32, i32)> for Position {
    fn from(&(x, y): &(i32, i32)) -> Self {
        Position(x, y)
    }
}

impl From<GameUpdateError> for ActionError {
    fn from(_: GameUpdateError) -> Self {
        Self::InternalError
    }
}

impl Tiles {
    pub fn rect(&self) -> Option<Rect> {
        self.0.values().fold(None, |x, t| match x {
            Some((x0, y0, x1, y1)) => Some((x0.min(t.x), y0.min(t.y), x1.max(t.x), y1.max(t.y))),
            None => Some((t.x, t.y, t.x, t.y)),
        })
    }
    pub fn get(&self, tile_id: TileId) -> Option<Tile> {
        self.0.get(&tile_id).cloned()
    }
    pub fn get_unit_tile(&self, unit_id: UnitId) -> Option<(TileId, Tile)> {
        self.0
            .iter()
            .filter(|&(_, t)| t.unit == Some(unit_id))
            .only()
            .map(|(&id, t)| (id, t.clone()))
    }
    pub fn get_at(&self, &Position(x, y): &Position) -> ActionResult<(TileId, Tile)> {
        self.0
            .iter()
            .filter(|&(_, t)| t.x == x && t.y == y)
            .only()
            .map(|(&id, t)| (id, t.clone()))
            .ok_or(ActionError::InvalidPath)
    }
    pub fn get_path_tiles(&self, path: &[Position]) -> ActionResult<Vec<Tile>> {
        path.iter()
            .map(|&Position(x, y)| self.iter().filter(|t| t.x == x && t.y == y).only().cloned())
            .collect::<Option<Vec<Tile>>>()
            .ok_or(ActionError::InvalidPath)
    }
    pub fn iter(&self) -> impl Iterator<Item = &Tile> {
        self.0.values()
    }
    pub fn iter_with_ids(&self) -> impl Iterator<Item = (&TileId, &Tile)> {
        self.0.iter()
    }
    pub fn iter_ids(&self) -> impl Iterator<Item = &TileId> {
        self.0.keys()
    }
    pub fn owned_by_player(
        &self,
        player_number: PlayerNumber,
    ) -> impl Iterator<Item = (TileId, &Tile)> {
        self.iter_with_ids()
            .filter(move |(_, tile)| tile.owner == Some(player_number))
            .map(|(tile_id, tile)| (*tile_id, tile))
    }
    pub fn update(&mut self, id: TileId, tile: Tile) -> GameUpdateResult<()> {
        let current = self.0.get_mut(&id).ok_or(GameUpdateError::InvalidTileId)?;
        *current = tile;
        Ok(())
    }
}

impl Units {
    pub fn iter(&self) -> impl Iterator<Item = &Unit> {
        self.0.values()
    }
    pub fn iter_ids(&self) -> impl Iterator<Item = &UnitId> {
        self.0.keys()
    }
    pub fn iter_with_ids(&self) -> impl Iterator<Item = (&UnitId, &Unit)> {
        self.0.iter()
    }
    pub fn get_ref(&self, id: &UnitId) -> Option<&Unit> {
        self.0.get(id)
    }
    pub fn get(&self, unit_id: UnitId) -> Option<Unit> {
        self.0.get(&unit_id).cloned()
    }
    pub fn owned_by_player(
        &self,
        player_number: PlayerNumber,
    ) -> impl Iterator<Item = (UnitId, &Unit)> {
        self.iter_with_ids()
            .filter(move |(_, unit)| unit.owner == Some(player_number))
            .map(|(unit_id, unit)| (*unit_id, unit))
    }
    pub fn update(&mut self, id: UnitId, unit: Unit) -> GameUpdateResult<()> {
        if let Some(current) = self.0.get_mut(&id) {
            *current = unit;
            Ok(())
        } else {
            Err(GameUpdateError::InvalidUnitId)
        }
    }
    pub fn insert(&mut self, unit: Unit) -> UnitId {
        // FIXME: smarter id system
        let unit_id = self.iter_ids().max().map(|x| x + 1).unwrap_or(0);
        self.0.insert(unit_id, unit);
        unit_id
    }
    pub fn remove(&mut self, unit_id: UnitId) -> GameUpdateResult<()> {
        self.0
            .remove(&unit_id)
            .ok_or(GameUpdateError::InvalidUnitId)?;
        Ok(())
    }
}

impl Players {
    pub fn iter(&self) -> impl Iterator<Item = &Player> {
        self.0.iter()
    }
    pub fn update(&mut self, player: Player) -> GameUpdateResult<()> {
        let current = self
            .0
            .iter_mut()
            .filter(|p| p.number == player.number)
            .only()
            .ok_or(GameUpdateError::InvalidPlayerNumber)?;
        *current = player;
        Ok(())
    }
}
impl Game {
    pub fn new(map: Map, players: &[auth::UserId]) -> Game {
        let &max_unit_id = { map.units.iter().map(|(id, _)| id).max().unwrap_or(&0) };

        let players = Players(
            players
                .iter()
                .enumerate()
                .map(|(number, &uid)| Player {
                    user_id: uid,
                    number: number as u32 + 1,
                    funds: map.funds,
                    score: 0,
                    alive: true,
                })
                .collect(),
        );

        Game {
            state: GameState::Pregame,
            units: Units(map.units),
            tiles: Tiles(map.tiles),
            players,
            in_turn_index: 0,
            round_count: 0,
            turn_count: 0,
            next_unit_id: max_unit_id + 1,
        }
    }

    // Mutators

    pub fn set_player_in_turn(&mut self, player_number: PlayerNumber) -> GameUpdateResult<()> {
        self.in_turn_index = self
            .players
            .0
            .iter()
            .enumerate()
            .filter(|(_, p)| p.number == player_number)
            .map(|(i, _)| i)
            .next()
            .ok_or(GameUpdateError::InvalidPlayerNumber)?;
        Ok(())
    }
    pub fn set_state(&mut self, state: GameState) -> GameUpdateResult<()> {
        match (&self.state, &state) {
            (GameState::Pregame, GameState::InProgress) => Ok(()),
            (GameState::InProgress, GameState::Finished) => Ok(()),
            _ => Err(GameUpdateError::InvalidStateTransition),
        }?;

        self.state = state;
        Ok(())
    }
    pub fn update_tiles_and_units(
        &mut self,
        tiles: impl IntoIterator<Item = (TileId, Tile)>,
        units: impl IntoIterator<Item = (UnitId, Unit)>,
    ) -> GameUpdateResult<()> {
        for (tile_id, tile) in tiles.into_iter() {
            self.tiles.update(tile_id, tile)?;
        }
        for (unit_id, unit) in units.into_iter() {
            self.units.update(unit_id, unit)?;
        }
        Ok(())
    }
    pub fn ascii_representation(&self) -> String {
        if let Some((x0, y0, x1, y1)) = self.tiles.rect() {
            let buffer_height = (2 * (x1 - x0) + 4 * (y1 - y0) + 5) as usize;
            let buffer_width = (6 * (x1 - x0) + 9) as usize;

            let mut buffer = vec![vec![' '; buffer_width]; buffer_height];

            fn to_char(x: u32) -> char {
                match x {
                    0..=9 => (b'0' + x as u8) as char,
                    10..=35 => (b'a' + x as u8) as char,
                    36..=61 => (b'A' + x as u8) as char,
                    _ => '?',
                }
            }
            for t in self.tiles.iter() {
                let x = (6 * (t.x - x0) + 4) as usize;
                let y = (2 * (t.x - x0) + 4 * (t.y - y0) + 2) as usize;
                let unit = t.unit.map(|u| self.units.get_ref(&u).unwrap());

                //   432101234
                //-2   ·---·
                //-1  /     \
                // 0 ·  OU   ·
                //+1  \ ot  /
                //+2   ·---·

                let uty = unit.map(|u| to_char(u.unit_type as u32)).unwrap_or(' ');
                let uow = unit.map(|u| u.owner.map(to_char)).flatten().unwrap_or(' ');
                let ter = to_char(t.terrain as u32);
                let tow = t.owner.map(|o| to_char(o.clone())).unwrap_or(' ');
                let tile: [[char; 9]; 5] = [
                    [' ', ' ', '·', '-', '-', '-', '·', ' ', ' '],
                    [' ', '/', ' ', ' ', ' ', ' ', ' ', '\\', ' '],
                    ['·', ' ', ' ', uow, uty, ' ', ' ', ' ', '·'],
                    [' ', '\\', ' ', tow, ter, ' ', ' ', '/', ' '],
                    [' ', ' ', '·', '-', '-', '-', '·', ' ', ' '],
                ];
                tile.iter().enumerate().for_each(|(dy, row)| {
                    let y = y + dy - tile.len() / 2;
                    row.iter().enumerate().for_each(|(dx, &cell)| {
                        let x = x + dx - row.len() / 2;
                        if cell != ' ' {
                            buffer[y][x] = cell;
                        }
                    })
                });
            }

            // Map each row vector into a string, filter empty lines
            // collect as a vector of strings, join with newline
            let map: String = buffer
                .iter()
                .map(|v| v.iter().collect::<String>())
                .filter(|l| l.chars().any(|c| c != ' '))
                .collect::<Vec<_>>()
                .join("\n");

            let terrain_types: std::collections::BTreeSet<Terrain> =
                self.tiles.iter().map(|tile| tile.terrain).collect();
            let terrain_names = terrain_types
                .into_iter()
                .map(|t| format!("{}: {}", to_char(t as u32), terrain(t).name))
                .collect::<Vec<String>>()
                .join(", ");

            let unit_types: std::collections::BTreeSet<UnitType> =
                self.units.iter().map(|unit| unit.unit_type).collect();
            let unit_names = unit_types
                .into_iter()
                .map(|t| format!("{}: {}", to_char(t as u32), unit_type(t).name))
                .collect::<Vec<String>>()
                .join(", ");

            [map, terrain_names, unit_names].join("\n")
        } else {
            "<no data>".to_owned()
        }
    }

    // Selectors

    pub fn unit_can_move_path(&self, unit_id: UnitId, path: &[Position]) -> ActionResult<()> {
        if path.is_empty() {
            return Err(ActionError::InvalidPath);
        }

        let unit = self.units.get(unit_id).ok_or(ActionError::UnitNotFound)?;
        let tiles = self.tiles.get_path_tiles(path)?;

        if tiles[0].unit != Some(unit_id) {
            return Err(ActionError::InvalidPath);
        }

        if path.len() == 1 {
            // Special case: unit stays where it is
            return Ok(());
        }

        let (_, distance) = path
            .iter()
            .fold((None, 0), |(prev, total), pos| match prev {
                Some(prev) => (Some(pos), total + prev.distance_to(pos)),
                None => (Some(pos), 0),
            });

        if distance + 1 != path.len() as u32 {
            return Err(ActionError::InvalidPath);
        }

        let unit_data = model::unit_type(unit.unit_type);
        let movement_type = model::movement(unit_data.movement_type);

        let cost = tiles
            .iter()
            .skip(1) // Source tile cost not included
            .map(|t| (movement_type.terrain_cost_map)(t.terrain))
            .fold(Some(0), |cost, tile_cost| match (cost, tile_cost) {
                (None, _) | (_, None) => None,
                (Some(a), Some(b)) => Some(a + b),
            })
            .ok_or(ActionError::InvalidPath)?;

        if cost > unit_data.movement {
            return Err(ActionError::InvalidPath);
        }

        let path_has_blocking_units = tiles.iter().any(|t| {
            t.unit
                .map(|u_id| {
                    self.units
                        .get_ref(&u_id)
                        .map(|u| u.owner != unit.owner)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        });

        if path_has_blocking_units {
            return Err(ActionError::InvalidPath);
        }

        Ok(())
    }

    pub fn unit_move_options(&self, unit_id: UnitId) -> Option<HashMap<Position, Vec<Position>>> {
        let (_unit_tile_id, unit_tile) = self.tiles.get_unit_tile(unit_id)?;
        let Some((min_x, min_y, max_x, max_y)) = self.tiles.rect() else {
            return None;
        };
        let mut result = HashMap::new();
        let mut queue = VecDeque::from([vec![Position(unit_tile.x, unit_tile.y)]]);

        while let Some(path) = queue.pop_front() {
            let Some(destination) = path.last() else {
                continue;
            };
            if !result.contains_key(destination) && self.unit_can_move_path(unit_id, &path).is_ok()
            {
                for Position(x, y) in destination.adjacent() {
                    if x < min_x
                        || x > max_x
                        || y < min_y
                        || y > max_y
                        || result.contains_key(&Position(x, y))
                    {
                        continue;
                    }
                    let mut next_path = path.clone();
                    next_path.push(Position(x, y));
                    queue.push_back(next_path);
                }

                if self.unit_can_stay_at(unit_id, &destination).is_ok() {
                    result.insert(destination.clone(), path);
                }
            }
        }
        Some(result)
    }
    pub fn unit_can_stay_at(&self, unit_id: UnitId, coords: &Position) -> ActionResult<()> {
        let (_, tile) = self.tiles.get_at(coords)?;

        if tile.unit.is_some() && tile.unit != Some(unit_id) {
            return Err(ActionError::InvalidPath);
        }

        Ok(())
    }
    pub fn unit_can_attack_target(
        &self,
        attacker_id: &UnitId,
        target_id: &UnitId,
        attack_from: &Position,
    ) -> Option<bool> {
        let attacker = self.units.get_ref(attacker_id)?;
        let target = self.units.get_ref(target_id)?;
        let (_, target_tile) = self.tiles.get_unit_tile(*target_id)?;
        let distance = attack_from.distance_to(&Position(target_tile.x, target_tile.y));
        let damage =
            action::calculate_attack_damage(attacker, target, distance, target_tile.terrain);
        Some(damage.is_some())
    }
    pub fn unit_attack_options(&self, unit_id: UnitId, attack_from: &Position) -> HashSet<UnitId> {
        self.units
            .iter_ids()
            .filter(|target_id| {
                self.unit_can_attack_target(&unit_id, target_id, attack_from)
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }
    pub fn in_turn_number(&self) -> Option<PlayerNumber> {
        match self.state {
            GameState::InProgress => Some(self.players.0[self.in_turn_index].number),
            _ => None,
        }
    }
    pub fn in_turn_player(&self) -> Option<Player> {
        if self.state == GameState::InProgress {
            Some(self.players.0.get(self.in_turn_index).unwrap().clone())
        } else {
            None
        }
    }
    pub fn get_player(&self, player_number: PlayerNumber) -> Option<Player> {
        self.players
            .0
            .iter()
            .filter(|p| p.number == player_number)
            .cloned()
            .next()
    }
    pub fn next_player_number(&self) -> Option<PlayerNumber> {
        let next_in_turn_index = (0..self.players.0.len())
            .map(|i| (i + 1 + self.in_turn_index) % self.players.0.len())
            .filter(|i| self.players.0.get(*i).unwrap().alive)
            .next()?;

        Some(self.players.0.get(next_in_turn_index).unwrap().number)
    }
    pub fn players_with_units(&self) -> HashSet<PlayerNumber> {
        self.units.iter().filter_map(|u| u.owner).collect()
    }
    pub fn players_with_build_tiles(&self) -> HashSet<PlayerNumber> {
        self.tiles
            .iter()
            .filter(|t| !terrain(t.terrain).build_classes.is_empty())
            .filter_map(|t| t.owner)
            .collect()
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
    pub fn winner(&self) -> Option<PlayerNumber> {
        // TODO: Add short-circuit for when only one player can do anything
        let mut alive_players = self.players.0.iter().filter(|p| p.alive);
        let maybe_winner = alive_players.next()?;
        if alive_players.next().is_none() {
            Some(maybe_winner.number)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::game::*;
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");

    #[test]
    fn third_party_map_rect() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let game = Game::new(map, &[0, 1]);
        let (x0, y0, x1, y1) = game.tiles.rect().unwrap();
        assert!(
            x0 == 0 && y0 == -7 && x1 == 14 && y1 == 14,
            "({}, {}, {}, {}) != (0, -7, 14, 14)",
            x0,
            y0,
            x1,
            y1
        );
    }
    #[test]
    fn third_party_ascii() {
        let map = Map::from_json(THIRD_PARTY_MAP).unwrap();
        let game = Game::new(map, &[0, 1]);
        println!("{}", game.ascii_representation());
    }

    #[test]
    fn position_distance() {
        assert!(Position(0, 0).distance_to(&Position(0, 1)) == 1);
        assert!(Position(0, 0).distance_to(&Position(0, -1)) == 1);
        assert!(Position(0, 0).distance_to(&Position(1, 0)) == 1);
        assert!(Position(0, 0).distance_to(&Position(1, 0)) == 1);
        assert!(Position(0, 0).distance_to(&Position(1, -1)) == 1);
        assert!(Position(0, 0).distance_to(&Position(-1, 1)) == 1);
    }
}
