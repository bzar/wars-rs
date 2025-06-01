use crate::{
    BuildMenu, CaptureBar, CaptureBarBit, CaptureState, DeployEmblem, Deployed, EventProcessor,
    Game, Health, MapAction, MapInteractionState, Moved, OnesDigit, Owner, Prop, SpriteSheet,
    TensDigit, Theme, Tile, TileHighlight, Unit, UnitHighlight, VisibleActionButtons,
};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                unit_deployed_emblem_system,
                unit_moved_system,
                unit_highlight_system,
                tile_owner_system,
                tile_highlight_system,
                capture_bar_bit_system,
                health_number_system,
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
            let (ox, oy) = theme.hex_sprite_center_offset();
            if theme_tile.prop_index.is_some() {
                commands.spawn((
                    prop_bundle(*tile_id, tile, &theme, &sprite_sheet),
                    ChildOf(tile_sprite),
                    Transform::from_xyz(ox as f32, oy as f32, 0.1),
                ));
            }
            if tile.is_capturable() {
                let capture_bar = commands
                    .spawn((
                        CaptureBar,
                        ChildOf(tile_sprite),
                        sprite_sheet.sprite(theme.capture_bar.bar_index),
                        Transform::from_xyz(ox as f32, oy as f32, 0.2),
                    ))
                    .id();

                for i in 0..theme.spec.capture_bar.total_bits {
                    let y = theme.spec.capture_bar.bit_height * i;
                    let capture_point_limit =
                        i * tile.max_capture_points() / theme.spec.capture_bar.total_bits + 1;
                    commands.spawn((
                        CaptureBarBit(capture_point_limit),
                        ChildOf(capture_bar),
                        sprite_sheet.sprite(theme.capture_bar.recovering_bit_index),
                        Transform::from_xyz(0.0, y as f32, 0.0),
                    ));
                }
            }
            if let Some(unit_id) = tile.unit {
                let (ox, oy) = theme.hex_sprite_center_offset();
                let unit = game.units.get_ref(&unit_id).unwrap();
                commands.spawn((
                    unit_bundle(unit_id, unit, &theme, &sprite_sheet),
                    Transform::from_xyz(pos.x + ox as f32, pos.y + oy as f32, tz as f32 + 1.5),
                ));
            }
        }
    }
}

