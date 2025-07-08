use bevy::prelude::*;

mod animation;
mod bot;
mod camera;
mod components;
mod game_state;
mod interaction_state;
mod main_menu_state;
mod map;
mod resources;
mod theme;
mod ui;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    MainMenu,
    LoadGame,
    InGame,
}
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            game_state::GameStatePlugin,
            main_menu_state::MainMenuStatePlugin,
        ))
        .insert_state(AppState::default())
        .run();
}
