use crate::model::{create_game, load_game, save_game, DatabasePool, EventIndex, GameId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Action {
    NoOp,
    Ping,
    GameAction(GameId, wars::game::Action),
    SubscribeGame(GameId),
    CreateGame(wars::game::Game),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Event {
    Pong,
    GameState(wars::game::Game, EventIndex),
    GameCreated(GameId),
    GameEvent(GameId, wars::game::Event),
    GameActionError(GameId, wars::game::ActionError),
    NoSuchGame,
    ServerError,
}

#[derive(Copy, Clone)]
pub enum Recipient {
    Actor,
    Subscribers,
}
pub type Events = Vec<(Recipient, Event)>;

pub struct State {}

impl State {
    pub async fn action(&mut self, action: Action, pool: &DatabasePool) -> Events {
        match action {
            Action::NoOp => Events::new(),
            Action::Ping => Events::from_iter([(Recipient::Actor, Event::Pong)]),
            Action::GameAction(game_id, action) => {
                let mut events = Events::new();
                let mut new_game_events = Vec::new();
                let (mut game, _last_event_index) = match load_game(game_id, pool).await {
                    Ok(game) => game,
                    Err(_) => return Events::from_iter([(Recipient::Actor, Event::NoSuchGame)]),
                };
                let mut emit = |event: wars::game::Event| {
                    new_game_events.push(event.clone());
                    events.push((Recipient::Subscribers, Event::GameEvent(game_id, event)));
                };
                if let Err(e) = wars::game::action::perform(&mut game, action, &mut emit) {
                    events.push((Recipient::Actor, Event::GameActionError(game_id, e)));
                }

                if let Err(_) = save_game(game_id, game, new_game_events, pool).await {
                    events = Events::from_iter([(Recipient::Actor, Event::ServerError)]);
                }

                events
            }
            Action::SubscribeGame(game_id) => {
                let Ok((game, last_event_index)) = load_game(game_id, pool).await else {
                    return Events::from_iter([(Recipient::Actor, Event::NoSuchGame)]);
                };
                // TODO: Add subscription to game events
                Events::from_iter([(Recipient::Actor, Event::GameState(game, last_event_index))])
            }
            Action::CreateGame(game) => {
                let Ok(game_id) = create_game(game, pool).await else {
                    return Events::from_iter([(Recipient::Actor, Event::NoSuchGame)]);
                };
                // TODO: Add subscription to game events
                Events::from_iter([(Recipient::Actor, Event::GameCreated(game_id))])
            }
        }
    }
}
