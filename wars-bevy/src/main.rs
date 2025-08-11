use bevy::prelude::*;

mod animation;
mod bot;
mod camera;
mod components;
mod connection;
mod game_state;
mod interaction_state;
mod main_menu_state;
mod map;
mod resources;
mod theme;
mod ui;

mod bevy_nfws;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    MainMenu,
    SetupLocal,
    ConnectToServer,
    SelectGame,
    HostSelectMap,
    HostPreGame,
    JoinPreGame,
    LoadGame,
    InGame,
}
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            bevy_nfws::NfwsPlugin,
            game_state::GameStatePlugin,
            main_menu_state::MainMenuStatePlugin,
            connection::ConnectionPlugin,
        ))
        .insert_state(AppState::default())
        .run();
}
