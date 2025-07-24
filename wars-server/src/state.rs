use crate::model::{create_game, load_game, load_game_events, save_game, DatabasePool};
use wars::protocol::{ActionMessage, EventMessage, GameId};

#[derive(Copy, Clone)]
pub enum Recipient {
    Actor,
    Subscribers(GameId),
}
pub type Events = Vec<(Recipient, EventMessage)>;

pub struct State {}

impl State {
    pub async fn action(&mut self, action: ActionMessage, pool: &DatabasePool) -> Events {
        match action {
            ActionMessage::NoOp => Events::new(),
            ActionMessage::Ping => Events::from_iter([(Recipient::Actor, EventMessage::Pong)]),
            ActionMessage::GameAction(game_id, action) => {
                                let mut events = Events::new();
                                let mut new_game_events = Vec::new();
                                let (mut game, _last_event_index) = match load_game(game_id, pool).await {
                                    Ok(game) => game,
                                    Err(_) => return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]),
                                };
                                let mut emit = |event: wars::game::Event| {
                                    new_game_events.push(event.clone());
                                    events.push((Recipient::Subscribers(game_id), EventMessage::GameEvent(game_id, event)));
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
                                let Ok((game, last_event_index)) = load_game(game_id, pool).await else {
                                    return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                                };
                                Events::from_iter([(Recipient::Actor, EventMessage::GameState(game, last_event_index))])
                            }
            ActionMessage::CreateGame(game) => {
                                let Ok(game_id) = create_game(game, pool).await else {
                                    return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                                };
                                // TODO: Add subscription to game events
                                Events::from_iter([(Recipient::Actor, EventMessage::GameCreated(game_id))])
                            }
            ActionMessage::GetEvents(game_id, since) => {
                let Ok(events) = load_game_events(game_id, since, pool).await else {
                    return Events::from_iter([(Recipient::Actor, EventMessage::NoSuchGame)]);
                };
                events.into_iter().map(|(index, event)| (Recipient::Actor, EventMessage::GameEvent(index, event))).collect()
            },
            ActionMessage::Quit => Events::new(),
        }
    }
}
