use std::collections::HashSet;

use crate::{
    Action, ActionMenu, BuildItem, BuildMenu, DisabledButton, EndTurnButton, Funds, Game,
    InTurnPlayer, InputEvent, InputLayer, MenuBar, PlayerColored, SpriteSheet, Theme, UnloadMenu,
    UnloadMenuItem, VisibleActionButtons,
};
use bevy::prelude::*;

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
                build_button_system,
                disable_build_items_outside_price_range,
                disabled_button_system,
                unload_menu_system,
                unload_menu_item_button_system,
                input_layer_system,
                player_colored_ui_system,
            ),
        );
    }
}

fn setup(
    mut commands: Commands,
    game: Res<Game>,
    theme: Res<Theme>,
    sprite_sheet: Res<SpriteSheet>,
    asset_server: Res<AssetServer>,
) {
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
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(64.0),
            height: Val::Px(64.0),
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..Default::default()
        },
        Button,
        Action::Cancel,
        children![(
            ImageNode::new(asset_server.load("gui/action-cancel.png")),
            PlayerColored
        )],
    ));
    commands.spawn((
        ActionMenu,
        Node {
            display: Display::Grid,
            grid_template_columns: (0..3).map(|_| GridTrack::px(64.0)).into_iter().collect(),
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        children![
            (
                button_bundle("Wait", asset_server.load("gui/action-wait.png")),
                Action::Wait,
            ),
            (
                button_bundle("Attack", asset_server.load("gui/action-attack.png")),
                Action::Attack,
            ),
            (
                button_bundle("Capture", asset_server.load("gui/action-capture.png")),
                Action::Capture,
            ),
            (
                button_bundle("Deploy", asset_server.load("gui/action-deploy.png")),
                Action::Deploy,
            ),
            (
                button_bundle("Undeploy", asset_server.load("gui/action-undeploy.png")),
                Action::Undeploy,
            ),
            (
                button_bundle("Load", asset_server.load("gui/action-load.png")),
                Action::Load,
            ),
            (
                button_bundle("Unload", asset_server.load("gui/action-unload.png")),
                Action::Unload,
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
            BuildMenu {
                price_limit: 0,
                unit_classes: HashSet::new(),
            },
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
            Visibility::Hidden,
        ))
        .id();

    commands.spawn((
        UnloadMenu::default(),
        Node {
            display: Display::Grid,
            padding: UiRect::all(Val::Px(3.0)),
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..Default::default()
        },
        BorderColor(Color::BLACK.with_alpha(0.9)),
        BackgroundColor(Color::BLACK.with_alpha(0.8)),
        Visibility::Hidden,
    ));

    let player_number = game.state.in_turn_number();
    let mut unit_types = enum_iterator::all::<wars::model::UnitType>().collect::<Vec<_>>();
    unit_types.sort_by_key(|t| wars::model::unit_type(*t).price);
    for unit_type in unit_types {
        let info = wars::model::unit_type(unit_type);

        let button = commands
            .spawn((
                Button,
                BuildItem(unit_type),
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

fn button_bundle(text: &str, icon: Handle<Image>) -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(64.0),
            height: Val::Px(64.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        children![(
            /*Text::new(text),
            TextFont {
                //font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::BLACK),*/
            ImageNode::new(icon),
            PlayerColored
        )],
    )
}

// Necessary to move click focus between UI and Game to avoid clicks being handled by both
fn input_layer_system(
    mut input_layer: ResMut<InputLayer>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    node_interactions: Query<&Interaction, With<Node>>,
) {
    if mouse_buttons.pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Right) {
        return;
    }
    *input_layer = if node_interactions.iter().all(|&i| i == Interaction::None) {
        InputLayer::Game
    } else {
        InputLayer::UI
    };
}
fn unload_menu_system(
    mut commands: Commands,
    sprite_sheet: Res<SpriteSheet>,
    game: Res<Game>,
    theme: Res<Theme>,
    mut unload_menus: Query<(Entity, &UnloadMenu, &mut Visibility), Changed<UnloadMenu>>,
) {
    let Ok((entity_id, UnloadMenu(unit_ids), mut visibility)) = unload_menus.single_mut() else {
        return;
    };

    if unit_ids.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Inherited;

    let mut entity = commands.entity(entity_id);
    entity.despawn_related::<Children>();

    for unit_id in unit_ids {
        let unit = game.state.units.get_ref(unit_id).unwrap();
        commands.spawn((
            ChildOf(entity_id),
            Button,
            UnloadMenuItem(*unit_id),
            children![
                sprite_sheet.image(theme.unit(unit.unit_type, unit.owner).unwrap().unit_index),
            ],
        ));
    }
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
fn visible_action_buttons_system(
    visible_buttons: Res<VisibleActionButtons>,
    mut action_button_visibility: Query<(&Action, &mut Node), With<Button>>,
) {
    for (action, mut node) in action_button_visibility.iter_mut() {
        node.display = if visible_buttons.contains(action) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn unload_menu_item_button_system(
    unload_menu_items: Query<
        (&Interaction, &UnloadMenuItem),
        (Changed<Interaction>, With<Button>, Without<DisabledButton>),
    >,
    mut events: EventWriter<InputEvent>,
) {
    for (&interaction, UnloadMenuItem(unit_id)) in unload_menu_items.iter() {
        if interaction != Interaction::Pressed {
            continue;
        }

        events.write(InputEvent::UnloadUnit(*unit_id));
    }
}

fn build_button_system(
    build_buttons: Query<
        (&Interaction, &BuildItem),
        (Changed<Interaction>, With<Button>, Without<DisabledButton>),
    >,
    mut events: EventWriter<InputEvent>,
) {
    let presses = build_buttons
        .iter()
        .filter_map(|(i, bi)| (*i == Interaction::Pressed).then_some(bi));

    for BuildItem(unit_type) in presses {
        events.write(InputEvent::BuildUnit(*unit_type));
    }
}
fn map_action_button_system(
    action_buttons: Query<(&Interaction, &Action), (Changed<Interaction>, With<Button>)>,
    mut events: EventWriter<InputEvent>,
) {
    for (interaction, action) in action_buttons.iter() {
        if *interaction == Interaction::Pressed {
            events.write(InputEvent::Action(*action));
        }
    }
}

fn disabled_button_system(
    mut disabled_buttons: Query<&mut BackgroundColor, (With<Button>, With<DisabledButton>)>,
    mut enabled_buttons: Query<&mut BackgroundColor, (With<Button>, Without<DisabledButton>)>,
) {
    for mut background_color in disabled_buttons.iter_mut() {
        *background_color = BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 1.0));
    }
    for mut background_color in enabled_buttons.iter_mut() {
        *background_color = BackgroundColor(Color::srgba(0.7, 0.7, 0.7, 1.0));
    }
}
fn disable_build_items_outside_price_range(
    mut commands: Commands,
    build_menus: Query<(&BuildMenu, &Children)>,
    mut build_items: Query<(&mut Node, &BuildItem)>,
) {
    let (
        BuildMenu {
            price_limit,
            unit_classes,
        },
        children,
    ) = build_menus.single().unwrap();
    for child in children.iter() {
        let Ok((mut node, BuildItem(unit_type))) = build_items.get_mut(child) else {
            continue;
        };

        let unit_info = wars::model::unit_type(*unit_type);

        let mut build_item = commands.entity(child);
        if unit_info.price > *price_limit {
            build_item.insert(DisabledButton);
        } else {
            build_item.remove::<DisabledButton>();
        }

        node.display = if unit_classes.contains(&unit_info.unit_class) {
            Display::Flex
        } else {
            Display::None
        };
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
        image_node.color = player_color.into();
    }
}
