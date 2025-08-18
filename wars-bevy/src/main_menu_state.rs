use crate::{
    AppState,
    connection::{Connection, ConnectionEvent},
    resources::{Game, Player},
};
use bevy::prelude::*;
use include_dir::{File, include_dir};
use wars::protocol::{GameId, PlayerSlotType};

pub struct MainMenuStatePlugin;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PlayerType {
    None,
    Human,
    Bot,
}

impl Plugin for MainMenuStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_egui::EguiPlugin::default())
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                (
                    main_menu_system.run_if(in_state(AppState::MainMenu)),
                    setup_local_menu_system.run_if(in_state(AppState::SetupLocal)),
                    connect_to_server_menu_system.run_if(in_state(AppState::ConnectToServer)),
                    select_game_menu_system.run_if(in_state(AppState::SelectGame)),
                    host_select_map_menu_system.run_if(in_state(AppState::HostSelectMap)),
                    host_pregame_menu_system.run_if(in_state(AppState::HostPreGame)),
                ),
            );
    }
}

fn main_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            if ui.button("Local game").clicked() {
                next_state.set(AppState::SetupLocal);
            }
            if ui.button("Connect to server").clicked() {
                next_state.set(AppState::ConnectToServer);
            }
            if ui.button("Quit").clicked() {
                exit.write(AppExit::Success);
            }
        });
    });
}

fn connect_to_server_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut address: Local<Option<String>>,
    mut connection: Single<&mut Connection>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if connection.is_up() {
        if let Some(ConnectionEvent::Connected) = connection.recv() {
            next_state.set(AppState::SelectGame);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Connecting...");
        });
    } else {
        let address = address.get_or_insert("ws://localhost:3000/ws".to_owned());
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.text_edit_singleline(address);
            if ui.button("Connect").clicked() {
                connection.connect(address.clone());
            }
            if ui.button("Back").clicked() {
                next_state.set(AppState::MainMenu);
            }
        });
    }
}

fn select_game_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut connection: Single<&mut Connection>,
) {
    if !connection.is_up() {
        next_state.set(AppState::MainMenu);
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // TODO: Fetch game list, show it
    egui::CentralPanel::default().show(ctx, |ui| {
        if ui.button("Host a new game").clicked() {
            next_state.set(AppState::HostSelectMap);
        }
        if ui.button("Back").clicked() {
            connection.disconnect();
            next_state.set(AppState::ConnectToServer);
        }
        ui.label("TODO: Joinable games here");
    });
}

fn host_select_map_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut connection: Single<&mut Connection>,
    mut maps: Local<Option<Vec<wars::game::Map>>>,
    mut map_index: Local<usize>,
    mut pregame_state: Local<HostPregameState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if !connection.is_up() {
        next_state.set(AppState::MainMenu);
        return;
    }

    if maps.is_none() {
        connection.send(wars::protocol::ActionMessage::GetMaps);
        *maps = Some(Vec::new());
    }

    if let Some(ConnectionEvent::Maps(ms)) = connection.recv() {
        *maps = Some(ms);
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(ref maps) = *maps
            && !maps.is_empty()
        {
            if *map_index >= maps.len() {
                *map_index = 0
            }
            let map = &maps[*map_index];
            egui::ComboBox::from_label("Map")
                .selected_text(&map.name)
                .show_ui(ui, |ui| {
                    for (i, map) in maps.iter().enumerate() {
                        ui.selectable_value(&mut *map_index, i, &map.name);
                    }
                });
        } else {
            ui.label("Loading maps...");
        }

        // TODO: Fetch game list, show it
        let map = maps.as_ref().and_then(|maps| maps.get(*map_index));
        if ui.button("Create game").clicked()
            && let Some(map) = map
        {
            connection.send(wars::protocol::ActionMessage::CreateGame(map.name.clone()));
            *pregame_state = HostPregameState::CreatingGame;
            next_state.set(AppState::HostPreGame);
        }
        if ui.button("Back").clicked() {
            next_state.set(AppState::SelectGame);
        }
    });
}

