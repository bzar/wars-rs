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

#[derive(Resource, Default)]
enum MapInteractionState {
    #[default]
    Normal,
    SelectDestination(
        wars::game::UnitId,
        HashMap<wars::game::Position, Vec<wars::game::Position>>,
    ),
    //SelectAttackTarget(wars::game::UnitId, wars::game::Position)
}

#[derive(Event)]
enum HighlightEvent {
    Nothing,
    MoveOptions(HashSet<wars::game::TileId>),
    //AttackOptions(Vec<wars::game::UnitId>),
}

#[derive(Event)]
enum UnitEvent {
    ReadyToMove(wars::game::UnitId),
    Moved(wars::game::UnitId),
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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                map_movement_input_system,
                event_processor_system,
                end_turn_button_system,
                highlight_event_system,
                unit_event_system,
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
        children![(
            Button,
            EndTurnButton,
            Node {
                width: Val::Px(128.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::WHITE),
            children![(
                Text::new("End turn"),
                TextFont {
                    //font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::BLACK),
            )]
        )],
    ));
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
fn unit_event_system(mut events: EventReader<UnitEvent>, mut units: Query<(&mut Sprite, &Unit)>) {
    for event in events.read() {
        match event {
            UnitEvent::ReadyToMove(unit_id) => {
                for (mut sprite, Unit(uid)) in units.iter_mut() {
                    if unit_id == uid {
                        sprite.color = Color::WHITE;
                    }
                }
            }
            UnitEvent::Moved(unit_id) => {
                for (mut sprite, Unit(uid)) in units.iter_mut() {
                    if unit_id == uid {
                        sprite.color = Color::srgba(0.7, 0.7, 0.7, 0.9);
                    }
                }
            }
        }
    }
}
fn highlight_event_system(
    mut events: EventReader<HighlightEvent>,
    mut tiles: Query<(&mut Sprite, &Tile), Without<Prop>>,
    mut props: Query<(&mut Sprite, &Prop), Without<Tile>>,
) {
    for event in events.read() {
        match event {
            HighlightEvent::Nothing => {
                info!("Highlight nothing");
                for (mut sprite, _) in tiles.iter_mut() {
                    sprite.color = Color::WHITE;
                }
                for (mut sprite, _) in props.iter_mut() {
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
        }
    }
}
fn tile_click_observer(
    trigger: Trigger<Pointer<Click>>,
    tile_query: Query<&Tile>,
    game: ResMut<Game>,
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
    info!("{tile:?}");
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
                        *state = MapInteractionState::SelectDestination(unit_id, destinations);
                    }
                }
            }
        }
        MapInteractionState::SelectDestination(unit_id, ref destinations) => {
            info!("Moving unit {unit_id} to {},{}", tile.x, tile.y);
            if let Some(path) = destinations.get(&wars::game::Position(tile.x, tile.y)) {
                wars::game::action::move_and_wait(
                    &mut game.into_inner().0,
                    unit_id,
                    &path,
                    &mut |e| event_processor.queue.push_back(e),
                )
                .expect("Could not move unit");
            }
            highlight_event_writer.write(HighlightEvent::Nothing);
            *state = MapInteractionState::Normal;
        }
    };
}
fn unit_bundle(
    unit_id: wars::game::UnitId,
    unit: &wars::game::Unit,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let theme_unit = theme.unit(unit).unwrap();
    (
        Unit(unit_id),
        Sprite::from_atlas_image(
            sprite_sheet.texture.clone(),
            TextureAtlas {
                layout: sprite_sheet.layout.clone(),
                index: theme_unit.unit_index,
            },
        ),
    )
}

fn tile_bundle(
    tile_id: wars::game::TileId,
    tile: &wars::game::Tile,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let theme_tile = theme.tile(tile).unwrap();
    (
        Tile(tile_id),
        Sprite::from_atlas_image(
            sprite_sheet.texture.clone(),
            TextureAtlas {
                layout: sprite_sheet.layout.clone(),
                index: theme_tile.tile_index,
            },
        ),
    )
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
        Sprite::from_atlas_image(
            sprite_sheet.texture.clone(),
            TextureAtlas {
                layout: sprite_sheet.layout.clone(),
                index: theme_tile.prop_index.unwrap(),
            },
        ),
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
    mut players: Query<&AnimationPlayer>,
    mut top_bar_colors: Query<&mut BackgroundColor, With<TopBar>>,
    mut unit_event_writer: EventWriter<UnitEvent>,
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
                    None
                } else {
                    Some(EventProcess::Animation(entity))
                }
            }
        }
    } else {
        None
    };

    if ep.state.is_none() {
        if let Some(event) = ep.queue.pop_front() {
            use wars::game::Event::*;
            ep.state = match event {
                StartTurn(player_number) => {
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
                    None
                }
                EndTurn(player_number) => {
                    for (unit_id, _unit) in game.units.owned_by_player(player_number) {
                        unit_event_writer.write(UnitEvent::ReadyToMove(unit_id));
                    }
                    None
                }
                //Funds(player_number, credits) => None,
                //UnitRepair(unit_id, health) => None,
                //WinGame(player_number) => None,
                //Surrender(player_number) => None,
                Move(unit_id, path) => {
                    let unit = units
                        .iter()
                        .find_map(|(entity, Unit(uid))| (*uid == unit_id).then_some(entity))
                        .unwrap();
                    let mut unit = commands.entity(unit);
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
                    Some(EventProcess::Animation(unit.id()))
                }
                Wait(unit_id) => {
                    unit_event_writer.write(UnitEvent::Moved(unit_id));
                    None
                }
                //Attack(attacking_unit_id, target_unit_id, health) => None,
                //Counterattack(attacking_unit_id, target_unit_id, health) => None,
                //Destroyed(attacking_unit_id, target_unit_id) => None,
                //Deploy(unit_id) => None,
                //Undeploy(unit_id) => None,
                //Load(loaded_unit_id, loading_unit_id) => None,
                //Unload(unloading_unit_id, unloaded_unit_id, position) => None,
                //Capture(unit_id, tile_id, capture_points) => None,
                //Captured(unit_id, tile_id) => None,
                //Build(tile_id, unit_id, unit_type, credits) => None,
                //TileCapturePointRegen(tile_id, capture_points) => None,
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
