use bevy::prelude::*;
use std::collections::VecDeque;
use wars::{
    game::{ActionError, Game, Map, PlayerNumber},
    protocol::{ActionMessage, EventIndex, EventMessage, GameId, PlayerSlotType},
};

use crate::bevy_nfws::NfwsHandle;
pub struct ConnectionPlugin;
impl Plugin for ConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, process_messages);
    }
}

pub enum ConnectionEvent {
    Connected,
    Maps(Vec<Map>),
    GameState(Game, Vec<(PlayerNumber, PlayerSlotType)>, EventIndex),
    GameCreated(GameId),
    GameJoined(GameId, PlayerNumber, PlayerSlotType),
    GameStarted(GameId),
    GameEvent(GameId, wars::game::Event),
    GameActionError(GameId, ActionError),
    Disconnected,
}

#[derive(Component, Default)]
pub struct Connection {
    events: VecDeque<ConnectionEvent>,
    actions: VecDeque<ActionMessage>,
    server_url: Option<String>,
    up: bool,
}

impl TryFrom<wars::protocol::EventMessage> for ConnectionEvent {
    type Error = ();

    fn try_from(value: wars::protocol::EventMessage) -> std::result::Result<Self, Self::Error> {
        match value {
            wars::protocol::EventMessage::Maps(maps) => Ok(Self::Maps(maps)),
            wars::protocol::EventMessage::GameState(game, items, players) => {
                Ok(Self::GameState(game, items, players))
            }
            wars::protocol::EventMessage::GameCreated(game_id) => Ok(Self::GameCreated(game_id)),
            wars::protocol::EventMessage::GameJoined(game_id, player_number, player_slot_type) => {
                Ok(Self::GameJoined(game_id, player_number, player_slot_type))
            }
            wars::protocol::EventMessage::GameStarted(game_id) => Ok(Self::GameStarted(game_id)),
            wars::protocol::EventMessage::GameEvent(game_id, event) => {
                Ok(Self::GameEvent(game_id, event))
            }
            wars::protocol::EventMessage::GameActionError(game_id, action_error) => {
                Ok(Self::GameActionError(game_id, action_error))
            }
            _ => Err(()),
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Connection::default());
}
fn process_messages(
    mut commands: Commands,
    mut connection: Single<&mut Connection>,
    mut handles: Query<(Entity, &mut NfwsHandle)>,
) {
    if connection.server_url.is_none() {
        for (entity, _) in handles.iter() {
            commands.entity(entity).despawn();
        }
    }

    let Ok((_, mut handle)) = handles.single_mut() else {
        connection.up = false;
        if handles.iter().count() > 1 {
            panic!("Multiple server connections!");
        }

        if let Some(server_url) = connection.server_url.as_ref() {
            commands.spawn(NfwsHandle::new(server_url.clone()));
        }
        return;
    };

    connection.up = true;

    while let Some(action) = connection.actions.pop_front() {
        handle.send_text(action.as_text().unwrap());
    }

    let mut handle_message = |event_message| match event_message {
        wars::protocol::EventMessage::ServerVersion(version) => {
            if version == wars::protocol::VERSION {
                connection.events.push_back(ConnectionEvent::Connected);
            } else {
                error!("Server protocol version mismatch!");
                connection.disconnect();
            }
        }
        wars::protocol::EventMessage::Pong => (),
        wars::protocol::EventMessage::NoSuchMap => {
            warn!("No such map");
        }
        wars::protocol::EventMessage::NoSuchGame => {
            warn!("No such game");
        }
        wars::protocol::EventMessage::ServerError => {
            error!("Server error!");
            connection.disconnect();
        }
        other => {
            let Ok(event) = ConnectionEvent::try_from(other) else {
                error!("Unidentified message type");
                connection.disconnect();
                return;
            };
            connection.events.push_back(event);
        }
    };
    match handle.next_event() {
        crate::bevy_nfws::NfwsPollResult::Closed => connection.disconnect(),
        crate::bevy_nfws::NfwsPollResult::Empty => (),
        crate::bevy_nfws::NfwsPollResult::Event(nfws_event) => match nfws_event {
            crate::bevy_nfws::NfwsEvent::Connecting => info!("Connecting..."),
            crate::bevy_nfws::NfwsEvent::Connected => info!("Connected!"),
            crate::bevy_nfws::NfwsEvent::TextMessage(text) => {
                debug!("TextMessage: {}", text);
                match wars::protocol::EventMessage::from_text(&text) {
                    Ok(event_message) => handle_message(event_message),
                    Err(error) => {
                        error!("Connection error: {}", error);
                        connection.disconnect();
                    }
                }
            }
            crate::bevy_nfws::NfwsEvent::BinaryMessage(bytes) => {
                debug!("BinaryMessage: <binary data>");
                match wars::protocol::EventMessage::from_bytes(&bytes) {
                    Ok(event_message) => handle_message(event_message),
                    Err(error) => {
                        error!("Connection error: {}", error);
                        connection.disconnect();
                    }
                }
            }
            crate::bevy_nfws::NfwsEvent::Error(nfws_err) => {
                error!("Socket error: {nfws_err:?}");
                connection.disconnect();
            }
            crate::bevy_nfws::NfwsEvent::Closed(reason) => {
                error!("Disconnected: {reason:?}");
                connection.disconnect();
            }
        },
    };
}

impl Connection {
    pub fn send(&mut self, action_message: ActionMessage) {
        self.actions.push_back(action_message);
    }
    pub fn recv(&mut self) -> Option<ConnectionEvent> {
        self.events.pop_front()
    }
    pub fn recv_all(&mut self) -> impl Iterator<Item = ConnectionEvent> {
        let mut events = VecDeque::new();
        std::mem::swap(&mut self.events, &mut events);
        events.into_iter()
    }
    pub fn is_up(&self) -> bool {
        self.up
    }
    pub fn connect(&mut self, server_url: String) {
        self.server_url = Some(server_url);
    }
    pub fn disconnect(&mut self) {
        self.events.push_back(ConnectionEvent::Disconnected);
        self.server_url = None;
    }
}