fn unit_deployed_emblem_system(
    changed_deploys: Query<&Deployed, Changed<Deployed>>,
    mut emblems: Query<(&ChildOf, &mut Visibility), With<DeployEmblem>>,
) {
    for (ChildOf(unit), mut visibility) in emblems.iter_mut() {
        if let Ok(Deployed(deployed)) = changed_deploys.get(*unit) {
            *visibility = if *deployed {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn unit_moved_system(mut changed_moved: Query<(&Moved, &mut Sprite), Changed<Moved>>) {
    for (Moved(moved), mut sprite) in changed_moved.iter_mut() {
        sprite.color.set_alpha(if *moved { 0.8 } else { 1.0 });
    }
}
fn unit_highlight_system(
    mut changed_highlights: Query<(&UnitHighlight, &mut Sprite), Changed<UnitHighlight>>,
) {
    for (highlight, mut sprite) in changed_highlights.iter_mut() {
        match highlight {
            UnitHighlight::Normal => sprite.color = Color::WHITE.with_alpha(sprite.color.alpha()),
            UnitHighlight::Target => {
                sprite.color = Color::srgba(1.0, 0.1, 0.1, 1.0).with_alpha(sprite.color.alpha())
            }
        }
    }
}
fn tile_owner_system(
    theme: Res<Theme>,
    game: Res<Game>,
    changed_owners: Query<&Owner, (With<Tile>, Changed<Owner>)>,
    mut props: Query<(&Prop, &ChildOf, &mut Sprite)>,
) {
    for (Prop(tile_id), ChildOf(tile), mut sprite) in props.iter_mut() {
        if let Ok(Owner(_owner)) = changed_owners.get(*tile) {
            let tile = game.tiles.get(*tile_id).unwrap();
            let theme_tile = theme.tile(&tile).unwrap();
            if let Some(prop_index) = theme_tile.prop_index {
                sprite.texture_atlas.as_mut().map(|a| a.index = prop_index);
            }
        }
    }
}
fn tile_highlight_system(
    mut changed_highlights: Query<
        (&TileHighlight, &mut Sprite, Option<&Children>),
        (Changed<TileHighlight>, Without<Prop>),
    >,
    mut props: Query<&mut Sprite, With<Prop>>,
) {
    for (highlight, mut sprite, children) in changed_highlights.iter_mut() {
        match highlight {
            TileHighlight::Normal | TileHighlight::Movable => {
                sprite.color = Color::WHITE;
            }
            TileHighlight::Unmovable => {
                sprite.color = Color::srgba(0.5, 0.5, 0.5, 1.0);
            }
        }
        if let Some(children) = children {
            for child in children {
                if let Ok(mut prop_sprite) = props.get_mut(*child) {
                    prop_sprite.color = sprite.color;
                }
            }
        }
    }
}

fn tile_click_observer(
    trigger: Trigger<Pointer<Click>>,
    tile_query: Query<&Tile>,
    game: ResMut<Game>,
    mut visible_action_buttons: ResMut<VisibleActionButtons>,
    mut state: ResMut<MapInteractionState>,
    mut event_processor: ResMut<EventProcessor>,
    mut unit_highlights: Query<(&Unit, &mut UnitHighlight)>,
    mut tile_highlights: Query<(&Tile, &mut TileHighlight)>,
    mut build_menus: Query<(&mut BuildMenu, &mut Visibility)>,
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
                    if let Some(destinations) = game.unit_move_options(unit_id) {
                        for (Tile(tile_id), mut highlight) in tile_highlights.iter_mut() {
                            let tile = game.tiles.get(*tile_id).unwrap();
                            *highlight = if destinations.contains_key(&tile.position()) {
                                TileHighlight::Movable
                            } else {
                                TileHighlight::Unmovable
                            };
                        }
                        next_state = Some(MapInteractionState::SelectDestination(
                            unit_id,
                            destinations,
                        ))
                    }
                }
            } else if !tile.terrain_data().build_classes.is_empty()
                && tile.owner == game.in_turn_number()
            {
                if let Ok((mut build_menu, mut visibility)) = build_menus.single_mut() {
                    *visibility = Visibility::Inherited;
                    build_menu.price_limit = game.in_turn_player().unwrap().funds;
                }
                next_state = Some(MapInteractionState::SelectUnitToBuild(*tile_id));
            }
        }
        MapInteractionState::SelectDestination(unit_id, ref destinations) => {
            for (_, mut highlight) in tile_highlights.iter_mut() {
                *highlight = TileHighlight::Normal;
            }
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
                if !attack_options.is_empty() {
                    action_options.insert(MapAction::Attack);
                }
                if game.unit_can_capture_tile(unit_id, *tile_id).is_ok() {
                    action_options.insert(MapAction::Capture);
                }

                *visible_action_buttons = VisibleActionButtons(action_options.clone());
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
            _unit_id,
            ref _path,
            ref _action_options,
            ref _attack_options,
        ) => {}
        MapInteractionState::SelectAttackTarget(unit_id, ref path, ref attack_options) => {
            if let Ok((_, target_tile)) = game.tiles.get_at(&position) {
                if let Some(target_id) = target_tile.unit {
                    if attack_options.contains(&target_id) {
                        for (_, mut highlight) in unit_highlights.iter_mut() {
                            *highlight = UnitHighlight::Normal;
                        }
                        visible_action_buttons.clear();
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
        }
        MapInteractionState::SelectUnitToBuild(_tile_id) => {
            build_menus
                .iter_mut()
                .for_each(|(_, mut v)| *v = Visibility::Hidden);
            next_state = Some(MapInteractionState::Normal);
        }
    };
    if let Some(next_state) = next_state {
        *state = next_state;
    }
}

fn health_number_system(
    theme: Res<Theme>,
    changed_healths: Query<(&Health, &Children), Changed<Health>>,
    mut ones: Query<(&mut Visibility, &mut Sprite), (With<OnesDigit>, Without<TensDigit>)>,
    mut tens: Query<(&mut Visibility, &mut Sprite), (With<TensDigit>, Without<OnesDigit>)>,
) {
    for (health, children) in changed_healths.iter() {
        for number in children.iter() {
            if let Ok((mut visibility, mut sprite)) = ones.get_mut(number) {
                match health {
                    Health::Full => {
                        *visibility = Visibility::Hidden;
                    }
                    Health::Damaged(x) => {
                        let digit = x % 10;
                        sprite.texture_atlas.as_mut().map(|a| {
                            a.index = theme.health_number(digit as usize).unwrap().number_index
                        });
                        *visibility = Visibility::Visible;
                    }
                }
            } else if let Ok((mut visibility, mut sprite)) = tens.get_mut(number) {
                match health {
                    Health::Full => {
                        *visibility = Visibility::Hidden;
                    }
                    Health::Damaged(x) => {
                        let digit = x % 100 / 10;
                        sprite.texture_atlas.as_mut().map(|a| {
                            a.index = theme.health_number(digit as usize).unwrap().number_index
                        });
                        *visibility = Visibility::Visible;
                    }
                }
            }
        }
    }
}
fn capture_bar_bit_system(
    theme: Res<Theme>,
    changed_capture_states: Query<(&CaptureState, &Children), Changed<CaptureState>>,
    mut capture_bars: Query<
        (&mut Visibility, &Children),
        (With<CaptureBar>, Without<CaptureBarBit>),
    >,
    mut capture_bar_bits: Query<(&CaptureBarBit, &mut Sprite, &mut Visibility)>,
) {
    for (capture_state, entity_children) in changed_capture_states.iter() {
        // FIXME: Not the most performant option, but works
        for bar in entity_children.iter() {
            let Ok((mut bar_visibility, bar_children)) = capture_bars.get_mut(bar) else {
                continue;
            };
            if matches!(capture_state, CaptureState::Full) {
                *bar_visibility = Visibility::Hidden;
            } else {
                *bar_visibility = Visibility::Visible;
            }

            for bit in bar_children.iter() {
                let Ok((CaptureBarBit(threshold), mut sprite, mut visibility)) =
                    capture_bar_bits.get_mut(bit)
                else {
                    continue;
                };
                match capture_state {
                    CaptureState::Capturing(value) if value >= threshold => {
                        sprite
                            .texture_atlas
                            .as_mut()
                            .map(|a| a.index = theme.capture_bar.capturing_bit_index);
                        *visibility = Visibility::Visible;
                    }
                    CaptureState::Recovering(value) if value >= threshold => {
                        sprite
                            .texture_atlas
                            .as_mut()
                            .map(|a| a.index = theme.capture_bar.recovering_bit_index);
                        *visibility = Visibility::Visible;
                    }
                    _ => {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
        }
    }
}
pub fn unit_bundle(
    unit_id: wars::game::UnitId,
    unit: &wars::game::Unit,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let deploy_emblem_visibility = if unit.deployed {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let theme_unit = theme.unit(unit.unit_type, unit.owner).unwrap();
    let health = if unit.is_damaged() {
        Health::Damaged(unit.health)
    } else {
        Health::Full
    };
    (
        Unit(unit_id),
        health,
        Deployed(unit.deployed),
        UnitHighlight::Normal,
        Moved(unit.moved),
        sprite_sheet.sprite(theme_unit.unit_index),
        children![
            (
                DeployEmblem,
                sprite_sheet.sprite(theme.deploy_emblem.emblem_index),
                deploy_emblem_visibility
            ),
            (
                TensDigit,
                sprite_sheet.sprite(
                    theme
                        .health_number(unit.health as usize % 100 / 10)
                        .unwrap()
                        .number_index
                ),
                Transform::from_xyz(-10.0, 0.0, 0.0)
            ),
            (
                OnesDigit,
                sprite_sheet.sprite(
                    theme
                        .health_number(unit.health as usize % 10)
                        .unwrap()
                        .number_index
                )
            )
        ],
    )
}

fn tile_bundle(
    tile_id: wars::game::TileId,
    tile: &wars::game::Tile,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    let theme_tile = theme.tile(tile).unwrap();
    let capture_state = if tile.capture_points == tile.max_capture_points() {
        CaptureState::Full
    } else {
        CaptureState::Recovering(tile.capture_points)
    };
    (
        Tile(tile_id),
        Owner(tile.owner.unwrap_or(0)),
        TileHighlight::Normal,
        capture_state,
        sprite_sheet.sprite(theme_tile.tile_index),
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
        sprite_sheet.sprite(theme_tile.prop_index.unwrap()),
    )
}
