//! Configuration de la fenêtre principale de l'application.

use bevy::{prelude::*, window::WindowResolution};

/// Plugin Bevy initialisant la fenêtre de l'application.
pub struct Window;

impl Plugin for Window {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(bevy::window::Window {
                title: "Logic Gate Simulator".to_string(),
                mode: bevy::window::WindowMode::Windowed,
                position: WindowPosition::Centered(MonitorSelection::Primary),
                resolution: WindowResolution::new(1500, 1200),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }
}
