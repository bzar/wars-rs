use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::f32::consts::TAU;

mod theme;

#[derive(Resource, Deref)]
struct Game(wars::game::Game);

#[derive(Resource, Deref)]
struct Theme(theme::Theme);

#[derive(Resource, Default)]
struct SpriteSheet {
    texture: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl SpriteSheet {
    fn sprite(&self, index: usize) -> Sprite {
        Sprite::from_atlas_image(
            self.texture.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index,
            },
        )
    }
}
#[derive(Component)]
struct Tile(wars::game::TileId);

#[derive(Component)]
struct Prop(wars::game::TileId);

#[derive(Component)]
struct Unit(wars::game::UnitId);

enum EventProcess {
    NoOp(wars::game::Event),
    Animation(Entity),
}
#[derive(Resource, Default)]
struct EventProcessor {
    pub state: Option<EventProcess>,
    pub queue: VecDeque<wars::game::Event>,
}

#[derive(Component)]
struct EndTurnButton;

#[derive(Component)]
struct TopBar;

#[derive(Component)]
struct Funds(u32);

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
enum MapAction {
    Wait,
    Attack,
    Capture,
    Deploy,
    Undeploy,
    Cancel,
}

#[derive(Resource, Default)]
enum MapInteractionState {
    #[default]
    Normal,
    SelectDestination(
        wars::game::UnitId,
        HashMap<wars::game::Position, Vec<wars::game::Position>>,
    ),
    SelectAction(
        wars::game::UnitId,
        Vec<wars::game::Position>,
        HashSet<MapAction>,
        HashSet<wars::game::UnitId>,
    ),
    SelectAttackTarget(
        wars::game::UnitId,
        Vec<wars::game::Position>,
        HashSet<wars::game::UnitId>,
    ),
}

#[derive(Event)]
enum HighlightEvent {
    ResetTiles,
    ResetUnits,
    MoveOptions(HashSet<wars::game::TileId>),
    AttackOptions(HashSet<wars::game::UnitId>),
    ShowActionButtons(HashSet<MapAction>),
    HideActionButtons,
}

