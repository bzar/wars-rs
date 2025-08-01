use bevy::prelude::*;
use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub filename: String,
    pub high_dpi_filename: String,
    pub cols: usize,
    pub rows: usize,
}
#[derive(Deserialize)]
pub struct WidthHeight {
    pub width: u32,
    pub height: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hex {
    pub width: u32,
    pub height: u32,
    pub thickness: u32,
    pub tri_width: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Masks {
    pub attack_hex: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureBar {
    pub bit_height: u32,
    pub total_bits: u32,
    pub bar_name: String,
    pub capturing_name: String,
    pub recovering_name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarrierSlot {
    pub slot_height: u32,
    pub free_slot_name: String,
    pub occupied_slot_name: String,
}
#[derive(Deserialize)]
pub struct Tile {
    pub hex: String,
    pub prop: Option<String>,
    pub offset: i32,
}
#[derive(Deserialize)]
pub struct Numbers {
    pub health: Vec<String>,
    pub damage: Vec<String>,
}
#[derive(Deserialize)]
pub struct Emblems {
    pub deploy: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ui {
    pub build_item_background: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSpec {
    pub player_colors: Vec<Color>,
    pub sheet: Sheet,
    pub image: WidthHeight,
    pub hex: Hex,
    pub number: WidthHeight,
    pub capture_bar: CaptureBar,
    pub carrier_slot: CarrierSlot,
    pub sheet_layout: Vec<Option<String>>,
    pub tiles: Vec<Vec<Vec<Tile>>>,
    pub units: Vec<Vec<String>>,
    pub numbers: Numbers,
    pub emblems: Emblems,
    pub masks: Masks,
    pub ui: Ui,
}

pub type Index = usize;

#[derive(Clone)]
pub struct ThemeTile {
    pub tile_index: Index,
    pub prop_index: Option<Index>,
    pub offset: i32,
}

#[derive(Clone)]
pub struct ThemeUnit {
    pub unit_index: Index,
}
pub struct ThemeNumber {
    pub number_index: Index,
}
pub struct ThemeEmblem {
    pub emblem_index: Index,
}
pub struct ThemeMasks {
    pub attack_hex_mask_index: Index,
}
pub struct ThemeCaptureBar {
    pub bar_index: Index,
    pub capturing_bit_index: Index,
    pub recovering_bit_index: Index,
}
pub struct ThemeCarrierSlot {
    pub empty_index: Index,
    pub full_index: Index,
    pub height: u32,
}
pub struct ThemeBuildItem {
    pub background_index: Index,
}
pub struct Theme {
    pub spec: ThemeSpec,
    tiles: HashMap<(usize, usize, usize), ThemeTile>,
    units: HashMap<(usize, usize), ThemeUnit>,
    health_numbers: Vec<ThemeNumber>,
    damage_numbers: Vec<ThemeNumber>,
    pub deploy_emblem: ThemeEmblem,
    pub capture_bar: ThemeCaptureBar,
    pub carrier_slot: ThemeCarrierSlot,
    pub masks: ThemeMasks,
    pub build_item: ThemeBuildItem,
}

impl From<ThemeSpec> for Theme {
    fn from(spec: ThemeSpec) -> Self {
        let label_indices: HashMap<&String, usize> = spec
            .sheet_layout
            .iter()
            .enumerate()
            .filter_map(|(i, x)| x.as_ref().map(move |x| (x, i)))
            .collect();
        let tiles = spec
            .tiles
            .iter()
            .enumerate()
            .flat_map(|(terrain_index, subtypes)| {
                subtypes
                    .iter()
                    .enumerate()
                    .flat_map(move |(terrain_subtype_index, owners)| {
                        owners.iter().enumerate().map(move |(owner, tile)| {
                            (terrain_index, terrain_subtype_index, owner, tile)
                        })
                    })
            })
            .filter_map(|(terrain_index, terrain_subtype_index, owner, tile)| {
                let tile_index = label_indices.get(&tile.hex).copied()?;
                let prop_index = tile
                    .prop
                    .as_ref()
                    .and_then(|label| label_indices.get(&label).copied());
                let offset = tile.offset;
                let theme_tile = ThemeTile {
                    tile_index,
                    prop_index,
                    offset,
                };
                Some(((terrain_index, terrain_subtype_index, owner), theme_tile))
            })
            .collect();
        let units = spec
            .units
            .iter()
            .enumerate()
            .flat_map(|(unit_index, owners)| {
                owners
                    .iter()
                    .enumerate()
                    .map(move |(owner, label)| (unit_index, owner, label))
            })
            .filter_map(|(unit_type, owner, label)| {
                let unit_index = label_indices.get(&label).copied()?;
                let theme_unit = ThemeUnit { unit_index };
                Some(((unit_type, owner), theme_unit))
            })
            .collect();

        let health_numbers = spec
            .numbers
            .health
            .iter()
            .filter_map(|label| label_indices.get(label))
            .map(|&number_index| ThemeNumber { number_index })
            .collect();
        let damage_numbers = spec
            .numbers
            .damage
            .iter()
            .filter_map(|label| label_indices.get(label))
            .map(|&number_index| ThemeNumber { number_index })
            .collect();
        let deploy_emblem = ThemeEmblem {
            emblem_index: label_indices.get(&spec.emblems.deploy).copied().unwrap(),
        };
        let capture_bar = ThemeCaptureBar {
            bar_index: *label_indices.get(&spec.capture_bar.bar_name).unwrap(),
            capturing_bit_index: *label_indices.get(&spec.capture_bar.capturing_name).unwrap(),
            recovering_bit_index: *label_indices
                .get(&spec.capture_bar.recovering_name)
                .unwrap(),
        };
        let carrier_slot = ThemeCarrierSlot {
            empty_index: *label_indices
                .get(&spec.carrier_slot.free_slot_name)
                .unwrap(),
            full_index: *label_indices
                .get(&spec.carrier_slot.occupied_slot_name)
                .unwrap(),
            height: spec.carrier_slot.slot_height,
        };
        let masks = ThemeMasks {
            attack_hex_mask_index: label_indices.get(&spec.masks.attack_hex).copied().unwrap(),
        };
        let build_item = ThemeBuildItem {
            background_index: label_indices
                .get(&spec.ui.build_item_background)
                .copied()
                .unwrap(),
        };
        Self {
            spec,
            tiles,
            units,
            health_numbers,
            damage_numbers,
            deploy_emblem,
            capture_bar,
            carrier_slot,
            masks,
            build_item,
        }
    }
}
impl Theme {
    pub fn from_json(data: &str) -> Result<Self, serde_json::Error> {
        let spec = serde_json::from_str::<ThemeSpec>(data)?;
        Ok(Theme::from(spec))
    }

    pub fn tile(&self, tile: &wars::game::Tile) -> Option<&ThemeTile> {
        self.tiles.get(&(
            tile.terrain as usize,
            tile.terrain_subtype_id as usize,
            tile.owner.unwrap_or(0) as usize,
        ))
    }
    pub fn unit(
        &self,
        unit_type: wars::model::UnitType,
        unit_owner: Option<wars::game::PlayerNumber>,
    ) -> Option<&ThemeUnit> {
        self.units
            .get(&(unit_type as usize, unit_owner.unwrap_or(0) as usize))
    }
    pub fn health_number(&self, number: usize) -> Option<&ThemeNumber> {
        self.health_numbers.get(number)
    }
    pub fn damage_number(&self, number: usize) -> Option<&ThemeNumber> {
        self.damage_numbers.get(number)
    }

    pub fn map_hex_center(&self, x: i32, y: i32) -> (i32, i32, i32) {
        let w = self.spec.hex.width as i32;
        let h = self.spec.hex.height as i32;
        let tw = self.spec.hex.tri_width as i32;
        (w / 2 + (w - tw) * x, -(x + 1) * h / 2 + h * -y, 2 * y + x)
    }

    pub fn hex_sprite_center_offset(&self) -> (i32, i32) {
        (0, (self.spec.image.height - self.spec.hex.height) as i32)
    }

    pub fn hex_sprite_center(&self, x: i32, y: i32) -> (i32, i32, i32) {
        let (ox, oy) = self.hex_sprite_center_offset();
        let (x, y, z) = self.map_hex_center(x, y);
        (x + ox, y + oy / 2, z)
    }
    pub fn unit_position(&self, tile: &wars::game::Tile) -> Vec3 {
        let (ox, oy) = self.hex_sprite_center_offset();
        let (x, y, z) = self.map_hex_center(tile.x, tile.y);
        let offset = self.tile(tile).map(|tt| tt.offset).unwrap_or(0);
        Vec3::new((x + ox) as f32, (y + oy - offset) as f32, z as f32 + 1.5)
    }

    pub fn recolor_unit(
        &self,
        index: Index,
        player_number: Option<wars::game::PlayerNumber>,
    ) -> Option<ThemeUnit> {
        if let Some(unit_type) = self
            .units
            .iter()
            .find_map(|((unit_type, _), tu)| (index == tu.unit_index).then_some(unit_type))
        {
            return self
                .units
                .get(&(*unit_type, player_number.unwrap_or(0) as usize))
                .cloned();
        }
        None
    }

    pub fn recolor_tile(
        &self,
        index: Index,
        player_number: Option<wars::game::PlayerNumber>,
    ) -> Option<ThemeTile> {
        if let Some((terrain, subtype)) =
            self.tiles.iter().find_map(|((terrain, subtype, _), tt)| {
                (index == tt.tile_index).then_some((terrain, subtype))
            })
        {
            return self
                .tiles
                .get(&(*terrain, *subtype, player_number.unwrap_or(0) as usize))
                .cloned();
        }
        None
    }

    pub fn recolor(
        &self,
        index: Index,
        player_number: Option<wars::game::PlayerNumber>,
    ) -> Option<Index> {
        if let Some(theme_unit) = self.recolor_unit(index, player_number) {
            return Some(theme_unit.unit_index);
        }

        if let Some(theme_tile) = self.recolor_tile(index, player_number) {
            return Some(theme_tile.tile_index);
        }

        None
    }
}

impl Into<bevy::prelude::Color> for &Color {
    fn into(self) -> bevy::prelude::Color {
        bevy::prelude::Color::srgba_u8(self.r, self.g, self.b, u8::MAX)
    }
}
