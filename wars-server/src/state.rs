use crate::model::{DatabasePool, GameId, create_game, load_game, save_game};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Action {
    NoOp,
    Ping,
    GameAction(GameId, wars::game::Action),
    SubscribeGame(GameId),
    CreateGame(wars::game::Game),
}

#[derive(Serialize, Deserialize)]
pub enum Event {
    Pong,
    GameState(wars::game::Game),
    GameCreated(GameId),
    GameEvent(GameId, wars::game::Event),
    GameActionError(GameId, wars::game::ActionError),
    NoSuchGame,
}

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
                let mut game = match load_game(game_id, pool).await {
                    Ok(game) => game,
                    Err(_) => return Events::from_iter([(Recipient::Actor, Event::NoSuchGame)]),
                };
                let mut emit = |event| {
                    events.push((Recipient::Subscribers, Event::GameEvent(game_id, event)));
                };
                if let Err(e) = wars::game::action::perform(&mut game, action, &mut emit) {
                    events.push((Recipient::Actor, Event::GameActionError(game_id, e)));
                }

                save_game(game_id, game, pool).await;

                events
            }
            Action::SubscribeGame(game_id) => {
                let mut game = match load_game(game_id, pool).await {
                    Ok(game) => game,
                    Err(_) => return Events::from_iter([(Recipient::Actor, Event::NoSuchGame)]),
                };
                // TODO: Add subscription to game events
                Events::from_iter([(Recipient::Actor, Event::GameState(game))])
            }
            Action::CreateGame(game) => {
                let mut game_id = match create_game(game, pool).await {
                    Ok(game_id) => game_id,
                    Err(_) => return Events::from_iter([(Recipient::Actor, Event::NoSuchGame)]),
                };
                // TODO: Add subscription to game events
                Events::from_iter([(Recipient::Actor, Event::GameCreated(game_id))])
            }
        }
    }
}
