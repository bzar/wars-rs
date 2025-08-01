use crate::{
    bevy_nfws::{NfwsEvent, NfwsHandle},
    resources::{Game, Player},
    AppState,
};
use bevy::prelude::*;
use include_dir::{include_dir, File};

pub struct MainMenuStatePlugin;

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
                ),
            );
    }
}

#[derive(PartialEq, Debug)]
enum PlayerType {
    Human,
    Bot,
    None,
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
    mut commands: Commands,
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut address: Local<Option<String>>,
    mut handles: Query<(Entity, &mut NfwsHandle)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if let Some((entity, mut handle)) = handles.iter_mut().next() {
        let version_message = wars::protocol::version_message();
        match handle.next_event() {
            crate::bevy_nfws::NfwsPollResult::Closed => commands.entity(entity).despawn(),
            crate::bevy_nfws::NfwsPollResult::Empty => (),
            crate::bevy_nfws::NfwsPollResult::Event(nfws_event) => match nfws_event {
                NfwsEvent::Connecting => (),
                NfwsEvent::Connected => (),
                NfwsEvent::TextMessage(t) if t == version_message => {
                    next_state.set(AppState::SelectGame);
                }
                NfwsEvent::TextMessage(t) => {
                    info!("Received text message: {t}, expected {version_message}");
                    commands.entity(entity).despawn();
                }
                NfwsEvent::BinaryMessage(_) => {
                    info!("Received unexpected binary message");
                    commands.entity(entity).despawn();
                }
                NfwsEvent::Error(e) => {
                    error!("Error connecting: {e:?}");
                    commands.entity(entity).despawn();
                }
                NfwsEvent::Closed(e) => {
                    error!("Server closed the connection connecting: {e:?}");
                    commands.entity(entity).despawn();
                }
            },
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Connecting...");
        });
    } else {
        let address = address.get_or_insert("ws://localhost:3000/ws".to_owned());
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.text_edit_singleline(address);
            if ui.button("Connect").clicked() {
                let new_handle = crate::bevy_nfws::NfwsHandle::new(address.clone());
                commands.spawn(new_handle);
            }
            if ui.button("Back").clicked() {
                next_state.set(AppState::MainMenu);
            }
        });
    }
}

fn select_game_menu_system(
    mut commands: Commands,
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut handles: Query<(Entity, &mut NfwsHandle)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let Ok((handle_entity, handle)) = handles.single() else {
        next_state.set(AppState::MainMenu);
        return;
    };

    // TODO: Fetch game list, show it
    egui::CentralPanel::default().show(ctx, |ui| {
        if ui.button("Host a new game").clicked() {
            next_state.set(AppState::HostSelectMap);
        }
        if ui.button("Back").clicked() {
            commands.entity(handle_entity).despawn();
            next_state.set(AppState::ConnectToServer);
        }
        ui.label("TODO: Joinable games here");
    });
}
fn host_select_map_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut handles: Query<(Entity, &mut NfwsHandle)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let Ok((handle_entity, handle)) = handles.single() else {
        next_state.set(AppState::MainMenu);
        return;
    };

    // TODO: Fetch game list, show it
    egui::CentralPanel::default().show(ctx, |ui| {
        if ui.button("Create game").clicked() {
            next_state.set(AppState::HostPreGame);
        }
        if ui.button("Back").clicked() {
            next_state.set(AppState::SelectGame);
        }
        ui.label("TODO: map select");
    });
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
                );

                next_state.set(AppState::LoadGame);
            }
            if ui.button("Back").clicked() {
                next_state.set(AppState::MainMenu);
            }
        });
    });
}
