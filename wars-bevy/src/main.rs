use bevy::prelude::*;
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
    pub queue: Vec<wars::game::Event>,
}

fn main() {
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/third_party.json");
    const THEME_JSON: &str = include_str!("../assets/settings.json");
    let map = wars::game::Map::from_json(THIRD_PARTY_MAP).unwrap();
    let game = wars::game::Game::new(map, &[0, 1]);
    let theme: theme::Theme = theme::Theme::from_json(THEME_JSON).unwrap();
    let queue = vec![wars::game::Event::Move(
        222,
        vec![
            wars::game::Position(0, 1),
            wars::game::Position(1, 1),
            wars::game::Position(3, 1),
            wars::game::Position(5, 5),
        ],
    )];
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Game(game))
        .insert_resource(Theme(theme))
        .insert_resource(SpriteSheet::default())
        .insert_resource(EventProcessor {
            queue,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (input, event_processor))
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
        let (cx, cy) = theme.map_hex_center(center_x, center_y);
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
            let (tx, ty) = theme.map_hex_center(tile.x, tile.y);
            let pos = Vec2::new(tx as f32, (ty - theme_tile.offset) as f32);
            let tile_sprite = commands
                .spawn((
                    tile_bundle(*tile_id, tile, &theme, &sprite_sheet),
                    Transform::from_xyz(pos.x, pos.y, -pos.y),
                ))
                .id();
            if let Some(prop_index) = theme_tile.prop_index {
                let (ox, oy) = theme.hex_sprite_center_offset();
                let tile_sprite = commands.spawn((
                    prop_bundle(*tile_id, tile, &theme, &sprite_sheet),
                    ChildOf(tile_sprite),
                    Transform::from_xyz(ox as f32, oy as f32, 0.0),
                ));
            }
            if let Some(unit_id) = tile.unit {
                let (ox, oy) = theme.hex_sprite_center_offset();
                let unit = game.units.get_ref(&unit_id).unwrap();
                let theme_unit = theme.unit(unit).unwrap();
                commands.spawn((
                    unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                    Transform::from_xyz(pos.x + ox as f32, pos.y + oy as f32, -pos.y + 0.1),
                ));
            }
        }
    }
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

fn input(
    mut camera_query: Single<(&mut Camera, &mut Transform, &mut Projection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
) {
    let (camera, mut transform, mut projection) = camera_query.into_inner();
    if mouse_buttons.pressed(MouseButton::Left) {
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

fn event_processor(
    mut commands: Commands,
    mut ep: ResMut<EventProcessor>,
    theme: Res<Theme>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut units: Query<(Entity, &Unit)>,
    mut players: Query<&AnimationPlayer>,
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

    use bevy::animation::{AnimationTarget, AnimationTargetId, animated_field};
    if ep.state.is_none() {
        if let Some(event) = ep.queue.pop() {
            use wars::game::Event::*;
            ep.state = match event {
                Move(unit_id, path) => {
                    let unit = units
                        .iter()
                        .find_map(|(entity, Unit(uid))| (*uid == unit_id).then_some(entity))
                        .unwrap();
                    let mut unit = commands.entity(unit);
                    let mut animation = AnimationClip::default();
                    let target_name = Name::new("unit");
                    let animation_target_id = AnimationTargetId::from_name(&target_name);
                    let waypoints: Vec<Vec3> = path
                        .into_iter()
                        .map(|wars::game::Position(x, y)| {
                            let (x, y) = theme.map_hex_center(x, y);
                            Vec3::new(x as f32, y as f32, -y as f32)
                        })
                        .collect();
                    info!("waypoints: {waypoints:?}");
                    animation.add_curve_to_target(
                        animation_target_id,
                        AnimatableCurve::new(
                            animated_field!(Transform::translation),
                            SampleAutoCurve::new(Interval::new(0.0, 3.0).unwrap(), waypoints)
                                .unwrap(),
                        ),
                    );
                    let (graph, animation_index) =
                        AnimationGraph::from_clip(animations.add(animation));
                    let mut animation_player = AnimationPlayer::default();
                    animation_player.play(animation_index);
                    unit.insert(target_name);
                    unit.insert(AnimationGraphHandle(graphs.add(graph)));
                    unit.insert(animation_player);
                    unit.insert(AnimationTarget {
                        id: animation_target_id,
                        player: unit.id(),
                    });
                    Some(EventProcess::Animation(unit.id()))
                }
                e => Some(EventProcess::NoOp(e)),
            };
        }
    }
}
