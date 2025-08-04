use crate::game::{Action, ActionError, Event, Game, Map, PlayerNumber};
use serde::{Deserialize, Serialize};

pub type GameId = u32;
pub type EventIndex = u32;
pub const VERSION: &str = "0.1";

#[derive(Serialize, Deserialize, Clone)]
pub enum PlayerSlotType {
    Empty,
    Human(Option<String>),
    Bot(String),
}
#[derive(Serialize, Deserialize)]
pub enum ActionMessage {
    NoOp,
    Ping,
    GameAction(GameId, Action),
    SubscribeGame(GameId),
    GetEvents(GameId, EventIndex),
    GetMaps,
    CreateGame(String),
    SetPlayerSlotType(GameId, PlayerNumber, PlayerSlotType),
    StartGame(GameId),
    JoinGame(GameId, PlayerNumber),
    Quit,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum EventMessage {
    ServerVersion(String),
    Pong,
    Maps(Vec<Map>),
    GameState(Game, Vec<(PlayerNumber, PlayerSlotType)>, EventIndex),
    GameCreated(GameId),
    GameJoined(GameId, PlayerNumber, PlayerSlotType),
    GameStarted(GameId),
    GameEvent(GameId, Event),
    GameActionError(GameId, ActionError),
    NoSuchMap,
    NoSuchGame,
    ServerError,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Text parse error")]
    TextError(#[from] serde_json::Error),
    #[error("Binary parse error")]
    BinaryError(#[from] postcard::Error),
}
impl ActionMessage {
    pub fn from_text(text: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(text)?)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
    pub fn as_text(&self) -> Result<String, Error> {
        Ok(serde_json::to_string(self)?)
    }
    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(postcard::to_allocvec(self)?)
    }
}

impl EventMessage {
    pub fn from_text(text: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(text)?)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
    pub fn as_text(&self) -> Result<String, Error> {
        Ok(serde_json::to_string(self)?)
    }
    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(postcard::to_allocvec(self)?)
    }
}

pub fn version_message() -> String {
    EventMessage::ServerVersion(VERSION.to_string())
        .as_text()
        .expect("Could not serialize server version message?")
}
