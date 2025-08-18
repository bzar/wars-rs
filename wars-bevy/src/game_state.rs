use crate::connection::Connection;
use crate::interaction_state::InteractionEvent;
use bevy::prelude::*;

use crate::interaction_state::InteractionState;

use crate::{AppState, animation, bot, components::*, map, resources::*, theme};

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        const THEME_JSON: &str = include_str!("../assets/settings.json");
        let theme: theme::Theme = theme::Theme::from_json(THEME_JSON).unwrap();
        let visualizer = Visualizer::default();

        app.insert_resource(Game::None)
            .insert_resource(Theme(theme))
            .insert_resource(SpriteSheet::default())
            .insert_resource(visualizer)
            .insert_resource(VisibleActionButtons::default())
            .insert_resource(InputLayer::Game)
            .insert_resource(InTurnPlayer(None))
            .insert_resource(VisibleActionMenu(None))
            .insert_resource(VisibleBuildMenu(None))
            .insert_resource(VisibleUnloadMenu(None))
            .add_event::<InputEvent>()
            .add_event::<GameAction>()
            .add_event::<GameEvent>()
            .add_plugins((
                crate::camera::CameraPlugin,
                crate::map::MapPlugin,
                crate::ui::UIPlugin,
                crate::interaction_state::InteractionStatePlugin,
                crate::animation::SpriteAnimationPlugin,
            ))
            .add_systems(Startup, setup)
            .add_systems(OnEnter(AppState::LoadGame), on_enter_load_game)
            .add_systems(
                Update,
                (
                    visualizer_system,
                    interaction_event_system,
                    interaction_state_init_system,
                    bot_system,
                    remote_game_system,
                    action_dispatch_system,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
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

fn on_enter_load_game(
    mut game: ResMut<Game>,
    mut next_state: ResMut<NextState<AppState>>,
    mut game_events: EventWriter<GameEvent>,
) {
    let Game::PreGame(map, players, game_id) = game.as_ref() else {
        panic!("Entered game without pregame");
    };
    let mut player_numbers = players.keys().map(|&pn| (pn, 0)).collect::<Vec<_>>();
    player_numbers.sort();
    let mut state = wars::game::Game::new(map.clone(), &player_numbers);

    if game_id.is_none() {
        // Local game
        wars::game::action::start(&mut state, &mut |event| {
            game_events.write(GameEvent(event));
        })
        .expect("Could not start game");
    }

    *game = Game::InGame(state, players.clone(), game_id.clone());
    next_state.set(AppState::InGame);
}
fn bot_system(
    mut game: ResMut<Game>,
    visualizer: Res<Visualizer>,
    mut event_writer: EventWriter<GameEvent>,
) {
    if !visualizer.state.is_none()
        || !visualizer.queue.is_empty()
        || game.in_turn() != Some(&Player::Bot)
    {
        return;
    }
    let Game::InGame(state, ..) = game.as_mut() else {
        return;
    };
    let mut enqueue_event = move |e| {
        event_writer.write(GameEvent(e));
    };

    info!("Running bot system");
    bot::random_bot(state, &mut enqueue_event).expect("Bot made an ActionError");
}
fn visualizer_system(
    mut commands: Commands,
    mut visualizer: ResMut<Visualizer>,
    game: Res<Game>,
    theme: Res<Theme>,
    mut in_turn_player: ResMut<InTurnPlayer>,
    sprite_animations: Query<&animation::SpriteAnimation>,
    unit_queries: (
        Query<(Entity, &Unit)>,
        Query<&mut Moved, With<Unit>>,
        Query<&mut Deployed, With<Unit>>,
        Query<&mut Health, With<Unit>>,
        Query<&mut Carrier, With<Unit>>,
    ),
    transforms: Query<&Transform>,
    tile_queries: (
        Query<(Entity, &Tile)>,
        Query<&mut Owner, With<Tile>>,
        Query<&mut CaptureState, With<Tile>>,
    ),
    mut funds: Query<&mut Funds>,
    mut top_bar_colors: Query<&mut BackgroundColor, With<MenuBar>>,
    sprite_sheet: Res<SpriteSheet>,
    mut event_reader: EventReader<GameEvent>,
) {
    let Game::InGame(state, ..) = game.as_ref() else {
        return;
    };

    // These are in tuples due to Bevy's system parameter limit
    let (units, mut unit_moveds, mut unit_deployeds, mut unit_healths, mut carriers) = unit_queries;
    let (tiles, mut tile_owners, mut tile_capture_states) = tile_queries;

    // Enqueue new game events to visualization queue
    for GameEvent(e) in event_reader.read() {
        visualizer.queue.push_back(e.clone());
    }

    visualizer.state = visualizer.state.take().and_then(|state| {
        match state {
            EventProcess::NoOp(event) => {
                info!("Skipping event {event:?}");
                None
            }
            EventProcess::Animation(entity) => {
                // The animation has finished or the entity no longer exists
                if sprite_animations.get(entity).is_err() {
                    info!("Finished animation");
                    None
                } else {
                    Some(EventProcess::Animation(entity))
                }
            }
        }
    });

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
    while visualizer.state.is_none() && !visualizer.queue.is_empty() {
        if let Some(event) = visualizer.queue.pop_front() {
            info!("Game event: {event:?}");
            use wars::game::Event;
            visualizer.state = match event {
                Event::StartTurn(player_number) => {
                    *in_turn_player = InTurnPlayer(Some(player_number));
                    if let Some(player_color) = theme.spec.player_colors.get(player_number as usize)
                    {
                        for mut top_bar_color in top_bar_colors.iter_mut() {
                            top_bar_color.0 = player_color.into();
                        }
                    }

                    if let Some(player) = state.get_player(player_number) {
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
                    if let Some(player) = state.get_player(player_number) {
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
                        let waypoints = path
                            .into_iter()
                            .map(|pos| state.tiles.get_at(&pos).expect("No such tile"))
                            .map(|(_tile_id, tile)| theme.unit_position(&tile));
                        animation::animate_move_unit(&mut commands, unit_entity_id, waypoints);
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

                    let attacker_position =
                        transforms.get(attacking_entity_id).unwrap().translation;
                    let target_position = transforms.get(target_entity_id).unwrap().translation;
                    animation::animate_attack(
                        &mut commands,
                        attacking_entity_id,
                        attacker_position,
                        target_position,
                        theme.spec.hex.height as f32,
                    );
                    Some(EventProcess::Animation(attacking_entity_id))
                }
                Event::Counterattack(attacking_unit_id, target_unit_id, health) => {
                    let attacking_entity_id = find_unit_entity_id(attacking_unit_id).unwrap();
                    let target_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    let mut target_health = unit_healths.get_mut(target_entity_id).unwrap();
                    *target_health = target_health.damage(health);

                    let attacker_position =
                        transforms.get(attacking_entity_id).unwrap().translation;
                    let target_position = transforms.get(target_entity_id).unwrap().translation;
                    animation::animate_attack(
                        &mut commands,
                        attacking_entity_id,
                        attacker_position,
                        target_position,
                        theme.spec.hex.height as f32,
                    );
                    Some(EventProcess::Animation(attacking_entity_id))
                }
                Event::Destroyed(_attacking_unit_id, target_unit_id) => {
                    let unit_entity_id = find_unit_entity_id(target_unit_id).unwrap();
                    animation::animate_destroy(&mut commands, unit_entity_id);
                    Some(EventProcess::Animation(unit_entity_id))
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
                    let (_tile_id, tile) = state.tiles.get_at(&position).unwrap();
                    let unit = state.units.get_ref(&unit_id).unwrap();
                    commands
                        .spawn((
                            map::unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                            Transform::from_translation(theme.unit_position(&tile)),
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

                    let unit_position = transforms.get(unit_entity_id).unwrap().translation;
                    animation::animate_capturing(&mut commands, unit_entity_id, unit_position);
                    Some(EventProcess::Animation(unit_entity_id))
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

                    let unit_position = transforms.get(unit_entity_id).unwrap().translation;
                    animation::animate_captured(&mut commands, unit_entity_id, unit_position);
                    Some(EventProcess::Animation(unit_entity_id))
                }
                Event::Build(tile_id, unit_id, _unit_type, credits) => {
                    let tile = state.tiles.get(tile_id).unwrap();
                    let unit = state.units.get_ref(&unit_id).unwrap();
                    commands.spawn((
                        map::unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                        Transform::from_translation(theme.unit_position(&tile)),
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
            }
        }
    }
}

fn interaction_state_init_system(
    mut game_events: EventReader<GameEvent>,
    mut interaction_state: ResMut<InteractionState>,
    game: Res<Game>,
) {
    let Game::InGame(game, players, game_id) = game.as_ref() else {
        return;
    };

    for GameEvent(event) in game_events.read() {
        info!(
            "interaction_game_init_system: event = {event:?}, players = {players:?}, game_id = {game_id:?}"
        );
        match event {
            wars::game::Event::StartTurn(player_number) => {
                if players.get(&player_number) == Some(&Player::Human) {
                    let s = &game.state;
                    let t = game.in_turn_index;
                    info!(
                        "Human player starting turn, initializing interaction state from state={s:?}, turn={t:?}"
                    );
                    *interaction_state = InteractionState::from_game(&game);
                }
            }
            _ => (),
        }
    }
}
fn interaction_event_system(
    mut commands: Commands,
    mut events: EventReader<InputEvent>,
    mut game_actions: EventWriter<GameAction>,
    mut interaction_state: ResMut<InteractionState>,
    mut game_res: ResMut<Game>,
    menus: (
        ResMut<VisibleActionMenu>,
        ResMut<VisibleBuildMenu>,
        ResMut<VisibleUnloadMenu>,
    ),
    mut unit_highlights: Query<(&Unit, &mut UnitHighlight)>,
    mut tile_highlights: Query<(&Tile, &mut TileHighlight)>,
    unit_entities: Query<(&Unit, Entity)>,
    entities_with_move_previews: Query<Entity, With<UnitMovePreview>>,
    mut tile_in_attack_ranges: Query<(&Tile, &mut InAttackRange)>,
    mut damage_indicators: Query<(&Unit, &mut DamageIndicator)>,
    mut action_menus: Query<(&mut Node, &mut Visibility), (With<ActionMenu>, Without<BuildMenu>)>,
) {
    let Game::InGame(game, players, _) = game_res.as_mut() else {
        return;
    };
    let (mut visible_action_menu, mut visible_build_menu, mut visible_unload_menu) = menus;

    let mut interaction_event_handler = |event, mut game: &mut wars::game::Game| {
        info!("Interaction event: {event:?}");
        match event {
            InteractionEvent::SelectUnitOrBase(_units, _tiles) => {
                entities_with_move_previews.iter().for_each(|entity| {
                    commands.entity(entity).remove::<UnitMovePreview>();
                });
            }
            InteractionEvent::MoveAndWait(unit_id, ref path) => {
                game_actions.write(GameAction(wars::game::Action::MoveAndWait(
                    unit_id,
                    path.clone(),
                )));
            }
            InteractionEvent::MoveAndAttack(unit_id, ref path, target_id) => {
                for (_, mut highlight) in unit_highlights.iter_mut() {
                    *highlight = UnitHighlight::Normal;
                }
                for (_, mut damage_indicator) in damage_indicators.iter_mut() {
                    *damage_indicator = DamageIndicator::Hidden;
                }
                for (_, mut in_attack_range) in tile_in_attack_ranges.iter_mut() {
                    *in_attack_range = InAttackRange(false);
                }
                game_actions.write(GameAction(wars::game::Action::MoveAndAttack(
                    unit_id,
                    path.clone(),
                    target_id,
                )));
            }
            InteractionEvent::MoveAndCapture(unit_id, ref path) => {
                game_actions.write(GameAction(wars::game::Action::MoveAndCapture(
                    unit_id,
                    path.clone(),
                )));
            }
            InteractionEvent::MoveAndDeploy(unit_id, ref path) => {
                game_actions.write(GameAction(wars::game::Action::MoveAndDeploy(
                    unit_id,
                    path.clone(),
                )));
            }
            InteractionEvent::Undeploy(unit_id) => {
                game_actions.write(GameAction(wars::game::Action::Undeploy(unit_id)));
            }
            InteractionEvent::MoveAndLoadInto(unit_id, ref path) => {
                game_actions.write(GameAction(wars::game::Action::MoveAndLoadInto(
                    unit_id,
                    path.clone(),
                )));
            }
            InteractionEvent::MoveAndUnloadUnitTo(carrier_id, ref path, unit_id, position) => {
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
                game_actions.write(GameAction(wars::game::Action::MoveAndUnload(
                    carrier_id,
                    path.clone(),
                    unit_id,
                    position,
                )));
            }
            InteractionEvent::SelectDestination(ref options) => {
                for (Tile(tile_id), mut highlight) in tile_highlights.iter_mut() {
                    let tile = game.tiles.get(*tile_id).unwrap();
                    *highlight = if options.contains(&tile.position()) {
                        TileHighlight::Movable
                    } else {
                        TileHighlight::Unmovable
                    };
                }
            }
            InteractionEvent::SelectedDestination(unit_id, ref path) => {
                unit_entities
                    .iter()
                    .find_map(|(Unit(uid), entity)| (unit_id == *uid).then_some(entity))
                    .map(|entity| {
                        commands
                            .entity(entity)
                            .remove::<UnitMovePreview>()
                            .insert(UnitMovePreview(path.clone()));
                    });
            }
            InteractionEvent::CancelSelectDestination => {
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
            }
            InteractionEvent::SelectAction(position, ref options, ref tiles_in_range) => {
                *visible_action_menu = VisibleActionMenu(Some((position, options.clone())));
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
                for (Tile(tid), mut in_attack_range) in tile_in_attack_ranges.iter_mut() {
                    *in_attack_range = InAttackRange(tiles_in_range.contains(tid))
                }
            }
            InteractionEvent::SelectedAction(_) => {
                action_menus
                    .iter_mut()
                    .for_each(|(_, mut v)| *v = Visibility::Hidden);
                for (_, mut in_attack_range) in tile_in_attack_ranges.iter_mut() {
                    *in_attack_range = InAttackRange(false);
                }
                *visible_action_menu = VisibleActionMenu(None);
            }
            InteractionEvent::SelectAttackTarget(ref options, ref tiles_in_range) => {
                for (Unit(uid), mut highlight) in unit_highlights.iter_mut() {
                    *highlight = if options.contains_key(&uid) {
                        UnitHighlight::Target
                    } else {
                        UnitHighlight::Normal
                    };
                }
                for (Unit(uid), mut damage_indicator) in damage_indicators.iter_mut() {
                    if let Some(damage) = options.get(uid) {
                        *damage_indicator = DamageIndicator::Visible(*damage);
                    }
                }
                for (Tile(tid), mut in_attack_range) in tile_in_attack_ranges.iter_mut() {
                    *in_attack_range = InAttackRange(tiles_in_range.contains(tid))
                }
            }
            InteractionEvent::SelectUnloadUnit(position, ref options) => {
                *visible_unload_menu = VisibleUnloadMenu(Some((position, options.clone())));
            }
            InteractionEvent::SelectUnloadDestination(ref options) => {
                *visible_unload_menu = VisibleUnloadMenu(None);
                for (Tile(tile_id), mut highlight) in tile_highlights.iter_mut() {
                    let tile = game.tiles.get(*tile_id).unwrap();
                    *highlight = if options.contains(&tile.position()) {
                        TileHighlight::Movable
                    } else {
                        TileHighlight::Unmovable
                    };
                }
            }
            InteractionEvent::SelectUnitToBuild(position, ref unit_classes) => {
                *visible_build_menu = VisibleBuildMenu(Some((
                    position,
                    unit_classes.clone(),
                    game.in_turn_number(),
                    game.in_turn_player().map(|p| p.funds).unwrap_or(0),
                )));
            }
            InteractionEvent::BuildUnit(tile_id, unit_type) => {
                let tile = game.tiles.get(tile_id).expect("Tile does not exist");
                game_actions.write(GameAction(wars::game::Action::Build(
                    tile.position(),
                    unit_type,
                )));
                *visible_build_menu = VisibleBuildMenu(None);
            }
            InteractionEvent::CancelSelectUnitToBuild => {
                *visible_build_menu = VisibleBuildMenu(None);
            }
            InteractionEvent::CancelSelectAction => {
                for (_, mut in_attack_range) in tile_in_attack_ranges.iter_mut() {
                    *in_attack_range = InAttackRange(false);
                }
                *visible_action_menu = VisibleActionMenu(None);
            }
            InteractionEvent::CancelSelectAttackTarget => {
                for (_, mut highlight) in unit_highlights.iter_mut() {
                    *highlight = UnitHighlight::Normal;
                }
                for (_, mut damage_indicator) in damage_indicators.iter_mut() {
                    *damage_indicator = DamageIndicator::Hidden;
                }
                for (_, mut in_attack_range) in tile_in_attack_ranges.iter_mut() {
                    *in_attack_range = InAttackRange(false);
                }
            }
            InteractionEvent::CancelSelectUnloadUnit => {
                *visible_unload_menu = VisibleUnloadMenu(None);
            }
            InteractionEvent::CancelSelectUnloadDestination => {
                for (_, mut highlight) in tile_highlights.iter_mut() {
                    *highlight = TileHighlight::Normal;
                }
            }
            InteractionEvent::EndTurn => {
                game_actions.write(GameAction(wars::game::Action::EndTurn));
            }
        }
    };

    for event in events.read() {
        info!("Input event: {event:?}");

        if game.in_turn_number().and_then(|n| players.get(&n)) == Some(&Player::Bot) {
            info!("Bot in turn");
            continue;
        }
        interaction_state
            .handle(*event, game, &mut interaction_event_handler)
            .expect("Interaction error");
    }
}

fn action_dispatch_system(
    mut next_state: ResMut<NextState<AppState>>,
    mut connection: Single<&mut Connection>,
    mut game: ResMut<Game>,
    mut game_actions: EventReader<GameAction>,
    mut game_events: EventWriter<GameEvent>,
) {
    let Game::InGame(game, _players, game_id) = game.as_mut() else {
        error!("Not in game!");
        next_state.set(AppState::MainMenu);
        return;
    };

    for GameAction(action) in game_actions.read() {
        if let Some(game_id) = game_id {
            // Remote game
            connection.send(wars::protocol::ActionMessage::GameAction(
                *game_id,
                action.clone(),
            ));
        } else {
            // Local game
            wars::game::action::perform(game, action.clone(), &mut |event| {
                game_events.write(GameEvent(event));
            })
            .expect("Error performing action");
        }
    }
}
fn remote_game_system(
    mut next_state: ResMut<NextState<AppState>>,
    mut connection: Single<&mut Connection>,
    mut game: ResMut<Game>,
    mut game_events: EventWriter<GameEvent>,
) {
    let Game::InGame(game, _players, game_id) = game.as_mut() else {
        error!("Not in game!");
        next_state.set(AppState::MainMenu);
        return;
    };

    let Some(game_id) = game_id else {
        // Local game
        return;
    };
    for event in connection.recv_all() {
        match event {
            crate::connection::ConnectionEvent::GameEvent(event_game_id, game_event) => {
                info!("GameEvent: {event_game_id}: {game_event:?}");
                if *game_id != event_game_id {
                    continue;
                }
                wars::game::action::process(game, &game_event).expect("Error processing event");
                game_events.write(GameEvent(game_event));
            }
            crate::connection::ConnectionEvent::GameActionError(event_game_id, action_error) => {
                if *game_id != event_game_id {
                    continue;
                }
                error!("GameActionError: {action_error}");
            }
            crate::connection::ConnectionEvent::Disconnected => {
                next_state.set(AppState::MainMenu);
            }
            _ => (),
        }
    }
}
