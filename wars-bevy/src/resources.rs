use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use wars::game::PlayerNumber;

#[derive(PartialEq)]
pub enum Player {
    Human,
    Bot,
}
#[derive(Resource)]
pub struct Game {
    pub state: wars::game::Game,
    pub players: HashMap<PlayerNumber, Player>,
}

impl Game {
    pub fn in_turn(&self) -> Option<&Player> {
        self.state
            .in_turn_number()
            .and_then(|n| self.players.get(&n))
    }
}

#[derive(Resource, Deref)]
pub struct Theme(pub crate::theme::Theme);

#[derive(Resource, Default)]
pub struct SpriteSheet {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

impl SpriteSheet {
    pub fn sprite(&self, index: usize) -> Sprite {
        Sprite::from_atlas_image(
            self.texture.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index,
            },
        )
    }
    pub fn image(&self, index: usize) -> ImageNode {
        ImageNode::from_atlas_image(
            self.texture.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index,
            },
        )
    }
}

pub enum EventProcess {
    NoOp(wars::game::Event),
    Animation(Entity),
}

#[derive(Resource, Default)]
pub struct Visualizer {
    pub state: Option<EventProcess>,
    pub queue: VecDeque<wars::game::Event>,
}
#[derive(Resource, Default, Deref, DerefMut)]
pub struct VisibleActionButtons(pub HashSet<crate::components::Action>);

#[derive(Resource, Eq, PartialEq)]
pub enum InputLayer {
    UI,
    Game,
}

#[derive(Resource)]
pub struct InTurnPlayer(pub Option<wars::game::PlayerNumber>);
