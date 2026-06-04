//! Point d'entrée — assemble les plugins système et les items du circuit logique.

use bevy::prelude::*;

use crate::{
    items::Items,
    plugins::{Background, Camera, Menu, Minimap, Window},
};

pub mod items;
pub mod plugins;

fn main() {
    App::new()
        .add_plugins((Window, Camera, Background, Minimap, Menu))
        .add_plugins(Items)
        .run();
}