#[derive(Default)]
enum HostPregameState {
    #[default]
    CreatingGame,
    LoadingGame(GameId),
    PreparingGame(
        GameId,
        wars::game::Game,
        Vec<(wars::game::PlayerNumber, PlayerType, String)>,
    ),
}
fn host_pregame_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut connection: Single<&mut Connection>,
    mut state: Local<HostPregameState>,
    mut game_state: ResMut<Game>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if !connection.is_up() {
        next_state.set(AppState::MainMenu);
        return;
    };

    match (&mut *state, connection.recv()) {
        (HostPregameState::CreatingGame, Some(ConnectionEvent::GameCreated(game_id))) => {
            info!("Game created with id {game_id}");
            *state = HostPregameState::LoadingGame(game_id);
            connection.send(wars::protocol::ActionMessage::SubscribeGame(game_id));
        }
        (
            HostPregameState::LoadingGame(game_id),
            Some(ConnectionEvent::GameState(game, players, last_event)),
        ) => {
            info!("Received game state, last event: {last_event}");
            let players = players
                .into_iter()
                .map(|slot| match slot {
                    (pn, PlayerSlotType::Empty) => (pn, PlayerType::None, String::new()),
                    (pn, PlayerSlotType::Human(name)) => {
                        (pn, PlayerType::Human, name.unwrap_or(String::new()))
                    }
                    (pn, PlayerSlotType::Bot(name)) => (pn, PlayerType::Bot, name.clone()),
                })
                .collect();
            *state = HostPregameState::PreparingGame(*game_id, game, players);
        }
        (
            HostPregameState::PreparingGame(game_id, game, players),
            Some(ConnectionEvent::GameJoined(event_game_id, player_number, slot)),
        ) => {
            if event_game_id == *game_id {
                if let Some((player, name)) = players
                    .iter_mut()
                    .find_map(|(pn, player, name)| (*pn == player_number).then_some((player, name)))
                {
                    match slot {
                        PlayerSlotType::Empty => {
                            *player = PlayerType::None;
                            name.clear();
                        }
                        PlayerSlotType::Human(n) => {
                            *player = PlayerType::Human;
                            *name = n.unwrap_or_default();
                        }
                        PlayerSlotType::Bot(n) => {
                            *player = PlayerType::Bot;
                            *name = n;
                        }
                    }
                }
            }
        }
        (
            HostPregameState::PreparingGame(game_id, game, players),
            Some(ConnectionEvent::GameStarted(event_game_id)),
        ) => {
            // TODO: Game should be started here?
            if event_game_id == *game_id {
                *game_state = Game::InGame(
                    game.clone(),
                    players
                        .iter()
                        .filter_map(
                            |(player_number, player_type, _player_name)| match player_type {
                                // TODO: Set correctly as remote or local human
                                PlayerType::None => None,
                                PlayerType::Human => Some((*player_number, Player::Human)),
                                PlayerType::Bot => Some((*player_number, Player::Remote)),
                            },
                        )
                        .collect(),
                    Some(*game_id),
                );
                next_state.set(AppState::InGame);
            }
        }
        (_, Some(connection_event)) => {
            info!("Unexpected event");
        }
        _ => (),
    }

    match &mut *state {
        HostPregameState::CreatingGame => {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Creating game...");
            });
        }
        HostPregameState::LoadingGame(game_id) => {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(format!("Loading game #{game_id}"));
            });
        }
        HostPregameState::PreparingGame(game_id, game, players) => {
            egui::CentralPanel::default().show(ctx, |ui| {
                players
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, (pn, slot, name))| {
                        let previous = slot.clone();
                        egui::ComboBox::new(pn.clone(), name.as_str())
                            .selected_text(match slot {
                                PlayerType::Human => "Human",
                                PlayerType::Bot => "Bot",
                                PlayerType::None => "None",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(slot, PlayerType::Human, "Human");
                                ui.selectable_value(slot, PlayerType::Bot, "Bot");
                                ui.selectable_value(slot, PlayerType::None, "None");
                            });

                        if *slot != previous {
                            let slot_type = match slot {
                                PlayerType::None => PlayerSlotType::Empty,
                                PlayerType::Human => PlayerSlotType::Human(None),
                                PlayerType::Bot => PlayerSlotType::Bot(String::new()),
                            };
                            connection.send(wars::protocol::ActionMessage::SetPlayerSlotType(
                                *game_id, *pn, slot_type,
                            ));
                        }

                        if *slot == PlayerType::Human {
                            if name.is_empty() {
                                if ui.button("Join").clicked() {
                                    connection.send(wars::protocol::ActionMessage::JoinGame(
                                        *game_id, *pn,
                                    ));
                                }
                            }
                        }
                    });
                if ui.button("Start game").clicked() {
                    connection.send(wars::protocol::ActionMessage::StartGame(*game_id));
                }
                if ui.button("Back").clicked() {
                    next_state.set(AppState::HostSelectMap);
                }
            });
        }
    }
}
fn setup_local_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut game: ResMut<Game>,
    mut player_types: Local<Vec<(wars::game::PlayerNumber, PlayerType)>>,
    mut maps: Local<Option<Vec<wars::game::Map>>>,
    mut map_index: Local<usize>,
) {
    let maps = maps.get_or_insert_with(|| {
        include_dir!("$CARGO_MANIFEST_DIR/../data/maps")
            .entries()
            .into_iter()
            .filter_map(|e| {
                e.as_file()
                    .and_then(File::contents_utf8)
                    .and_then(|content| wars::game::Map::from_json(content).ok())
            })
            .collect()
    });

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::CentralPanel::default().show(ctx, |ui| {
        let previous_map_index = *map_index;
        ui.vertical_centered(|ui| {
            let map = &maps[*map_index];
            egui::ComboBox::from_label("Map")
                .selected_text(&map.name)
                .show_ui(ui, |ui| {
                    for (i, map) in maps.iter().enumerate() {
                        ui.selectable_value(&mut *map_index, i, &map.name);
                    }
                });

            let map = &maps[*map_index];
            if previous_map_index != *map_index || player_types.is_empty() {
                *player_types = map
                    .player_numbers()
                    .iter()
                    .map(|pn| (*pn, PlayerType::Human))
                    .collect();
                player_types.sort_by_key(|(pn, _)| *pn);
            }

            player_types.iter_mut().enumerate().for_each(|(i, slot)| {
                let pn = i as u32 + 1;
                egui::ComboBox::from_label(format!("Player {pn}"))
                    .selected_text(match slot {
                        (_, PlayerType::Human) => "Human",
                        (_, PlayerType::Bot) => "Bot",
                        (_, PlayerType::None) => "None",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(slot, (pn, PlayerType::Human), "Human");
                        ui.selectable_value(slot, (pn, PlayerType::Bot), "Bot");
                        ui.selectable_value(slot, (pn, PlayerType::None), "None");
                    });
            });

            if ui.button("Start game").clicked() {
                *game = Game::PreGame(
                    map.clone(),
                    player_types
                        .iter()
                        .filter_map(|(pn, pt)| match pt {
                            PlayerType::Human => Some((*pn, Player::Human)),
                            PlayerType::Bot => Some((*pn, Player::Bot)),
                            PlayerType::None => None,
                        })
                        .collect(),
                    None,
                );

                next_state.set(AppState::LoadGame);
            }
            if ui.button("Back").clicked() {
                next_state.set(AppState::MainMenu);
            }
        });
    });
}
