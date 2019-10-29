extern crate wars;

use wars::game::{Map, Game};
use std::env::args;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let file_path = args()
        .skip(1) // executable name
        .next() // first command line argument
        .expect("First argument should be map file path");
    let mut map_json = String::new();
    File::open(&file_path)
        .expect("Error opening file")
        .read_to_string(&mut map_json)
        .expect("Error reading file");
    let map = Map::from_json(&map_json).expect("Error loading map");
    let players: Vec<_> = map.player_numbers();

    let game = Game::new(map, &players);
}

