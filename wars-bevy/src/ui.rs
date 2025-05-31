use crate::{
    EndTurnButton, EventProcessor, Funds, Game, MapAction, MapInteractionState, MenuBar,
    SpriteSheet, Theme, Unit, UnitHighlight, VisibleActionButtons,
};
use bevy::{log::tracing::Instrument, prelude::*};

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                funds_display_system,
                end_turn_button_system,
                map_action_button_system,
                visible_action_buttons_system,
            ),
        );
    }
}

fn setup(
    mut commands: Commands,
    game: Res<Game>,
    theme: Res<Theme>,
    sprite_sheet: Res<SpriteSheet>,
) {
    commands.spawn((
        MenuBar,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0),
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
        children![
            (button_bundle("End turn"), EndTurnButton,),
            (button_bundle("Wait"), MapAction::Wait, Visibility::Hidden),
            (
                button_bundle("Attack"),
                MapAction::Attack,
                Visibility::Hidden
            ),
            (
                button_bundle("Capture"),
                MapAction::Capture,
                Visibility::Hidden
            ),
            (
                button_bundle("Deploy"),
                MapAction::Deploy,
                Visibility::Hidden
            ),
            (
                button_bundle("Undeploy"),
                MapAction::Undeploy,
                Visibility::Hidden
            ),
            (
                button_bundle("Cancel"),
                MapAction::Cancel,
                Visibility::Hidden
            ),
        ],
    ));

    let unit_type_count = enum_iterator::cardinality::<wars::model::UnitType>();
    let item_width = 134.0;
    let item_height = 100.0;
    let num_rows = unit_type_count.isqrt();
    let num_cols = unit_type_count.div_ceil(num_rows);
    let build_menu = commands
        .spawn((
            Node {
                width: Val::Px(num_cols as f32 * item_width),
                height: Val::Px(num_rows as f32 * item_height),
                position_type: PositionType::Absolute,
                left: Val::Percent(10.0),
                top: Val::Percent(10.0),
                display: Display::Grid,
                padding: UiRect::all(Val::Px(3.0)),
                grid_template_columns: (0..num_cols).map(|_| GridTrack::px(item_width)).collect(),
                grid_template_rows: (0..num_rows).map(|_| GridTrack::px(item_height)).collect(),
                ..Default::default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .id();

    let player_number = game.in_turn_number();
    for unit_type in enum_iterator::all::<wars::model::UnitType>() {
        let info = wars::model::unit_type(unit_type);

        let button = commands
            .spawn((
                Node {
                    display: Display::Grid,
                    width: Val::Px(128.0),
                    height: Val::Px(96.0),
                    grid_template_rows: vec![GridTrack::px(64.0), GridTrack::px(32.0)],
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor(Color::BLACK.with_alpha(0.9)),
                BackgroundColor(Color::BLACK.with_alpha(0.5)),
                ChildOf(build_menu),
            ))
            .id();
        commands.spawn((
            sprite_sheet.image(theme.unit(unit_type, player_number).unwrap().unit_index),
            ChildOf(button),
        ));
        commands.spawn((Text(format!("{} cr", info.price)), ChildOf(button)));
    }
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

fn button_bundle(text: &str) -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(128.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::WHITE.with_alpha(0.5)),
        BorderColor(Color::BLACK.with_alpha(0.9)),
        children![(
            Text::new(text),
            TextFont {
                //font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::BLACK),
        )],
    )
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
    game: ResMut<Game>,
    mut event_processor: ResMut<EventProcessor>,
) {
    let Game(game) = &mut game.into_inner();
    for interaction in end_turn_buttons.iter() {
        match *interaction {
            Interaction::Pressed => {
                info!("end turn clicked");
                wars::game::action::end_turn(game, &mut |e| event_processor.queue.push_back(e))
                    .expect("Could not start game");
            }
            _ => (),
        }
    }
}
fn visible_action_buttons_system(
    visible_buttons: Res<VisibleActionButtons>,
    mut action_button_visibility: Query<(&MapAction, &mut Visibility), With<Button>>,
) {
    for (action, mut visibility) in action_button_visibility.iter_mut() {
        *visibility = if visible_buttons.contains(action) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
fn map_action_button_system(
    action_buttons: Query<(&Interaction, &MapAction), (Changed<Interaction>, With<Button>)>,
    game: ResMut<Game>,
    mut visible_action_buttons: ResMut<VisibleActionButtons>,
    mut state: ResMut<MapInteractionState>,
    mut event_processor: ResMut<EventProcessor>,
    mut unit_highlights: Query<(&Unit, &mut UnitHighlight)>,
) {
    let MapInteractionState::SelectAction(unit_id, ref path, ref options, ref attack_options) =
        *state
    else {
        return;
    };
    let game = &mut game.into_inner().0;
    let mut next_state = None;
    for (interaction, action) in action_buttons.iter() {
        if *interaction == Interaction::Pressed && options.contains(action) {
            info!("Pressed action {action:?}");
            match action {
                MapAction::Wait => {
                    wars::game::action::move_and_wait(game, unit_id, &path, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not move unit");
                    visible_action_buttons.clear();
                    next_state = Some(MapInteractionState::Normal);
                }
                MapAction::Deploy => {
                    wars::game::action::move_and_deploy(game, unit_id, &path, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not deploy");
                    visible_action_buttons.clear();
                    next_state = Some(MapInteractionState::Normal);
                }
                MapAction::Undeploy => {
                    wars::game::action::undeploy(game, unit_id, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not undeploy");
                    visible_action_buttons.clear();
                    next_state = Some(MapInteractionState::Normal);
                }
                MapAction::Cancel => {
                    next_state = Some(MapInteractionState::Normal);
                    visible_action_buttons.clear();
                }
                MapAction::Attack => {
                    next_state = Some(MapInteractionState::SelectAttackTarget(
                        unit_id,
                        path.clone(),
                        attack_options.clone(),
                    ));
                    for (Unit(uid), mut highlight) in unit_highlights.iter_mut() {
                        *highlight = if attack_options.contains(&uid) {
                            UnitHighlight::Target
                        } else {
                            UnitHighlight::Normal
                        };
                    }
                    visible_action_buttons.clear();
                }
                MapAction::Capture => {
                    wars::game::action::move_and_capture(game, unit_id, &path, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not capture");
                    visible_action_buttons.clear();
                    next_state = Some(MapInteractionState::Normal);
                }
            }
        }
    }

    if let Some(next_state) = next_state {
        *state = next_state;
    }
}
