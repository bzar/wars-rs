use crate::{
    AppState,
    resources::{Game, Player},
};
use bevy::prelude::*;
use include_dir::{File, include_dir};

pub struct MainMenuStatePlugin;

impl Plugin for MainMenuStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_egui::EguiPlugin::default())
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                main_menu_system.run_if(in_state(AppState::MainMenu)),
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
        });
    });
}
