use crate::{components::*, resources::*, AppState};
use bevy::prelude::*;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                funds_display_system,
                end_turn_button_system,
                input_layer_system,
                player_colored_ui_system,
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(64.0),
            height: Val::Px(64.0),
            top: Val::Px(8.0),
            right: Val::Px(8.0),
            ..Default::default()
        },
        EndTurnButton,
        Button,
        children![(
            ImageNode::new(asset_server.load("gui/action-endturn.png")),
            PlayerColored
        )],
    ));
    commands.spawn((
        MenuBar,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0),
            bottom: Val::Px(0.0),
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
        children![(Funds(0), Text::new("0")), Text::new(" credits")],
    ));
}

// Necessary to move click focus between UI and Game to avoid clicks being handled by both
fn input_layer_system(
    mut input_layer: ResMut<InputLayer>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    node_interactions: Query<&Interaction, With<Node>>,
    touches: Res<Touches>,
) {
    if mouse_buttons.pressed(MouseButton::Left)
        || mouse_buttons.pressed(MouseButton::Right)
        || touches.iter().next().is_some()
    {
        return;
    }
    *input_layer = if node_interactions.iter().all(|&i| i == Interaction::None) {
        InputLayer::Game
    } else {
        InputLayer::UI
    };
}
fn funds_display_system(mut funds_query: Query<(&Funds, &mut Text), Changed<Funds>>) {
    for (Funds(funds), mut text) in funds_query.iter_mut() {
        *text = Text(format!("{}", funds));
    }
}

fn end_turn_button_system(
    end_turn_buttons: Query<
        &Interaction,
        (Changed<Interaction>, With<Button>, With<EndTurnButton>),
    >,
    mut events: EventWriter<InputEvent>,
) {
    for interaction in end_turn_buttons.iter() {
        if *interaction == Interaction::Pressed {
            events.write(InputEvent::EndTurn);
        }
    }
}

fn player_colored_ui_system(
    in_turn: Res<InTurnPlayer>,
    theme: Res<Theme>,
    mut image_nodes: Query<&mut ImageNode, With<PlayerColored>>,
) {
    let Some(player_color) = theme
        .spec
        .player_colors
        .get(in_turn.0.unwrap_or(0) as usize)
    else {
        return;
    };
    for mut image_node in image_nodes.iter_mut() {
        if let Some(atlas_image) = image_node.texture_atlas.as_mut() {
            theme
                .recolor(atlas_image.index, in_turn.0)
                .map(move |i| atlas_image.index = i);
        } else {
            image_node.color = player_color.into();
        }
    }
}
