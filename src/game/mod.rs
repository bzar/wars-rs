use std::collections::HashMap;
use crate::model;
use crate::auth;

mod game;
mod unit;
mod tile;
mod map;
mod action;
pub use self::game::*;
pub use self::unit::*;
pub use self::tile::*;
pub use self::map::*;
pub use self::action::*;

pub type UnitId = usize;
pub type TileId = usize;
pub type PlayerNumber = u32;
pub type TerrainSubtypeId = u32;
pub type Rect = (i32, i32, i32, i32);

pub enum GameState { Pregame = 0, InProgress, Finished }

pub struct Tiles(HashMap<TileId, Tile>);
pub struct Units(HashMap<UnitId, Unit>);
#[derive(Clone,Debug,PartialEq)]
pub struct Position(i32, i32);

pub struct Game {
    pub state: GameState,
    pub units: Units,
    pub tiles: Tiles,
    pub players: Vec<Player>,
    pub in_turn_index: usize,
    pub round_count: u32,
    pub turn_count: u32,
    pub next_unit_id: UnitId
}

#[derive(Clone)]
pub struct Player {
    pub user_id: auth::UserId,
    pub number: PlayerNumber,
    pub funds: u32,
    pub score: u32
}

#[derive(Clone)]
pub struct Tile {
    pub terrain: model::Terrain,
    pub terrain_subtype_id: TerrainSubtypeId,
    pub owner: Option<PlayerNumber>,
    pub capture_points: u32,
    pub unit: Option<UnitId>,
    pub x: i32,
    pub y: i32
}

#[derive(Clone,Debug)]
pub struct Unit {
    pub unit_type: model::UnitType,
    pub health: u32,
    pub carried: Vec<UnitId>,
    pub owner: Option<PlayerNumber>,
    pub deployed: bool,
    pub moved: bool,
    pub capturing: bool
}

pub struct Map {
    pub name: String,
    pub units: HashMap<UnitId, Unit>,
    pub tiles: HashMap<TileId, Tile>,
    pub funds: u32
}

#[derive(Debug,PartialEq)]
pub enum ActionError {
    UnitNotFound, OwnerNotInTurn, UnitAlreadyMoved, GameNotInProgress, InvalidPath,
    UnitNotOnMap, GameAlreadyStarted
}

pub type ActionResult<T> = Result<T, ActionError>;

#[derive(Debug,PartialEq)]
pub enum Event {
    Move(UnitId, Vec<Position>), Wait(UnitId)
}
