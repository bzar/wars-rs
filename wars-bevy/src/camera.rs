use crate::{
    resources::{Game, Theme},
    AppState,
};
use bevy::prelude::*;
pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_camera)
            .add_systems(
                Update,
                map_movement_input_system.run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(AppState::InGame), on_enter_game);
    }
}

fn on_enter_game(
    game: Res<Game>,
    theme: Res<Theme>,
    mut camera_transform: Single<&mut Transform, With<Camera>>,
) {
    let Game::InGame(game, ..) = game.as_ref() else {
        panic!("Not in game");
    };
    *(camera_transform.as_mut()) = if let Some((min_x, min_y, max_x, max_y)) = game.tiles.rect() {
        let center_x = (max_x - min_x) / 2;
        let center_y = (max_y - min_y) / 2;
        let (cx, cy, _) = theme.map_hex_center(center_x, center_y);
        Transform::from_xyz(cx as f32, cy as f32 / 2.0, 0.0)
    } else {
        Transform::default()
    };
}
fn add_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Off));
}

fn map_movement_input_system(
    camera_query: Single<(&mut Transform, &mut Projection), With<Camera>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
) {
    let (mut transform, mut projection) = camera_query.into_inner();
    if mouse_buttons.pressed(MouseButton::Right) {
        let delta = mouse_motion.delta;
        transform.translation.x -= delta.x;
        transform.translation.y += delta.y;
    }

    if let Projection::Orthographic(projection2d) = &mut *projection {
        if mouse_scroll.delta != Vec2::ZERO {
            let delta = mouse_scroll.delta.y;
            if delta < 0.0 {
                projection2d.scale *= bevy::math::ops::powf(1.1, -delta);
            } else if delta > 0.0 {
                projection2d.scale *= bevy::math::ops::powf(0.9, delta);
            }
        }
    }
}