#[derive(Event)]
enum UnitEvent {
    ResetMoved,
    Moved(Entity),
}
#[derive(Event)]
enum TileEvent {
    Captured(Entity, Option<Entity>),
}
fn main() {
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");
    const THEME_JSON: &str = include_str!("../assets/settings.json");
    let theme: theme::Theme = theme::Theme::from_json(THEME_JSON).unwrap();
    let map = wars::game::Map::from_json(THIRD_PARTY_MAP).unwrap();
    let mut game = wars::game::Game::new(map, &[0, 1]);

    let mut event_processor = EventProcessor::default();
    wars::game::action::start(&mut game, &mut |e| event_processor.queue.push_back(e))
        .expect("Could not start game");
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(Game(game))
        .insert_resource(Theme(theme))
        .insert_resource(MapInteractionState::default())
        .insert_resource(SpriteSheet::default())
        .insert_resource(event_processor)
        .add_event::<HighlightEvent>()
        .add_event::<UnitEvent>()
        .add_event::<TileEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                map_movement_input_system,
                event_processor_system,
                end_turn_button_system,
                highlight_event_system,
                unit_event_system,
                tile_event_system,
                map_action_button_system,
                funds_display_system,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    game: Res<Game>,
    theme: Res<Theme>,
    mut sprite_sheet: ResMut<SpriteSheet>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut asset_server: Res<AssetServer>,
) {
    let camera_transform = if let Some((min_x, min_y, max_x, max_y)) = game.tiles.rect() {
        let center_x = (max_x - min_x) / 2;
        let center_y = (max_y - min_y) / 2;
        let (cx, cy, _) = theme.map_hex_center(center_x, center_y);
        Transform::from_xyz(cx as f32, cy as f32 / 2.0, 0.0)
    } else {
        Transform::default()
    };
    commands.spawn((Camera2d, camera_transform, Msaa::Off));

    sprite_sheet.texture = asset_server
        .load_with_settings::<Image, bevy::image::ImageLoaderSettings>(
            &theme.spec.sheet.filename,
            |settings: &mut _| {
                settings.sampler = bevy::image::ImageSampler::nearest();
            },
        );
    sprite_sheet.layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(theme.spec.image.width, theme.spec.image.height),
        theme.spec.sheet.cols as u32,
        theme.spec.sheet.rows as u32,
        None,
        None,
    ));

    for (tile_id, tile) in game.tiles.iter_with_ids() {
        if let Some(theme_tile) = theme.tile(tile) {
            let (tx, ty, tz) = theme.map_hex_center(tile.x, tile.y);
            let pos = Vec2::new(tx as f32, (ty - theme_tile.offset) as f32);
            let tile_sprite = commands
                .spawn((
                    tile_bundle(*tile_id, tile, &theme, &sprite_sheet),
                    Transform::from_xyz(pos.x, pos.y, tz as f32),
                    Pickable::default(),
                ))
                .observe(tile_click_observer)
                .id();
            if let Some(prop_index) = theme_tile.prop_index {
                let (ox, oy) = theme.hex_sprite_center_offset();
                commands.spawn((
                    prop_bundle(*tile_id, tile, &theme, &sprite_sheet),
                    ChildOf(tile_sprite),
                    Transform::from_xyz(ox as f32, oy as f32, 0.1),
                ));
            }
            if let Some(unit_id) = tile.unit {
                let (ox, oy) = theme.hex_sprite_center_offset();
                let unit = game.units.get_ref(&unit_id).unwrap();
                let theme_unit = theme.unit(unit).unwrap();
                commands.spawn((
                    unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                    Transform::from_xyz(pos.x + ox as f32, pos.y + oy as f32, tz as f32 + 1.5),
                ));
            }
        }
    }

    commands.spawn((
        TopBar,
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

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0),
            bottom: Val::Px(0.0),
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
        children![(Funds(0), Text::new("0")), (Text::new(" credits"))],
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
fn unit_event_system(
    mut events: EventReader<UnitEvent>,
    mut units: Query<&mut Sprite, With<Unit>>,
) {
    for event in events.read() {
        match event {
            UnitEvent::ResetMoved => {
                for mut sprite in units.iter_mut() {
                    sprite.flip_y = false;
                }
            }
            UnitEvent::Moved(unit_entity_id) => {
                units.get_mut(*unit_entity_id).unwrap().flip_y = true;
            }
        }
    }
}
fn tile_event_system(
    theme: Res<Theme>,
    game: Res<Game>,
    mut events: EventReader<TileEvent>,
    mut tiles: Query<&Tile>,
    mut tile_sprites: Query<&mut Sprite, (With<Tile>, Without<Prop>)>,
    mut prop_sprites: Query<&mut Sprite, (With<Prop>, Without<Tile>)>,
) {
    for event in events.read() {
        match event {
            TileEvent::Captured(tile_entity_id, prop_entity_id) => {
                let Tile(tile_id) = tiles.get(*tile_entity_id).unwrap();
                let tile = game.tiles.get(*tile_id).unwrap();
                let theme_tile = theme.tile(&tile).unwrap();
                if let Some(prop_entity_id) = prop_entity_id {
                    let mut prop_sprite = prop_sprites.get_mut(*prop_entity_id).unwrap();
                    if let Some(prop_index) = theme_tile.prop_index {
                        prop_sprite
                            .texture_atlas
                            .as_mut()
                            .map(|a| a.index = prop_index);
                    }
                }
            }
        }
    }
}
fn highlight_event_system(
    mut events: EventReader<HighlightEvent>,
    mut units: Query<(&mut Sprite, &Unit), (Without<Tile>, Without<Prop>)>,
    mut tiles: Query<(&mut Sprite, &Tile), (Without<Prop>, Without<Unit>)>,
    mut props: Query<(&mut Sprite, &Prop), (Without<Tile>, Without<Unit>)>,
    mut action_button_visibility: Query<(&MapAction, &mut Visibility), With<Button>>,
) {
    for event in events.read() {
        match event {
            HighlightEvent::ResetTiles => {
                info!("Highlight nothing");
                for (mut sprite, _) in tiles.iter_mut() {
                    sprite.color = Color::WHITE;
                }
                for (mut sprite, _) in props.iter_mut() {
                    sprite.color = Color::WHITE;
                }
            }
            HighlightEvent::ResetUnits => {
                info!("Highlight nothing");
                for (mut sprite, _) in units.iter_mut() {
                    sprite.color = Color::WHITE;
                }
            }
            HighlightEvent::MoveOptions(tile_ids) => {
                info!("Move highlight {tile_ids:?}");
                for (mut sprite, Tile(tile_id)) in tiles.iter_mut() {
                    if !tile_ids.contains(tile_id) {
                        sprite.color = Color::srgba(0.5, 0.5, 0.5, 1.0);
                    }
                }
                for (mut sprite, Prop(tile_id)) in props.iter_mut() {
                    if !tile_ids.contains(tile_id) {
                        sprite.color = Color::srgba(0.5, 0.5, 0.5, 1.0);
                    }
                }
            }
            HighlightEvent::AttackOptions(unit_ids) => {
                for (mut sprite, Unit(unit_id)) in units.iter_mut() {
                    if unit_ids.contains(unit_id) {
                        sprite.color = Color::srgba(1.0, 0.1, 0.1, 1.0);
                    }
                }
            }
            HighlightEvent::ShowActionButtons(actions) => {
                for (action, mut visibility) in action_button_visibility.iter_mut() {
                    *visibility = if actions.contains(action) {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
            }
            HighlightEvent::HideActionButtons => {
                for (action, mut visibility) in action_button_visibility.iter_mut() {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}
fn map_action_button_system(
    action_buttons: Query<(&Interaction, &MapAction), (Changed<Interaction>, With<Button>)>,
    game: ResMut<Game>,
    mut state: ResMut<MapInteractionState>,
    mut event_processor: ResMut<EventProcessor>,
    mut highlight_event_writer: EventWriter<HighlightEvent>,
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
                    highlight_event_writer.write(HighlightEvent::HideActionButtons);
                    next_state = Some(MapInteractionState::Normal);
                }
                MapAction::Deploy => {
                    wars::game::action::move_and_deploy(game, unit_id, &path, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not deploy");
                    highlight_event_writer.write(HighlightEvent::HideActionButtons);
                    next_state = Some(MapInteractionState::Normal);
                }
                MapAction::Undeploy => {
                    wars::game::action::undeploy(game, unit_id, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not undeploy");
                    highlight_event_writer.write(HighlightEvent::HideActionButtons);
                    next_state = Some(MapInteractionState::Normal);
                }
                MapAction::Cancel => {
                    next_state = Some(MapInteractionState::Normal);
                    highlight_event_writer.write(HighlightEvent::HideActionButtons);
                }
                MapAction::Attack => {
                    next_state = Some(MapInteractionState::SelectAttackTarget(
                        unit_id,
                        path.clone(),
                        attack_options.clone(),
                    ));
                    highlight_event_writer
                        .write(HighlightEvent::AttackOptions(attack_options.clone()));
                }
                MapAction::Capture => {
                    wars::game::action::move_and_capture(game, unit_id, &path, &mut |e| {
                        event_processor.queue.push_back(e)
                    })
                    .expect("Could not capture");
                    highlight_event_writer.write(HighlightEvent::HideActionButtons);
                    next_state = Some(MapInteractionState::Normal);
                }
                _ => unimplemented!(),
            }
        }
    }

    if let Some(next_state) = next_state {
        *state = next_state;
    }
}
fn tile_click_observer(
    trigger: Trigger<Pointer<Click>>,
    tile_query: Query<&Tile>,
    mut game: ResMut<Game>,
    mut state: ResMut<MapInteractionState>,
    mut event_processor: ResMut<EventProcessor>,
    mut highlight_event_writer: EventWriter<HighlightEvent>,
) {
    info!("{trigger:?}");
    let Ok(Tile(tile_id)) = tile_query.get(trigger.target()) else {
        info!("{}", trigger.target());
        return;
    };
    let tile = game.tiles.get(*tile_id).unwrap();
    let position = wars::game::Position(tile.x, tile.y);
    info!("{tile:?}");

    let mut next_state = None;
    match *state {
        MapInteractionState::Normal => {
            if let Some(unit_id) = tile.unit {
                let unit = game.units.get(unit_id).unwrap();
                if unit.owner == game.in_turn_number() && !unit.moved {
                    info!("Moving unit {unit_id}");
                    if let Some(destinations) = game.unit_move_options(unit_id) {
                        highlight_event_writer.write(HighlightEvent::MoveOptions(
                            game.tiles
                                .iter_with_ids()
                                .filter_map(|(tid, t)| {
                                    destinations
                                        .contains_key(&wars::game::Position(t.x, t.y))
                                        .then_some(*tid)
                                })
                                .collect(),
                        ));
                        next_state = Some(MapInteractionState::SelectDestination(
                            unit_id,
                            destinations,
                        ))
                    }
                }
            }
        }
        MapInteractionState::SelectDestination(unit_id, ref destinations) => {
            info!("Moving unit {unit_id} to {},{}", tile.x, tile.y);
            highlight_event_writer.write(HighlightEvent::ResetTiles);
            if let Some(path) = destinations.get(&position) {
                let unit = game.units.get_ref(&unit_id).unwrap();
                let mut action_options: HashSet<MapAction> =
                    [MapAction::Wait, MapAction::Cancel].into_iter().collect();
                if unit.can_deploy() {
                    if unit.deployed {
                        action_options.insert(MapAction::Undeploy);
                    } else {
                        action_options.insert(MapAction::Deploy);
                    }
                }
                let attack_options = game.unit_attack_options(unit_id, &position);
                info!("attack options: {attack_options:?}");
                if !attack_options.is_empty() {
                    action_options.insert(MapAction::Attack);
                }
                if game.unit_can_capture_tile(unit_id, *tile_id).is_ok() {
                    action_options.insert(MapAction::Capture);
                }
                highlight_event_writer
                    .write(HighlightEvent::ShowActionButtons(action_options.clone()));
                next_state = Some(MapInteractionState::SelectAction(
                    unit_id,
                    path.clone(),
                    action_options,
                    attack_options,
                ));
            } else {
                next_state = Some(MapInteractionState::Normal);
            }
        }
        MapInteractionState::SelectAction(
            unit_id,
            ref path,
            ref action_options,
            ref attack_options,
        ) => {}
        MapInteractionState::SelectAttackTarget(unit_id, ref path, ref attack_options) => {
            if let Ok((_, target_tile)) = game.tiles.get_at(&position) {
                if let Some(target_id) = target_tile.unit {
                    highlight_event_writer.write(HighlightEvent::ResetUnits);
                    highlight_event_writer.write(HighlightEvent::HideActionButtons);
                    wars::game::action::move_and_attack(
                        &mut game.into_inner().0,
                        unit_id,
                        path,
                        target_id,
                        &mut |e| event_processor.queue.push_back(e),
                    )
                    .expect("Could not move unit");
                    next_state = Some(MapInteractionState::Normal);
                }
            }
        }
    };
    if let Some(next_state) = next_state {
        *state = next_state;
    }
}
fn unit_bundle(
    unit_id: wars::game::UnitId,
    unit: &wars::game::Unit,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let theme_unit = theme.unit(unit).unwrap();
    (Unit(unit_id), sprite_sheet.sprite(theme_unit.unit_index))
}

fn tile_bundle(
    tile_id: wars::game::TileId,
    tile: &wars::game::Tile,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let theme_tile = theme.tile(tile).unwrap();
    (Tile(tile_id), sprite_sheet.sprite(theme_tile.tile_index))
}

fn prop_bundle(
    tile_id: wars::game::TileId,
    tile: &wars::game::Tile,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let theme_tile = theme.tile(tile).unwrap();
    (
        Prop(tile_id),
        sprite_sheet.sprite(theme_tile.prop_index.unwrap()),
    )
}

fn map_movement_input_system(
    mut camera_query: Single<(&mut Camera, &mut Transform, &mut Projection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
) {
    let (camera, mut transform, mut projection) = camera_query.into_inner();
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

fn event_processor_system(
    mut commands: Commands,
    mut ep: ResMut<EventProcessor>,
    game: Res<Game>,
    theme: Res<Theme>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut units: Query<(Entity, &Unit)>,
    mut tiles: Query<(Entity, &Tile)>,
    mut props: Query<(Entity, &Prop)>,
    mut funds: Query<&mut Funds>,
    mut players: Query<&AnimationPlayer>,
    mut top_bar_colors: Query<&mut BackgroundColor, With<TopBar>>,
    mut unit_event_writer: EventWriter<UnitEvent>,
    mut tile_event_writer: EventWriter<TileEvent>,
) {
    ep.state = if let Some(state) = ep.state.take() {
        match state {
            EventProcess::NoOp(event) => {
                info!("Skipping event {event:?}");
                None
            }
            EventProcess::Animation(entity) => {
                let player = players.get(entity).unwrap();
                if player.all_finished() {
                    info!("Finished animation");
                    commands.entity(entity).remove::<(
                        Name,
                        AnimationPlayer,
                        AnimationGraphHandle,
                        AnimationTarget,
                    )>();
                    None
                } else {
                    Some(EventProcess::Animation(entity))
                }
            }
        }
    } else {
        None
    };

    let find_unit_entity_id = |unit_id| {
        units
            .iter()
            .find_map(|(entity_id, Unit(uid))| (*uid == unit_id).then_some(entity_id))
    };
    let find_tile_entity_id = |tile_id| {
        tiles
            .iter()
            .find_map(|(entity_id, Tile(tid))| (*tid == tile_id).then_some(entity_id))
    };
    let find_prop_entity_id = |tile_id| {
        props
            .iter()
            .find_map(|(entity_id, Prop(tid))| (*tid == tile_id).then_some(entity_id))
    };
    if ep.state.is_none() {
        if let Some(event) = ep.queue.pop_front() {
            use wars::game::Event;
            ep.state = match event {
                Event::StartTurn(player_number) => {
                    if let Some(player_color) = theme.spec.player_colors.get(player_number as usize)
                    {
                        for mut top_bar_color in top_bar_colors.iter_mut() {
                            top_bar_color.0 = Color::srgba_u8(
                                player_color.r,
                                player_color.g,
                                player_color.b,
                                u8::MAX,
                            );
                        }
                    }

                    if let Some(player) = game.get_player(player_number) {
                        for mut fund in funds.iter_mut() {
                            *fund = Funds(player.funds);
                        }
                    }
                    None
                }
                Event::EndTurn(player_number) => {
                    unit_event_writer.write(UnitEvent::ResetMoved);
                    None
                }
                Event::Funds(player_number, credits) => {
                    if let Some(player) = game.get_player(player_number) {
                        for mut fund in funds.iter_mut() {
                            *fund = Funds(player.funds);
                        }
                    }
                    None
                }
                //Event::UnitRepair(unit_id, health) => None,
                //Event::WinGame(player_number) => None,
                //Event::Surrender(player_number) => None,
                Event::Move(unit_id, path) => {
                    if path.len() > 1 {
                        let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                        let mut unit = commands.entity(unit_entity_id);
                        let info = move_animation(path, &theme, &mut animations, &mut graphs);
                        unit.insert((
                            info.target_name,
                            AnimationGraphHandle(info.graph),
                            info.player,
                            AnimationTarget {
                                id: info.target_id,
                                player: unit.id(),
                            },
                        ));
                        Some(EventProcess::Animation(unit_entity_id))
                    } else {
                        None
                    }
                }
                Event::Wait(unit_id) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    unit_event_writer.write(UnitEvent::Moved(unit_entity_id));
                    None
                }
                //Event::Attack(attacking_unit_id, target_unit_id, health) => None,
                //Event::Counterattack(attacking_unit_id, target_unit_id, health) => None,
                Event::Destroyed(attacking_unit_id, target_unit_id) => {
                    let unit_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    commands.entity(unit_entity_id).despawn();

                    None
                }
                //Event::Deploy(unit_id) => None,
                //Event::Undeploy(unit_id) => None,
                //Event::Load(loaded_unit_id, loading_unit_id) => None,
                //Event::Unload(unloading_unit_id, unloaded_unit_id, position) => None,
                //Event::Capture(unit_id, tile_id, capture_points) => None,
                Event::Captured(unit_id, tile_id) => {
                    let tile_entity_id = find_tile_entity_id(tile_id).unwrap();
                    let prop_entity_id = find_prop_entity_id(tile_id);
                    tile_event_writer.write(TileEvent::Captured(tile_entity_id, prop_entity_id));
                    None
                }
                //Event::Build(tile_id, unit_id, unit_type, credits) => None,
                //Event::TileCapturePointRegen(tile_id, capture_points) => None,
                e => Some(EventProcess::NoOp(e)),
            };
        }
    }
}

use bevy::animation::{AnimationTarget, AnimationTargetId, animated_field};
struct AnimationInfo {
    target_name: Name,
    target_id: AnimationTargetId,
    graph: Handle<AnimationGraph>,
    player: AnimationPlayer,
}
fn move_animation(
    path: impl IntoIterator<Item = wars::game::Position>,
    theme: &Theme,
    animations: &mut ResMut<Assets<AnimationClip>>,
    graphs: &mut ResMut<Assets<AnimationGraph>>,
) -> AnimationInfo {
    let mut animation = AnimationClip::default();
    let target_name = Name::new("unit");
    let target_id = AnimationTargetId::from_name(&target_name);
    let (ox, oy) = theme.hex_sprite_center_offset();
    let waypoints: Vec<Vec3> = path
        .into_iter()
        .map(|wars::game::Position(hx, hy)| {
            let (x, y, z) = theme.map_hex_center(hx, hy);
            Vec3::new((x + ox) as f32, (y + oy) as f32, z as f32 + 1.5)
        })
        .collect();
    animation.add_curve_to_target(
        target_id,
        AnimatableCurve::new(
            animated_field!(Transform::translation),
            SampleAutoCurve::new(
                Interval::new(0.0, 0.2 * waypoints.len() as f32).unwrap(),
                waypoints,
            )
            .unwrap(),
        ),
    );
    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
    let graph = graphs.add(graph);
    let mut player = AnimationPlayer::default();
    player.play(animation_index);

    AnimationInfo {
        target_name,
        target_id,
        graph,
        player,
    }
}
