use crate::model::{
    DatabasePool, create_game, load_game, load_game_events, save_game, set_game_player,
};
use include_dir::{File, include_dir};
use wars::protocol::{ActionMessage, EventMessage, GameId};

#[derive(Copy, Clone)]
pub enum Recipient {
    Actor,
    Subscribers(GameId),
}
pub type Events = Vec<(Recipient, EventMessage)>;

pub struct State {
    maps: Vec<wars::game::Map>,
}

fn get_all_maps() -> Vec<wars::game::Map> {
    include_dir!("$CARGO_MANIFEST_DIR/../data/maps")
        .entries()
        .into_iter()
        .filter_map(|e| {
            e.as_file()
                .and_then(File::contents_utf8)
                .and_then(|content| wars::game::Map::from_json(content).ok())
        })
        .collect()
}
impl State {
    pub fn new() -> Self {
        Self {
            maps: get_all_maps(),
        }
    }
    pub async fn action(&mut self, action: ActionMessage, pool: &DatabasePool) -> Events {
        match action {
            ActionMessage::NoOp => Events::new(),
            ActionMessage::Ping => Events::from_iter([(Recipient::Actor, EventMessage::Pong)]),
            ActionMessage::GameAction(game_id, action) => {
                let mut events = Events::new();
                let mut new_game_events = Vec::new();
                let (mut game, _players, _last_event_index) = match load_game(game_id, pool).await {
                    Ok(game) => game,
                    Err(_) => {
                        return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                    }
                };
                let mut emit = |event: wars::game::Event| {
                    new_game_events.push(event.clone());
                    events.push((
                        Recipient::Subscribers(game_id),
                        EventMessage::GameEvent(game_id, event),
                    ));
                };
                if let Err(e) = wars::game::action::perform(&mut game, action, &mut emit) {
                    events.push((Recipient::Actor, EventMessage::GameActionError(game_id, e)));
                }

                if let Err(_) = save_game(game_id, game, new_game_events, pool).await {
                    events = Events::from_iter([(Recipient::Actor, EventMessage::ServerError)]);
                }

                events
            }
            ActionMessage::SubscribeGame(game_id) => {
                let Ok((game, players, last_event_index)) = load_game(game_id, pool).await else {
                    return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                };
                Events::from_iter([(
                    Recipient::Actor,
                    EventMessage::GameState(game, players, last_event_index),
                )])
            }
            ActionMessage::CreateGame(map_name) => {
                tracing::info!("CreateGame {map_name}");
                tracing::info!("Maps:");
                self.maps.iter().for_each(|m| tracing::info!("{}", m.name));
                let map = self.maps.iter().find(|m| m.name == map_name);
                if let Some(map) = map {
                    tracing::info!("Found map");
                    let players: Vec<_> = map.player_numbers().iter().map(|pn| (*pn, 0)).collect();
                    let game = wars::game::Game::new(map.clone(), &players);
                    tracing::info!("Creating game");
                    let Ok(game_id) = create_game(game, pool).await else {
                        return Events::from_iter([(Recipient::Actor, EventMessage::ServerError)]);
                    };
                    Events::from_iter([(Recipient::Actor, EventMessage::GameCreated(game_id))])
                } else {
                    tracing::info!("Error");
                    Events::from_iter([(Recipient::Actor, EventMessage::NoSuchMap)])
                }
            }
            ActionMessage::StartGame(_) => {
                unimplemented!("not yet.")
            }
            ActionMessage::JoinGame(_, _) => {
                unimplemented!("not yet.")
            }
            ActionMessage::GetEvents(game_id, since) => {
                let Ok(events) = load_game_events(game_id, since, pool).await else {
                    return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                };
                events
                    .into_iter()
                    .map(|(index, event)| (Recipient::Actor, EventMessage::GameEvent(index, event)))
                    .collect()
            }
            ActionMessage::GetMaps => {
                Events::from_iter([(Recipient::Actor, EventMessage::Maps(self.maps.clone()))])
            }
            ActionMessage::Quit => Events::new(),
            ActionMessage::SetPlayerSlotType(game_id, player_number, slot) => {
                let Ok(_) = set_game_player(game_id, player_number, &slot, pool).await else {
                    return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                };
                Events::from_iter([(
                    Recipient::Subscribers(game_id),
                    EventMessage::GameJoined(game_id, player_number, slot),
                )])
            }
        }
    }
}
