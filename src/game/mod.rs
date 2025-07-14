use crate::auth;
use crate::model;
use std::collections::HashMap;

pub mod action;
mod game;
mod map;
mod tile;
mod unit;
pub use model::UnitType;

pub type UnitId = usize;
pub type TileId = usize;
pub type PlayerNumber = u32;
pub type TerrainSubtypeId = u32;
pub type Rect = (i32, i32, i32, i32);
pub type Health = u32;
pub type Credits = u32;
pub type CapturePoints = u32;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum GameState {
    Pregame = 0,
    InProgress,
    Finished,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Tiles(HashMap<TileId, Tile>);
#[derive(Serialize, Deserialize, Clone)]
pub struct Units(HashMap<UnitId, Unit>);
#[derive(Serialize, Deserialize, Clone)]
pub struct Players(Vec<Player>);

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position(pub i32, pub i32);

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub user_id: auth::UserId,
    pub number: PlayerNumber,
    pub funds: Credits,
    pub score: u32,
    pub alive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tile {
    pub terrain: model::Terrain,
    pub terrain_subtype_id: TerrainSubtypeId,
    pub owner: Option<PlayerNumber>,
    pub capture_points: CapturePoints,
    pub unit: Option<UnitId>,
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Unit {
    pub unit_type: model::UnitType,
    pub health: Health,
    pub carried: Vec<UnitId>,
    pub owner: Option<PlayerNumber>,
    pub deployed: bool,
    pub moved: bool,
    pub capturing: bool,
}

#[derive(Clone)]
pub struct Map {
    pub name: String,
    pub units: HashMap<UnitId, Unit>,
    pub tiles: HashMap<TileId, Tile>,
    pub funds: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Action {
    StartGame,
    EndTurn,
    Surrender,
    Build(Position, UnitType),
    MoveAndWait(UnitId, Vec<Position>),
    MoveAndAttack(UnitId, Vec<Position>, UnitId),
    MoveAndCapture(UnitId, Vec<Position>),
    MoveAndDeploy(UnitId, Vec<Position>),
    Undeploy(UnitId),
    MoveAndLoadInto(UnitId, Vec<Position>),
    MoveAndUnload(UnitId, Vec<Position>, UnitId, Position),
}
#[derive(Serialize, Deserialize, thiserror::Error, Debug, PartialEq, Clone, Copy)]
pub enum ActionError {
    #[error("Internal error")]
    InternalError,
    #[error("Tile not found")]
    TileNotFound,
    #[error("Unit not found")]
    UnitNotFound,
    #[error("Owner is not in turn")]
    OwnerNotInTurn,
    #[error("Unit has already moved")]
    UnitAlreadyMoved,
    #[error("Game is not in progress")]
    GameNotInProgress,
    #[error("Invalid path")]
    InvalidPath,
    #[error("Unit is not on map")]
    UnitNotOnMap,
    #[error("Game has already started")]
    GameAlreadyStarted,
    #[error("Cannot capture")]
    CannotCapture,
    #[error("Cannot deploy")]
    CannotDeploy,
    #[error("Cannot undeploy")]
    CannotUndeploy,
    #[error("Cannot load")]
    CannotLoad,
    #[error("Cannot unload")]
    CannotUnload,
    #[error("Cannot build")]
    CannotBuild,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Cannot attack")]
    CannotAttack,
    #[error("Unit is deployed")]
    UnitIsDeployed,
    #[error("Unit is not deployed")]
    UnitIsNotDeployed,
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GameUpdateError {
    #[error("Invalid state transition")]
    InvalidStateTransition,
    #[error("Invalid player number")]
    InvalidPlayerNumber,
    #[error("Invalid unit ID")]
    InvalidUnitId,
    #[error("Invalid tile ID")]
    InvalidTileId,
}
pub type GameUpdateResult<T> = Result<T, GameUpdateError>;
pub type ActionResult<T> = Result<T, ActionError>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
    Captured(UnitId, TileId, Option<PlayerNumber>),
    Build(TileId, UnitId, UnitType, Credits),
    TileCapturePointRegen(TileId, CapturePoints),
}
