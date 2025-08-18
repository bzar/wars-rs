use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use wars::{game::PlayerNumber, protocol::GameId};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Player {
    Human,
    Bot,
    Remote,
}
#[derive(Resource)]
pub enum Game {
    None,
    PreGame(
        wars::game::Map,
        HashMap<PlayerNumber, Player>,
        Option<GameId>,
    ),
    InGame(
        wars::game::Game,
        HashMap<PlayerNumber, Player>,
        Option<GameId>,
    ),
}

impl Game {
    pub fn in_turn(&self) -> Option<&Player> {
        let Game::InGame(state, players, _) = self else {
            return None;
        };
        state.in_turn_number().and_then(|n| players.get(&n))
    }
    pub fn is_remote(&self) -> bool {
        match self {
            Game::None => false,
            Game::PreGame(_, _, game_id) => game_id.is_some(),
            Game::InGame(_, _, game_id) => game_id.is_some(),
        }
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

#[derive(Resource)]
pub struct VisibleActionMenu(
    pub Option<(wars::game::Position, HashSet<crate::components::Action>)>,
);
#[derive(Resource)]
pub struct VisibleBuildMenu(
    pub  Option<(
        wars::game::Position,
        HashSet<wars::model::UnitClass>,
        Option<wars::game::PlayerNumber>,
        u32,
    )>,
);

#[derive(Resource)]
pub struct VisibleUnloadMenu(pub Option<(wars::game::Position, Vec<wars::game::UnitId>)>);

#[derive(Resource, Eq, PartialEq)]
pub enum InputLayer {
    UI,
    Game,
}

#[derive(Resource)]
pub struct InTurnPlayer(pub Option<wars::game::PlayerNumber>);
