use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use wars::model::UNIT_MAX_HEALTH;

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

#[derive(Component)]
struct DeployEmblem;

#[derive(Component)]
struct Deployed(bool);

#[derive(Component)]
struct Moved(bool);

#[derive(Component)]
enum CaptureState {
    Capturing(u32),
    Recovering(u32),
    Full,
}

#[derive(Component)]
struct CaptureBar;

#[derive(Component)]
struct CaptureBarBit(u32);

#[derive(Component)]
enum TileHighlight {
    Normal,
    Unmovable,
    Movable,
}

#[derive(Component)]
enum UnitHighlight {
    Normal,
    Target,
}

#[derive(Component)]
enum Health {
    Full,
    Damaged(u32),
}

impl Health {
    fn value(&self) -> u32 {
        match self {
            Self::Full => UNIT_MAX_HEALTH,
            Self::Damaged(health) => *health,
        }
    }
    fn damage(&self, x: u32) -> Self {
        if x > self.value() {
            return Self::Damaged(0);
        } else {
            return Self::Damaged(self.value() - x);
        }
    }
}
#[derive(Component)]
struct OnesDigit;
#[derive(Component)]
struct TensDigit;

#[derive(Component)]
struct Owner(u32);

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
struct MenuBar;

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

#[derive(Resource, Default, Deref, DerefMut)]
struct VisibleActionButtons(HashSet<MapAction>);

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

mod camera;
mod map;
mod ui;

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
        .insert_resource(VisibleActionButtons::default())
        .add_plugins((camera::CameraPlugin, ui::UIPlugin, map::MapPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (event_processor_system,))
        .run();
}

fn setup(
    theme: Res<Theme>,
    mut sprite_sheet: ResMut<SpriteSheet>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
) {
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
}

fn event_processor_system(
    mut commands: Commands,
    mut ep: ResMut<EventProcessor>,
    game: Res<Game>,
    theme: Res<Theme>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    units: Query<(Entity, &Unit)>,
    tiles: Query<(Entity, &Tile)>,
    mut unit_moveds: Query<&mut Moved, With<Unit>>,
    mut unit_deployeds: Query<&mut Deployed, With<Unit>>,
    mut unit_healths: Query<&mut Health, With<Unit>>,
    mut tile_owners: Query<&mut Owner, With<Tile>>,
    mut tile_capture_states: Query<&mut CaptureState, With<Tile>>,
    mut funds: Query<&mut Funds>,
    players: Query<&AnimationPlayer>,
    mut top_bar_colors: Query<&mut BackgroundColor, With<MenuBar>>,
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
    let find_tile_entity_id = |tile_id: wars::game::TileId| {
        tiles
            .iter()
            .find_map(|(entity_id, Tile(tid))| (*tid == tile_id).then_some(entity_id))
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
                Event::EndTurn(_player_number) => {
                    for mut moved in unit_moveds.iter_mut() {
                        *moved = Moved(false);
                    }
                    None
                }
                Event::Funds(player_number, _credits) => {
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
                    let mut moved = unit_moveds.get_mut(unit_entity_id).unwrap();
                    *moved = Moved(true);
                    None
                }
                Event::Attack(_attacking_unit_id, target_unit_id, health) => {
                    let unit_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    let mut unit_health = unit_healths.get_mut(unit_entity_id).unwrap();
                    *unit_health = unit_health.damage(health);
                    None
                }
                Event::Counterattack(attacking_unit_id, target_unit_id, health) => {
                    let unit_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    let mut unit_health = unit_healths.get_mut(unit_entity_id).unwrap();
                    *unit_health = unit_health.damage(health);
                    None
                }
                Event::Destroyed(_attacking_unit_id, target_unit_id) => {
                    let unit_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    commands.entity(unit_entity_id).despawn();

                    None
                }
                Event::Deploy(unit_id) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let mut deployed = unit_deployeds.get_mut(unit_entity_id).unwrap();
                    *deployed = Deployed(true);
                    None
                }
                Event::Undeploy(unit_id) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let mut deployed = unit_deployeds.get_mut(unit_entity_id).unwrap();
                    *deployed = Deployed(false);
                    None
                }
                //Event::Load(loaded_unit_id, loading_unit_id) => None,
                //Event::Unload(unloading_unit_id, unloaded_unit_id, position) => None,
                Event::Capture(_unit_id, tile_id, capture_points) => {
                    let tile_entity_id = find_tile_entity_id(tile_id).unwrap();
                    let mut capture_status = tile_capture_states.get_mut(tile_entity_id).unwrap();
                    *capture_status = CaptureState::Capturing(capture_points);
                    None
                }
                Event::Captured(_unit_id, tile_id, player_number) => {
                    let tile_entity_id = find_tile_entity_id(tile_id).unwrap();
                    let mut owner = tile_owners.get_mut(tile_entity_id).unwrap();
                    *owner = Owner(player_number.unwrap_or(0));
                    let mut capture_status = tile_capture_states.get_mut(tile_entity_id).unwrap();
                    *capture_status = CaptureState::Recovering(1);
                    None
                }
                //Event::Build(tile_id, unit_id, unit_type, credits) => None,
                Event::TileCapturePointRegen(tile_id, capture_points) => {
                    let tile_entity_id = find_tile_entity_id(tile_id).unwrap();
                    let mut capture_status = tile_capture_states.get_mut(tile_entity_id).unwrap();
                    *capture_status = if capture_points == wars::model::MAX_CAPTURE_POINTS {
                        CaptureState::Full
                    } else {
                        CaptureState::Recovering(capture_points)
                    };
                    None
                }
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
