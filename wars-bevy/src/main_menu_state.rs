use crate::{
    resources::{Game, Player},
    AppState,
};
use bevy::prelude::*;
use include_dir::{include_dir, File};

pub struct MainMenuStatePlugin;

impl Plugin for MainMenuStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_plugins(bevy_egui::EguiPlugin {
                enable_multipass_for_primary_context: true,
            })
            .add_systems(
                bevy_egui::EguiContextPass,
                main_menu_system.run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnEnter(AppState::MainMenu), on_enter);
    }
}

fn setup() {}

fn on_enter() {}

fn main_menu_system(
    mut contexts: bevy_egui::EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut game: ResMut<Game>,
    mut player_1_ai: Local<bool>,
    mut player_2_ai: Local<bool>,
    mut map_index: Local<usize>,
) {
    let maps: Vec<wars::game::Map> = include_dir!("$CARGO_MANIFEST_DIR/../data/maps")
        .entries()
        .into_iter()
        .filter_map(|e| {
            e.as_file()
                .and_then(File::contents_utf8)
                .and_then(|content| wars::game::Map::from_json(content).ok())
        })
        .collect();

    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            egui::ComboBox::from_label("Map")
                .selected_text(&maps[*map_index].name)
                .show_ui(ui, |ui| {
                    for (i, map) in maps.iter().enumerate() {
                        ui.selectable_value(&mut *map_index, i, &map.name);
                    }
                });

            ui.checkbox(&mut player_1_ai, "Player 1 AI");
            ui.checkbox(&mut player_2_ai, "Player 2 AI");

            let player_or_bot = |is_ai| {
                if is_ai {
                    Player::Bot
                } else {
                    Player::Human
                }
            };

            if ui.button("Start game").clicked() {
                *game = Game::PreGame(
                    maps[*map_index].clone(),
                    [
                        (1, player_or_bot(*player_1_ai)),
                        (2, player_or_bot(*player_2_ai)),
                    ]
                    .into(),
                );

                next_state.set(AppState::LoadGame);
            }
        });
    });
}
