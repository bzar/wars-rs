use std::collections::VecDeque;

use bevy::color::ColorCurve;
use bevy::prelude::*;
use itertools::Itertools;

#[derive(Clone)]
pub enum SpriteAnimationState {
    Sequence(VecDeque<SpriteAnimation>),
    Parallel(Vec<SpriteAnimation>),
    Position(f32, EasingCurve<Vec3>),
    Color(f32, ColorCurve<Color>),
    Scale(f32, EasingCurve<Vec3>),
    Delay(f32),
    Despawn,
}

#[derive(Component, Clone)]
pub struct SpriteAnimation {
    state: SpriteAnimationState,
    time: f32,
}

pub struct SpriteAnimationPlugin;

impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sprite_animation_system);
    }
}
impl From<SpriteAnimationState> for SpriteAnimation {
    fn from(state: SpriteAnimationState) -> Self {
        SpriteAnimation { state, time: 0.0 }
    }
}

pub fn sequence(parts: impl IntoIterator<Item = SpriteAnimationState>) -> SpriteAnimationState {
    SpriteAnimationState::Sequence(parts.into_iter().map(From::from).collect())
}
pub fn parallel(parts: impl IntoIterator<Item = SpriteAnimationState>) -> SpriteAnimationState {
    SpriteAnimationState::Parallel(parts.into_iter().map(From::from).collect())
}
pub fn repeat(n: usize, state: SpriteAnimationState) -> SpriteAnimationState {
    sequence([state].into_iter().cycle().take(n))
}
pub fn translate(p0: Vec3, p1: Vec3, t: f32, easing: EaseFunction) -> SpriteAnimationState {
    SpriteAnimationState::Position(t, EasingCurve::new(p0, p1, easing)).into()
}
pub fn scale(s0: f32, s1: f32, t: f32, easing: EaseFunction) -> SpriteAnimationState {
    SpriteAnimationState::Scale(
        t,
        EasingCurve::new(Vec3::splat(s0), Vec3::splat(s1), easing),
    )
    .into()
}
pub fn fade(a0: f32, a1: f32, t: f32) -> SpriteAnimationState {
    SpriteAnimationState::Color(
        t,
        ColorCurve::new([Color::WHITE.with_alpha(a0), Color::WHITE.with_alpha(a1)]).unwrap(),
    )
}
impl SpriteAnimation {
    fn is_done(&self) -> bool {
        match self.state {
            SpriteAnimationState::Sequence(ref parts) => parts.is_empty(),
            SpriteAnimationState::Parallel(ref parts) => parts.is_empty(),
            SpriteAnimationState::Position(duration, _) => self.time > duration,
            SpriteAnimationState::Scale(duration, _) => self.time > duration,
            SpriteAnimationState::Color(duration, _) => self.time > duration,
            SpriteAnimationState::Delay(duration) => self.time > duration,
            SpriteAnimationState::Despawn => false,
        }
    }

    fn advance(
        &mut self,
        delta: f32,
        entity: &mut EntityCommands,
        sprite: &mut Sprite,
        transform: &mut Transform,
    ) {
        self.time += delta;
        match self.state {
            SpriteAnimationState::Sequence(ref mut parts) => {
                let done = if let Some(current) = parts.front_mut() {
                    current.advance(delta, entity, sprite, transform);
                    current.is_done()
                } else {
                    false
                };

                if done {
                    parts.pop_front();
                }
            }
            SpriteAnimationState::Parallel(ref mut parts) => {
                for part in parts.iter_mut() {
                    part.advance(delta, entity, sprite, transform);
                }
                parts.retain(|part| !part.is_done());
            }
            SpriteAnimationState::Position(duration, ref easing_curve) => {
                if let Some(position) = easing_curve.sample(self.time.min(duration) / duration) {
                    transform.translation = position;
                }
            }
            SpriteAnimationState::Scale(duration, ref easing_curve) => {
                if let Some(position) = easing_curve.sample(self.time.min(duration) / duration) {
                    transform.scale = position;
                }
            }
            SpriteAnimationState::Color(duration, ref color_curve) => {
                if let Some(color) = color_curve.sample(self.time.min(duration) / duration) {
                    sprite.color = color;
                }
            }
            SpriteAnimationState::Delay(_) => {}
            SpriteAnimationState::Despawn => entity.despawn(),
        }
    }
}

fn sprite_animation_system(
    mut commands: Commands,
    mut animated_sprites: Query<(Entity, &mut Sprite, &mut SpriteAnimation, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut sprite, mut animation, mut transform) in animated_sprites.iter_mut() {
        animation.advance(
            time.delta_secs(),
            &mut commands.entity(entity),
            &mut sprite,
            &mut transform,
        );

        if animation.is_done() {
            commands.entity(entity).remove::<SpriteAnimation>();
            continue;
        }
    }
}

pub fn animate_move_unit(
    commands: &mut Commands,
    entity: Entity,
    waypoints: impl IntoIterator<Item = Vec3>,
) {
    let animation = sequence(waypoints.into_iter().tuple_windows().map(|(p0, p1)| {
        SpriteAnimationState::Position(0.1, EasingCurve::new(p0, p1, EaseFunction::Linear))
    }));

    commands
        .entity(entity)
        .insert(SpriteAnimation::from(animation));
}
pub fn animate_capturing(commands: &mut Commands, entity: Entity, position: Vec3) {
    let p0 = position;
    let p1 = position + Vec3::Y * 16.0;
    let animation = sequence([
        translate(p0, p1, 0.1, EaseFunction::QuadraticOut),
        translate(p1, p0, 0.1, EaseFunction::QuadraticIn),
    ]);
    commands
        .entity(entity)
        .insert(SpriteAnimation::from(animation));
}
pub fn animate_captured(commands: &mut Commands, entity: Entity, position: Vec3) {
    let p0 = position;
    let p1 = position + Vec3::Y * 16.0;
    let animation = repeat(
        3,
        sequence([
            translate(p0, p1, 0.1, EaseFunction::QuadraticOut),
            translate(p1, p0, 0.1, EaseFunction::QuadraticIn),
        ]),
    );
    commands
        .entity(entity)
        .insert(SpriteAnimation::from(animation));
}
pub fn animate_attack(
    commands: &mut Commands,
    entity: Entity,
    attacker: Vec3,
    target: Vec3,
    hex_distance: f32,
) {
    let distance = hex_distance / 2.0;
    let p0 = attacker;
    let p1 = p0 + (target - p0).normalize() * distance;

    let animation = sequence([
        translate(p0, p1, 0.1, EaseFunction::Linear),
        translate(p1, p0, 0.2, EaseFunction::Linear),
    ]);
    commands
        .entity(entity)
        .insert(SpriteAnimation::from(animation));
}
pub fn animate_destroy(commands: &mut Commands, entity: Entity) {
    let animation = sequence([
        SpriteAnimationState::Color(
            0.3,
            ColorCurve::new([Color::WHITE, Color::WHITE.with_alpha(0.0)]).unwrap(),
        ),
        SpriteAnimationState::Despawn,
    ]);

    commands
        .entity(entity)
        .insert(SpriteAnimation::from(animation));
}
