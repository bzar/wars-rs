use std::{collections::HashSet, f32::consts::TAU};

use crate::{animation, components::*, resources::*, AppState};
use bevy::{asset::RenderAssetUsages, prelude::*};
use wars::game::PlayerNumber;

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    unit_deployed_emblem_system,
                    unit_moved_system,
                    unit_highlight_system,
                    tile_owner_system,
                    tile_highlight_system,
                    capture_bar_bit_system,
                    health_number_system,
                    damage_number_system,
                    carrier_slot_system,
                    cursor_system,
                    tile_attack_range_system,
                    unit_move_preview_added_system,
                    unit_move_preview_cleanup_system,
                    action_menu_system,
                    build_menu_system,
                    unload_menu_system,
                )
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(AppState::InGame), on_enter_game);
    }
}

fn on_enter_game(
    mut commands: Commands,
    game: Res<Game>,
    theme: Res<Theme>,
    sprite_sheet: Res<SpriteSheet>,
) {
    let Game::InGame(game, ..) = game.as_ref() else {
        panic!("Not in game");
    };
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
                .observe(tile_hover_observer)
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
fn setup() {}

#[derive(Component)]
struct HexCursor;

fn cursor_system(
    mut commands: Commands,
    game: Res<Game>,
    theme: Res<Theme>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cursors: Query<&mut Transform, With<HexCursor>>,
    mut events: EventReader<InputEvent>,
) {
    let Game::InGame(game, ..) = game.as_ref() else {
        panic!("Not in game");
    };

    if let Ok(mut cursor) = cursors.single_mut() {
        for event in events.read() {
            match event {
                InputEvent::MapHover(tile_id) => {
                    let Some(tile) = game.tiles.get(*tile_id) else {
                        return;
                    };
                    let (x, y, z) = theme.map_hex_center(tile.x, tile.y);
                    let Some(theme_tile) = theme.tile(&tile) else {
                        return;
                    };
                    cursor.translation = Vec3::new(
                        x as f32,
                        (y + (theme.spec.image.height as i32 - theme.spec.hex.height as i32) / 2)
                            as f32
                            - theme_tile.offset as f32,
                        z as f32 + 1.0,
                    );
                }
                _ => (),
            }
        }
    } else {
        let w = theme.spec.hex.width as f32 / 2.0 + 2.0;
        let h = theme.spec.hex.height as f32 / 2.0 + 2.0;
        let t = theme.spec.hex.tri_width as f32 + 2.0;
        commands.spawn((
            HexCursor,
            Mesh2d(
                meshes.add(
                    Mesh::new(
                        bevy::render::mesh::PrimitiveTopology::TriangleStrip,
                        RenderAssetUsages::default(),
                    )
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        [
                            (w, 0.0),
                            (w - t, h),
                            (w - t, -h),
                            (t - w, h),
                            (t - w, -h),
                            (-w, 0.0),
                        ]
                        .into_iter()
                        .map(|(x, y)| Vec3::new(x, y, 0.0))
                        .collect::<Vec<_>>(),
                    ),
                ),
            ),
            MeshMaterial2d(materials.add(Color::from(
                bevy::color::palettes::basic::WHITE.with_alpha(0.2),
            ))),
        ));
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

fn tile_attack_range_system(
    changed: Query<&InAttackRange, Changed<InAttackRange>>,
    mut indicators: Query<(&ChildOf, &mut Visibility), With<AttackRangeIndicator>>,
) {
    for (ChildOf(tile), mut visibility) in indicators.iter_mut() {
        if let Ok(InAttackRange(in_range)) = changed.get(*tile) {
            *visibility = if *in_range {
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
fn unit_move_preview_added_system(
    mut commands: Commands,
    game: Res<Game>,
    theme: Res<Theme>,
    added_move_previews: Query<(Entity, &UnitMovePreview), Added<UnitMovePreview>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Game::InGame(game, ..) = game.as_ref() else {
        panic!("Not in game");
    };

    let Some(player_color): Option<Color> = theme
        .spec
        .player_colors
        .get(game.in_turn_number().unwrap_or(0) as usize)
        .map(Into::into)
    else {
        return;
    };
    for (entity, UnitMovePreview(path)) in added_move_previews.iter() {
        if path.len() > 1 {
            let waypoints = path
                .into_iter()
                .map(|pos| game.tiles.get_at(&pos).expect("No such tile"))
                .map(|(_tile_id, tile)| theme.unit_position(&tile));

            for translation in waypoints {
                commands.spawn((
                    UnitMovePreviewProp(entity),
                    Mesh2d(meshes.add(Circle::new(8.0))),
                    MeshMaterial2d(materials.add(player_color)),
                    Transform::from_translation(translation),
                ));
            }
        }
    }
}
fn unit_move_preview_cleanup_system(
    mut commands: Commands,
    move_preview_props: Query<(Entity, &UnitMovePreviewProp)>,
    move_previews: Query<Entity, With<UnitMovePreview>>,
) {
    for (entity, UnitMovePreviewProp(parent)) in move_preview_props.iter() {
        if move_previews.get(*parent).is_err() {
            commands.entity(entity).despawn();
        }
    }
}
fn tile_owner_system(
    theme: Res<Theme>,
    game: Res<Game>,
    changed_owners: Query<&Owner, (With<Tile>, Changed<Owner>)>,
    mut props: Query<(&Prop, &ChildOf, &mut Sprite)>,
) {
    let Game::InGame(game, ..) = game.as_ref() else {
        panic!("Not in game");
    };

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
    input_layer: Res<InputLayer>,
    mut events: EventWriter<InputEvent>,
) {
    if *input_layer == InputLayer::UI {
        return;
    }
    let Ok(Tile(tile_id)) = tile_query.get(trigger.target()) else {
        return;
    };

    events.write(InputEvent::MapSelect(*tile_id));
}
fn tile_hover_observer(
    trigger: Trigger<Pointer<Over>>,
    tile_query: Query<&Tile>,
    input_layer: Res<InputLayer>,
    mut events: EventWriter<InputEvent>,
) {
    if *input_layer == InputLayer::UI {
        return;
    }
    let Ok(Tile(tile_id)) = tile_query.get(trigger.target()) else {
        return;
    };

    events.write(InputEvent::MapHover(*tile_id));
}

fn health_number_system(
    theme: Res<Theme>,
    changed_healths: Query<(&Health, &Children), Changed<Health>>,
    mut ones: Query<
        (&mut Visibility, &mut Sprite),
        (With<HealthOnesDigit>, Without<HealthTensDigit>),
    >,
    mut tens: Query<
        (&mut Visibility, &mut Sprite),
        (With<HealthTensDigit>, Without<HealthOnesDigit>),
    >,
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
                    Health::Damaged(x) if *x < 10 => {
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
fn damage_number_system(
    theme: Res<Theme>,
    changed_damages: Query<(&DamageIndicator, &Children), Changed<DamageIndicator>>,
    mut ones: Query<
        (&mut Visibility, &mut Sprite),
        (
            With<DamageOnesDigit>,
            (Without<DamageTensDigit>, Without<DamageHundredsDigit>),
        ),
    >,
    mut tens: Query<
        (&mut Visibility, &mut Sprite),
        (
            With<DamageTensDigit>,
            (Without<DamageOnesDigit>, Without<DamageHundredsDigit>),
        ),
    >,
    mut hundreds: Query<
        (&mut Visibility, &mut Sprite),
        (
            With<DamageHundredsDigit>,
            (Without<DamageOnesDigit>, Without<DamageTensDigit>),
        ),
    >,
) {
    for (damage, children) in changed_damages.iter() {
        for number in children.iter() {
            if let Ok((mut visibility, mut sprite)) = ones.get_mut(number) {
                match damage {
                    DamageIndicator::Hidden => {
                        *visibility = Visibility::Hidden;
                    }
                    DamageIndicator::Visible(x) => {
                        let digit = x % 10;
                        sprite.texture_atlas.as_mut().map(|a| {
                            a.index = theme.damage_number(digit as usize).unwrap().number_index
                        });
                        *visibility = Visibility::Visible;
                    }
                }
            } else if let Ok((mut visibility, mut sprite)) = tens.get_mut(number) {
                match damage {
                    DamageIndicator::Hidden => {
                        *visibility = Visibility::Hidden;
                    }
                    DamageIndicator::Visible(x) if *x < 10 => {
                        *visibility = Visibility::Hidden;
                    }
                    DamageIndicator::Visible(x) => {
                        let digit = x % 100 / 10;
                        sprite.texture_atlas.as_mut().map(|a| {
                            a.index = theme.damage_number(digit as usize).unwrap().number_index
                        });
                        *visibility = Visibility::Visible;
                    }
                }
            } else if let Ok((mut visibility, mut sprite)) = hundreds.get_mut(number) {
                match damage {
                    DamageIndicator::Hidden => {
                        *visibility = Visibility::Hidden;
                    }
                    DamageIndicator::Visible(x) if *x < 100 => {
                        *visibility = Visibility::Hidden;
                    }
                    DamageIndicator::Visible(x) => {
                        let digit = x % 1000 / 100;
                        sprite.texture_atlas.as_mut().map(|a| {
                            a.index = theme.damage_number(digit as usize).unwrap().number_index
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
fn carrier_slot_system(
    theme: Res<Theme>,
    changed_carrier_states: Query<(&Carrier, &Children), Changed<Carrier>>,
    mut carrier_slots: Query<(&CarrierSlot, &mut Visibility, &mut Sprite)>,
) {
    for (carrier, children) in changed_carrier_states.iter() {
        for &child in children {
            if let Ok((&CarrierSlot(index), mut visibility, mut sprite)) =
                carrier_slots.get_mut(child)
            {
                *visibility = if carrier.capacity > index {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                sprite.texture_atlas.as_mut().map(|a| {
                    a.index = if carrier.load > index {
                        theme.carrier_slot.full_index
                    } else {
                        theme.carrier_slot.empty_index
                    }
                });
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
    let theme_unit = theme.unit(unit.unit_type, unit.owner).unwrap();
    let health = if unit.is_damaged() {
        Health::Damaged(unit.health)
    } else {
        Health::Full
    };
    (
        Unit(unit_id),
        health,
        DamageIndicator::Hidden,
        Deployed(unit.deployed),
        UnitHighlight::Normal,
        Moved(unit.moved),
        Carrier {
            load: unit.carried.len() as u32,
            capacity: unit.unit_type_data().carry_num,
        },
        sprite_sheet.sprite(theme_unit.unit_index),
        children![
            (
                DeployEmblem,
                sprite_sheet.sprite(theme.deploy_emblem.emblem_index),
                Visibility::Hidden,
            ),
            (
                HealthTensDigit,
                sprite_sheet.sprite(
                    theme
                        .health_number(unit.health as usize % 100 / 10)
                        .unwrap()
                        .number_index
                ),
                Transform::from_xyz(-(theme.spec.number.width as f32), 0.0, 1.0),
                Visibility::Hidden,
            ),
            (
                HealthOnesDigit,
                sprite_sheet.sprite(
                    theme
                        .health_number(unit.health as usize % 10)
                        .unwrap()
                        .number_index
                ),
                Transform::from_xyz(0.0, 0.0, 1.0),
                Visibility::Hidden,
            ),
            (
                DamageHundredsDigit,
                sprite_sheet.sprite(theme.damage_number(0).unwrap().number_index),
                Transform::from_xyz(-2.0 * (theme.spec.number.width as f32), 0.0, 1.0),
                Visibility::Hidden,
            ),
            (
                DamageTensDigit,
                sprite_sheet.sprite(theme.damage_number(0).unwrap().number_index),
                Transform::from_xyz(-(theme.spec.number.width as f32), 0.0, 1.0),
                Visibility::Hidden,
            ),
            (
                DamageOnesDigit,
                sprite_sheet.sprite(theme.damage_number(0).unwrap().number_index),
                Transform::from_xyz(0.0, 0.0, 1.0),
                Visibility::Hidden,
            ),
            // TODO: Support more than two slots
            (
                CarrierSlot(0),
                sprite_sheet.sprite(theme.carrier_slot.empty_index),
                Transform::from_xyz(0.0, 0.0, 1.0),
                Visibility::Hidden,
            ),
            (
                CarrierSlot(1),
                sprite_sheet.sprite(theme.carrier_slot.empty_index),
                Transform::from_xyz(0.0, theme.carrier_slot.height as f32, 1.0),
                Visibility::Hidden,
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
        InAttackRange(false),
        TileHighlight::Normal,
        capture_state,
        sprite_sheet.sprite(theme_tile.tile_index),
        children![(
            AttackRangeIndicator,
            sprite_sheet.sprite(theme.masks.attack_hex_mask_index),
            Transform::from_xyz(0.0, 0.0, 0.15),
            Visibility::Hidden,
        )],
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

fn action_menu_system(
    mut commands: Commands,
    visible_action_menu: Res<VisibleActionMenu>,
    action_menus: Query<Entity, With<ActionMenu>>,
    theme: Res<Theme>,
    asset_server: Res<AssetServer>,
) {
    if let VisibleActionMenu(Some((position, ref actions))) = *visible_action_menu {
        if action_menus.is_empty() {
            let wars::game::Position(x, y) = position;
            let (x, y, z) = theme.hex_sprite_center(x, y);
            spawn_action_menu(
                commands,
                &actions,
                Vec3::new(x as f32, y as f32, z as f32 + 200.0),
                asset_server,
            );
        }
    } else if !action_menus.is_empty() {
        for menu in action_menus.iter() {
            commands.entity(menu).despawn();
        }
    }
}
pub fn spawn_action_menu(
    mut commands: Commands,
    actions: &HashSet<Action>,
    position: Vec3,
    asset_server: Res<AssetServer>,
) {
    let menu = commands
        .spawn((
            ActionMenu,
            Sprite::from_color(Color::WHITE.with_alpha(0.0), Vec2::ZERO),
            Transform::from_translation(position),
        ))
        .id();
    let mut i = 0;
    for action in enum_iterator::all::<Action>() {
        if actions.contains(&action) {
            let angle = TAU * i as f32 / actions.len() as f32;
            let position = Transform::from_rotation(Quat::from_rotation_z(angle))
                .transform_point(Vec3::Y * 64.0);
            let action = action.clone();
            commands
                .spawn((
                    action_menu_button_bundle(&action, position, menu, &asset_server),
                    Pickable::default(),
                ))
                .observe(
                    move |_trigger: Trigger<Pointer<Click>>,
                          mut events: EventWriter<InputEvent>| {
                        info!("triggered!");
                        events.write(InputEvent::Action(action));
                    },
                );

            i += 1;
        }
    }
}

fn action_menu_button_bundle(
    action: &Action,
    position: Vec3,
    menu: Entity,
    asset_server: &AssetServer,
) -> impl Bundle {
    let icon = match action {
        Action::Wait => "gui/action-wait.png",
        Action::Attack => "gui/action-attack.png",
        Action::Capture => "gui/action-capture.png",
        Action::Deploy => "gui/action-deploy.png",
        Action::Undeploy => "gui/action-undeploy.png",
        Action::Load => "gui/action-load.png",
        Action::Unload => "gui/action-unload.png",
        Action::Cancel => "gui/action-cancel.png",
    };

    (
        Sprite {
            image: asset_server.load(icon),
            color: Color::WHITE.with_alpha(0.0),
            ..Default::default()
        },
        Transform::from_translation(position),
        ChildOf(menu),
        animation::SpriteAnimation::from(animation::parallel([
            animation::translate(
                Vec3::new(0.0, 0.0, position.z),
                position,
                0.2,
                EaseFunction::QuadraticOut,
            ),
            animation::scale(0.0, 1.0, 0.2, EaseFunction::QuadraticOut),
            animation::fade(0.0, 0.9, 0.2),
        ])),
    )
}

fn build_menu_system(
    mut commands: Commands,
    visible_build_menu: Res<VisibleBuildMenu>,
    build_menus: Query<Entity, With<BuildMenu>>,
    theme: Res<Theme>,
    sprite_sheet: Res<SpriteSheet>,
) {
    if let VisibleBuildMenu(Some((position, ref unit_classes, player_number, price_limit))) =
        *visible_build_menu
    {
        if build_menus.is_empty() {
            let wars::game::Position(x, y) = position;
            let (x, y, z) = theme.hex_sprite_center(x, y);
            spawn_build_menu(
                commands,
                &unit_classes,
                player_number,
                price_limit,
                Vec3::new(x as f32, y as f32, z as f32 + 200.0),
                &theme,
                &sprite_sheet,
            );
        }
    } else if !build_menus.is_empty() {
        for menu in build_menus.iter() {
            commands.entity(menu).despawn();
        }
    }
}

pub fn spawn_build_menu(
    mut commands: Commands,
    unit_classes: &HashSet<wars::model::UnitClass>,
    player_number: Option<wars::game::PlayerNumber>,
    price_limit: u32,
    position: Vec3,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) {
    let menu = commands
        .spawn((
            BuildMenu,
            Sprite::from_color(Color::WHITE.with_alpha(0.0), Vec2::ZERO),
            Transform::from_translation(position),
        ))
        .id();

    let mut unit_types = enum_iterator::all::<wars::model::UnitType>()
        .filter(|ut| {
            let unit_type = wars::model::unit_type(*ut);
            unit_classes.contains(&unit_type.unit_class)
        })
        .collect::<Vec<_>>();
    unit_types.sort_by_key(|t| wars::model::unit_type(*t).price);

    for (i, unit_type) in unit_types.iter().enumerate() {
        let angle = TAU * i as f32 / unit_types.len() as f32;
        let position = Transform::from_rotation(Quat::from_rotation_z(angle))
            .transform_point(Vec3::Y * (32.0 + 12.0 * unit_types.len() as f32));
        let unit_type = unit_type.clone();
        let price = wars::model::unit_type(unit_type).price;
        let enabled = price <= price_limit;
        let mut item = commands.spawn((
            build_menu_button_bundle(
                &unit_type,
                player_number,
                position,
                menu,
                theme,
                sprite_sheet,
                price,
                enabled,
            ),
            Pickable::default(),
        ));

        if enabled {
            item.observe(
                move |_trigger: Trigger<Pointer<Click>>, mut events: EventWriter<InputEvent>| {
                    info!("triggered!");
                    events.write(InputEvent::BuildUnit(unit_type));
                },
            );
        }
    }
}

fn build_menu_button_bundle(
    unit_type: &wars::model::UnitType,
    player_number: Option<PlayerNumber>,
    position: Vec3,
    menu: Entity,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
    price: u32,
    enabled: bool,
) -> impl Bundle {
    let color = if enabled {
        Color::WHITE
    } else {
        Color::srgb(0.5, 0.5, 0.5)
    };
    let text_color = if enabled {
        Color::WHITE
    } else {
        Color::srgb(0.8, 0.8, 0.8)
    };
    (
        Sprite {
            color: color.with_alpha(0.0),
            ..sprite_sheet.sprite(theme.build_item.background_index)
        },
        Transform::from_translation(position).with_scale(Vec3::splat(0.0)),
        ChildOf(menu),
        animation::SpriteAnimation::from(animation::parallel([
            animation::translate(
                Vec3::new(0.0, 0.0, position.z),
                position,
                0.2,
                EaseFunction::QuadraticOut,
            ),
            animation::scale(0.0, 1.0, 0.2, EaseFunction::QuadraticOut),
            animation::fade(0.0, 0.9, 0.2),
        ])),
        children![
            (
                Sprite {
                    color,
                    ..sprite_sheet.sprite(theme.unit(*unit_type, player_number).unwrap().unit_index)
                },
                Transform::from_xyz(0.0, 0.0, 1.0)
            ),
            (
                Text2d::new(format!("{price} cr")),
                TextFont {
                    font_size: 13.0,
                    ..Default::default()
                },
                Transform::from_xyz(0.0, -20.0, 2.0),
                TextColor(text_color)
            ),
            (
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.8), Vec2::new(60.0, 14.0)),
                Transform::from_xyz(0.0, -20.0, 1.5),
            )
        ],
    )
}
fn unload_menu_system(
    mut commands: Commands,
    visible_unload_menu: Res<VisibleUnloadMenu>,
    unload_menus: Query<Entity, With<UnloadMenu>>,
    game: Res<Game>,
    theme: Res<Theme>,
    sprite_sheet: Res<SpriteSheet>,
) {
    if let VisibleUnloadMenu(Some((position, ref unit_ids))) = *visible_unload_menu {
        if unload_menus.is_empty() {
            let wars::game::Position(x, y) = position;
            let (x, y, z) = theme.hex_sprite_center(x, y);
            info!("Spawing unload menu at position {position:?} -> ({x}, {y}, {z})");
            spawn_unload_menu(
                commands,
                &unit_ids,
                Vec3::new(x as f32, y as f32, z as f32 + 200.0),
                &game,
                &theme,
                &sprite_sheet,
            );
        }
    } else if !unload_menus.is_empty() {
        for menu in unload_menus.iter() {
            commands.entity(menu).despawn();
        }
    }
}

pub fn spawn_unload_menu(
    mut commands: Commands,
    unit_ids: &Vec<wars::game::UnitId>,
    position: Vec3,
    game: &Game,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) {
    let Game::InGame(game, _) = game else {
        panic!("Not in game");
    };
    let menu = commands
        .spawn((
            UnloadMenu,
            Sprite::from_color(Color::WHITE.with_alpha(0.0), Vec2::ZERO),
            Transform::from_translation(position),
        ))
        .id();

    for (i, &unit_id) in unit_ids.iter().enumerate() {
        let angle = TAU * i as f32 / unit_ids.len() as f32;
        let position = Transform::from_rotation(Quat::from_rotation_z(angle))
            .transform_point(Vec3::Y * (32.0 + 12.0 * unit_ids.len() as f32));
        let Some(unit) = game.units.get(unit_id) else {
            panic!("Unknown unit in unload menu!");
        };
        let mut item = commands.spawn((
            unload_menu_button_bundle(
                &unit.unit_type,
                unit.owner,
                position,
                menu,
                theme,
                sprite_sheet,
            ),
            Pickable::default(),
        ));

        item.observe(
            move |_trigger: Trigger<Pointer<Click>>, mut events: EventWriter<InputEvent>| {
                info!("triggered!");
                events.write(InputEvent::UnloadUnit(unit_id));
            },
        );
    }
}

fn unload_menu_button_bundle(
    unit_type: &wars::model::UnitType,
    player_number: Option<PlayerNumber>,
    position: Vec3,
    menu: Entity,
    theme: &Theme,
    sprite_sheet: &SpriteSheet,
) -> impl Bundle {
    (
        Sprite {
            color: Color::WHITE.with_alpha(0.0),
            ..sprite_sheet.sprite(theme.build_item.background_index)
        },
        Transform::from_translation(position).with_scale(Vec3::splat(0.0)),
        ChildOf(menu),
        animation::SpriteAnimation::from(animation::parallel([
            animation::translate(
                Vec3::new(0.0, 0.0, position.z),
                position,
                0.2,
                EaseFunction::QuadraticOut,
            ),
            animation::scale(0.0, 1.0, 0.2, EaseFunction::QuadraticOut),
            animation::fade(0.0, 0.9, 0.2),
        ])),
        children![(
            sprite_sheet.sprite(theme.unit(*unit_type, player_number).unwrap().unit_index),
            Transform::from_xyz(0.0, 0.0, 1.0)
        ),],
    )
}
