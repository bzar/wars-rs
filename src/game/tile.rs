use crate::game::Tile;

impl Default for Tile {
    fn default() -> Tile {
        use crate::model;
        Tile {
            terrain: model::Terrain::Road,
            terrain_subtype_id: 0,
            owner: None,
            x: 0,
            y: 0,
            unit: None,
            capture_points: model::MAX_CAPTURE_POINTS
        }
    }
}


