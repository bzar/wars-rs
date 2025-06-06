use bevy::prelude::*;
use interaction_state::InteractionEvent;
use std::{
    collections::{HashSet, VecDeque},
    ops::DerefMut,
};
use wars::model::UNIT_MAX_HEALTH;

mod theme;

#[derive(Resource, Deref, DerefMut)]
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
    fn image(&self, index: usize) -> ImageNode {
        ImageNode::from_atlas_image(
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
struct Carrier {
    load: u32,
    capacity: u32,
}

#[derive(Component)]
struct CarrierSlot(u32);

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
    fn from_value(health: u32) -> Self {
        if health >= UNIT_MAX_HEALTH {
            Self::Full
        } else {
            Self::Damaged(health)
        }
    }
    fn value(&self) -> u32 {
        match self {
            Self::Full => UNIT_MAX_HEALTH,
            Self::Damaged(health) => *health,
        }
    }
    fn damage(&self, x: u32) -> Self {
        if x > self.value() {
            Self::Damaged(0)
        } else {
            Self::Damaged(self.value() - x)
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

#[derive(Component, Default, Deref, DerefMut)]
struct UnloadMenu(Vec<wars::game::UnitId>);

#[derive(Component, Default)]
struct UnloadMenuItem(wars::game::UnitId);

#[derive(Component)]
struct Funds(u32);

impl Funds {
    fn deduct(&self, amount: u32) -> Self {
        Self(self.0.saturating_sub(amount))
    }
}
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MapAction {
    Wait,
    Attack,
    Capture,
    Deploy,
    Undeploy,
    Load,
    Unload,
    Cancel,
}

#[derive(Resource, Default, Deref, DerefMut)]
struct VisibleActionButtons(HashSet<MapAction>);

#[derive(Component)]
struct BuildMenu {
    price_limit: u32,
    unit_classes: HashSet<wars::model::UnitClass>,
}

#[derive(Component)]
struct DisabledButton;

#[derive(Component)]
struct BuildItem(wars::model::UnitType);

#[derive(Resource, Eq, PartialEq)]
enum InputLayer {
    UI,
    Game,
}
mod camera;
mod interaction_state;
mod map;
mod ui;

fn main() {
    const THIRD_PARTY_MAP: &str = include_str!("../../data/maps/my-awesome-map.json");
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
        .insert_resource(SpriteSheet::default())
        .insert_resource(event_processor)
        .insert_resource(VisibleActionButtons::default())
        .insert_resource(InputLayer::Game)
        .add_plugins((
            camera::CameraPlugin,
            map::MapPlugin,
            ui::UIPlugin,
            interaction_state::InteractionStatePlugin,
        ))
        .add_systems(PreStartup, setup)
        .add_systems(Update, (event_processor_system, interaction_event_system))
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
    animation_params: (
        ResMut<Assets<AnimationClip>>,
        ResMut<Assets<AnimationGraph>>,
        Query<&AnimationPlayer>,
    ),
    unit_queries: (
        Query<(Entity, &Unit)>,
        Query<&mut Moved, With<Unit>>,
        Query<&mut Deployed, With<Unit>>,
        Query<&mut Health, With<Unit>>,
        Query<&mut Carrier, With<Unit>>,
    ),
    tile_queries: (
        Query<(Entity, &Tile)>,
        Query<&mut Owner, With<Tile>>,
        Query<&mut CaptureState, With<Tile>>,
    ),
    mut funds: Query<&mut Funds>,
    mut top_bar_colors: Query<&mut BackgroundColor, With<MenuBar>>,
    sprite_sheet: Res<SpriteSheet>,
) {
    // These are in tuples due to Bevy's system parameter limit
    let (units, mut unit_moveds, mut unit_deployeds, mut unit_healths, mut carriers) = unit_queries;
    let (tiles, mut tile_owners, mut tile_capture_states) = tile_queries;
    ep.state = if let Some(state) = ep.state.take() {
        match state {
            EventProcess::NoOp(event) => {
                info!("Skipping event {event:?}");
                None
            }
            EventProcess::Animation(entity) => {
                let (_, _, players) = animation_params;
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
                Event::UnitRepair(unit_id, health) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let mut unit_health = unit_healths.get_mut(unit_entity_id).unwrap();
                    *unit_health = Health::from_value(health);
                    None
                }
                //Event::WinGame(player_number) => None,
                //Event::Surrender(player_number) => None,
                Event::Move(unit_id, path) => {
                    if path.len() > 1 {
                        let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                        let mut unit = commands.entity(unit_entity_id);
                        let (mut animations, mut graphs, _) = animation_params;
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
                Event::Attack(attacking_unit_id, target_unit_id, health) => {
                    let target_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    let attacking_entity_id = find_unit_entity_id(attacking_unit_id).unwrap();
                    let mut target_health = unit_healths.get_mut(target_entity_id).unwrap();
                    let mut moved = unit_moveds.get_mut(attacking_entity_id).unwrap();
                    *moved = Moved(true);
                    *target_health = target_health.damage(health);
                    None
                }
                Event::Counterattack(_attacking_unit_id, target_unit_id, health) => {
                    let target_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    let mut target_health = unit_healths.get_mut(target_entity_id).unwrap();
                    *target_health = target_health.damage(health);
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
                    let mut moved = unit_moveds.get_mut(unit_entity_id).unwrap();
                    *deployed = Deployed(true);
                    *moved = Moved(true);
                    None
                }
                Event::Undeploy(unit_id) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let mut deployed = unit_deployeds.get_mut(unit_entity_id).unwrap();
                    let mut moved = unit_moveds.get_mut(unit_entity_id).unwrap();
                    *deployed = Deployed(false);
                    *moved = Moved(true);
                    None
                }
                Event::Load(unit_id, carrier_id) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let carrier_entity_id = find_unit_entity_id(carrier_id).unwrap();
                    commands.entity(unit_entity_id).despawn();

                    carriers.get_mut(carrier_entity_id).unwrap().load += 1;
                    None
                }
                Event::Unload(carrier_id, unit_id, position) => {
                    let (_tile_id, tile) = game.tiles.get_at(&position).unwrap();
                    let theme_tile = theme.tile(&tile).unwrap();
                    let (tx, ty, tz) = theme.map_hex_center(tile.x, tile.y);
                    let unit = game.units.get_ref(&unit_id).unwrap();
                    let pos = Vec2::new(tx as f32, (ty - theme_tile.offset) as f32);
                    let (ox, oy) = theme.hex_sprite_center_offset();
                    commands
                        .spawn((
                            map::unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                            Transform::from_xyz(
                                pos.x + ox as f32,
                                pos.y + oy as f32,
                                tz as f32 + 1.5,
                            ),
                        ))
                        .insert(Moved(true));
                    let carrier_entity_id = find_unit_entity_id(carrier_id).unwrap();
                    carriers.get_mut(carrier_entity_id).unwrap().load -= 1;
                    let mut carrier_moved = unit_moveds.get_mut(carrier_entity_id).unwrap();
                    *carrier_moved = Moved(true);
                    None
                }
                Event::Capture(unit_id, tile_id, capture_points) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let tile_entity_id = find_tile_entity_id(tile_id).unwrap();
                    let mut moved = unit_moveds.get_mut(unit_entity_id).unwrap();
                    let mut capture_status = tile_capture_states.get_mut(tile_entity_id).unwrap();
                    *moved = Moved(true);
                    *capture_status = CaptureState::Capturing(capture_points);
                    None
                }
                Event::Captured(unit_id, tile_id, player_number) => {
                    let unit_entity_id = find_unit_entity_id(unit_id).unwrap();
                    let tile_entity_id = find_tile_entity_id(tile_id).unwrap();
                    let mut moved = unit_moveds.get_mut(unit_entity_id).unwrap();
                    let mut owner = tile_owners.get_mut(tile_entity_id).unwrap();
                    let mut capture_status = tile_capture_states.get_mut(tile_entity_id).unwrap();
                    *moved = Moved(true);
                    *owner = Owner(player_number.unwrap_or(0));
                    *capture_status = CaptureState::Recovering(1);
                    None
                }
                Event::Build(tile_id, unit_id, _unit_type, credits) => {
                    let tile = game.tiles.get(tile_id).unwrap();
                    let theme_tile = theme.tile(&tile).unwrap();
                    let (tx, ty, tz) = theme.map_hex_center(tile.x, tile.y);
                    let pos = Vec2::new(tx as f32, (ty - theme_tile.offset) as f32);
                    let (ox, oy) = theme.hex_sprite_center_offset();
                    let unit = game.units.get_ref(&unit_id).unwrap();
                    commands.spawn((
                        map::unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                        Transform::from_xyz(pos.x + ox as f32, pos.y + oy as f32, tz as f32 + 1.5),
                    ));
                    for mut fund in funds.iter_mut() {
                        *fund = fund.deduct(credits);
                    }
                    None
                }
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

fn interaction_event_system(
    mut events: EventReader<InteractionEvent>,
    mut event_processor: ResMut<EventProcessor>,
    mut game: ResMut<Game>,
    mut visible_action_buttons: ResMut<VisibleActionButtons>,
    mut unit_highlights: Query<(&Unit, &mut UnitHighlight)>,
    mut tile_highlights: Query<(&Tile, &mut TileHighlight)>,
    mut build_menus: Query<(&mut BuildMenu, &mut Visibility)>,
    mut unload_menus: Query<&mut UnloadMenu>,
) {
    for event in events.read() {
        info!("Interaction event: {event:?}");

        let mut event_handler = |e| event_processor.queue.push_back(e);
        match *event {
            InteractionEvent::MoveAndWait(unit_id, ref path) => {
                visible_action_buttons.clear();
                wars::game::action::move_and_wait(
                    game.deref_mut(),
                    unit_id,
                    &path,
                    &mut event_handler,
                )
                .expect("Could not move unit");
            }
            InteractionEvent::MoveAndAttack(unit_id, ref path, target_id) => {
                visible_action_buttons.clear();
                for (_, mut highlight) in unit_highlights.iter_mut() {
                    *highlight = UnitHighlight::Normal;
                }
                wars::game::action::move_and_attack(
                    game.deref_mut(),
                    unit_id,
                    &path,
                    target_id,
                    &mut event_handler,
                )
                .expect("Could not attack");
            }
            InteractionEvent::MoveAndCapture(unit_id, ref path) => {
                visible_action_buttons.clear();
                wars::game::action::move_and_capture(
                    game.deref_mut(),
                    unit_id,
                    &path,
                    &mut event_handler,
                )
                .expect("Could not capture tile");
            }
            InteractionEvent::MoveAndDeploy(unit_id, ref path) => {
                visible_action_buttons.clear();
                wars::game::action::move_and_deploy(
                    game.deref_mut(),
                    unit_id,
                    &path,
                    &mut event_handler,
                )
                .expect("Could not deploy unit");
            }
            InteractionEvent::Undeploy(unit_id) => {
                visible_action_buttons.clear();
                wars::game::action::undeploy(game.deref_mut(), unit_id, &mut event_handler)
                    .expect("Could not undeploy unit");
            }
            InteractionEvent::MoveAndLoadInto(unit_id, ref path) => {
                visible_action_buttons.clear();
                wars::game::action::move_and_load_into(
                    game.deref_mut(),
                    unit_id,
                    &path,
                    &mut event_handler,
                )
                .expect("Could not load into unit");
            }
            InteractionEvent::MoveAndUnloadUnitTo(carrier_id, ref path, unit_id, position) => {
                visible_action_buttons.clear();
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
                wars::game::action::move_and_unload(
                    game.deref_mut(),
                    carrier_id,
                    &path,
                    unit_id,
                    position,
                    &mut event_handler,
                )
                .expect("Could not unload carried unit");
            }
            InteractionEvent::SelectDestination(ref options) => {
                *visible_action_buttons = VisibleActionButtons([MapAction::Cancel].into());
                for (Tile(tile_id), mut highlight) in tile_highlights.iter_mut() {
                    let tile = game.tiles.get(*tile_id).unwrap();
                    *highlight = if options.contains(&tile.position()) {
                        TileHighlight::Movable
                    } else {
                        TileHighlight::Unmovable
                    };
                }
            }
            InteractionEvent::CancelSelectDestination => {
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
            }
            InteractionEvent::SelectAction(ref options) => {
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
                *visible_action_buttons = VisibleActionButtons(options.clone());
            }
            InteractionEvent::SelectAttackTarget(ref options) => {
                *visible_action_buttons = VisibleActionButtons([MapAction::Cancel].into());
                for (Unit(uid), mut highlight) in unit_highlights.iter_mut() {
                    *highlight = if options.contains(&uid) {
                        UnitHighlight::Target
                    } else {
                        UnitHighlight::Normal
                    };
                }
            }
            InteractionEvent::SelectUnloadUnit(ref options) => {
                *visible_action_buttons = VisibleActionButtons([MapAction::Cancel].into());
                let mut menu = unload_menus.single_mut().unwrap();
                *menu = UnloadMenu(options.clone());
            }
            InteractionEvent::SelectUnloadDestination(ref options) => {
                *visible_action_buttons = VisibleActionButtons([MapAction::Cancel].into());
                unload_menus.single_mut().unwrap().clear();
                for (Tile(tile_id), mut highlight) in tile_highlights.iter_mut() {
                    let tile = game.tiles.get(*tile_id).unwrap();
                    *highlight = if options.contains(&tile.position()) {
                        TileHighlight::Movable
                    } else {
                        TileHighlight::Unmovable
                    };
                }
            }
            InteractionEvent::SelectUnitToBuild(ref unit_classes) => {
                *visible_action_buttons = VisibleActionButtons([MapAction::Cancel].into());
                let (mut build_menu, mut visibility) =
                    build_menus.single_mut().expect("Build menu does not exist");
                *visibility = Visibility::Inherited;
                build_menu.price_limit = game.in_turn_player().unwrap().funds;
                build_menu.unit_classes = unit_classes.clone();
            }
            InteractionEvent::BuildUnit(tile_id, unit_type) => {
                let tile = game.tiles.get(tile_id).expect("Tile does not exist");
                wars::game::action::build(
                    game.deref_mut(),
                    tile.position(),
                    unit_type,
                    &mut event_handler,
                )
                .expect("Could not build unit");
                build_menus
                    .iter_mut()
                    .for_each(|(_, mut v)| *v = Visibility::Hidden);
            }
            InteractionEvent::CancelSelectUnitToBuild => {
                build_menus
                    .iter_mut()
                    .for_each(|(_, mut v)| *v = Visibility::Hidden);
            }
            InteractionEvent::CancelSelectAction => {
                visible_action_buttons.clear();
            }
            InteractionEvent::CancelSelectAttackTarget => {
                for (_, mut highlight) in unit_highlights.iter_mut() {
                    *highlight = UnitHighlight::Normal;
                }
            }
            InteractionEvent::CancelSelectUnloadUnit => {
                unload_menus.single_mut().unwrap().clear();
            }
            InteractionEvent::CancelSelectUnloadDestination => {
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
            }
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
                Interval::new(0.0, 0.1 * waypoints.len() as f32).unwrap(),
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
