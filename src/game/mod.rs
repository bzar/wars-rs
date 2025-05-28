use crate::auth;
use crate::model;
use std::collections::HashMap;

pub mod action;
mod game;
mod map;
mod tile;
mod unit;
pub use self::action::*;
pub use self::game::*;
pub use self::map::*;
pub use self::tile::*;
pub use self::unit::*;
pub use model::UnitType;

pub type UnitId = usize;
pub type TileId = usize;
pub type PlayerNumber = u32;
pub type TerrainSubtypeId = u32;
pub type Rect = (i32, i32, i32, i32);
pub type Health = u32;
pub type Credits = u32;
pub type CapturePoints = u32;

#[derive(PartialEq)]
pub enum GameState {
    Pregame = 0,
    InProgress,
    Finished,
}

pub struct Tiles(HashMap<TileId, Tile>);
pub struct Units(HashMap<UnitId, Unit>);
pub struct Players(Vec<Player>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position(pub i32, pub i32);

pub struct Game {
    pub state: GameState,
    pub units: Units,
    pub tiles: Tiles,
    pub players: Players,
    pub in_turn_index: usize,
    pub round_count: u32,
    pub turn_count: u32,
    pub next_unit_id: UnitId,
}

#[derive(Clone)]
pub struct Player {
    pub user_id: auth::UserId,
    pub number: PlayerNumber,
    pub funds: Credits,
    pub score: u32,
    pub alive: bool,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub terrain: model::Terrain,
    pub terrain_subtype_id: TerrainSubtypeId,
    pub owner: Option<PlayerNumber>,
    pub capture_points: CapturePoints,
    pub unit: Option<UnitId>,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub unit_type: model::UnitType,
    pub health: Health,
    pub carried: Vec<UnitId>,
    pub owner: Option<PlayerNumber>,
    pub deployed: bool,
    pub moved: bool,
    pub capturing: bool,
}

pub struct Map {
    pub name: String,
    pub units: HashMap<UnitId, Unit>,
    pub tiles: HashMap<TileId, Tile>,
    pub funds: u32,
}

#[derive(Debug, PartialEq)]
pub enum ActionError {
    InternalError,
    UnitNotFound,
    OwnerNotInTurn,
    UnitAlreadyMoved,
    GameNotInProgress,
    InvalidPath,
    UnitNotOnMap,
    GameAlreadyStarted,
    CannotCapture,
    CannotDeploy,
    CannotUndeploy,
    CannotLoad,
    CannotUnload,
    CannotBuild,
    InsufficientFunds,
    CannotAttack,
    UnitIsDeployed,
    UnitIsNotDeployed,
}

#[derive(Debug, PartialEq)]
pub enum GameUpdateError {
    InvalidStateTransition,
    InvalidPlayerNumber,
    InvalidUnitId,
    InvalidTileId,
}
pub type GameUpdateResult<T> = Result<T, GameUpdateError>;
pub type ActionResult<T> = Result<T, ActionError>;

#[derive(Debug, PartialEq)]
pub enum Event {
    StartTurn(PlayerNumber),
    EndTurn(PlayerNumber),
    Funds(PlayerNumber, Credits),
    UnitRepair(UnitId, Health),
    WinGame(PlayerNumber),
    Surrender(PlayerNumber),
    Move(UnitId, Vec<Position>),
    Wait(UnitId),
    Attack(UnitId, UnitId, Health),
    Counterattack(UnitId, UnitId, Health),
    Destroyed(UnitId, UnitId),
    Deploy(UnitId),
    Undeploy(UnitId),
    Load(UnitId, UnitId),
    Unload(UnitId, UnitId, Position),
    Capture(UnitId, TileId, CapturePoints),
    Captured(UnitId, TileId),
    Build(TileId, UnitId, UnitType, Credits),
    TileCapturePointRegen(TileId, CapturePoints),
}
