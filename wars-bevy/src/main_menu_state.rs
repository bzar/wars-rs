use crate::{
    AppState,
    resources::{Game, Player},
};
use bevy::prelude::*;

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
    mut map_name: Local<String>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            let mut selected_map_name = if map_name.is_empty() {
                "My awesome map"
            } else {
                &map_name
            };
            egui::ComboBox::from_label("Map")
                .selected_text(selected_map_name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut selected_map_name,
                        "my_awesome_map.json",
                        "My awesome map",
                    );
                    ui.selectable_value(&mut selected_map_name, "third_party.json", "Third party");
                    ui.selectable_value(&mut selected_map_name, "hexed.json", "Hexed");
                });
            if *map_name != selected_map_name {
                *map_name = selected_map_name.to_owned();
            }

            ui.checkbox(&mut player_1_ai, "Player 1 AI");
            ui.checkbox(&mut player_2_ai, "Player 2 AI");

            let player_or_bot = |is_ai| {
                if is_ai { Player::Bot } else { Player::Human }
            };

            if ui.button("Start game").clicked() {
                game.players = [
                    (1, player_or_bot(*player_1_ai)),
                    (2, player_or_bot(*player_2_ai)),
                ]
                .into_iter()
                .collect();

                next_state.set(AppState::InGame);
            }
        });
    });
}
