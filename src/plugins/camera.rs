//! Caméra principale avec panoramique et zoom, contrainte aux limites du monde.

use crate::plugins::{BackgroundSettings, MenuSettings};
use bevy::{camera::visibility::RenderLayers, prelude::*};
use bevy_pancam::{DirectionKeys, PanCam, PanCamPlugin};

/// Calque de rendu de la scène principale (portes, fils, fond).
pub const WORLD_RENDER_LAYER: RenderLayers = RenderLayers::layer(0);

/// Plugin Bevy gérant la caméra de jeu avec panoramique et zoom.
pub struct Camera;

impl Plugin for Camera {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanCamPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(PreUpdate, toggle_pancam_in_viewport)
            .add_systems(PostUpdate, clamp_camera_in_space);
    }
}

fn spawn_camera(mut commands: Commands, settings: Res<BackgroundSettings>) {
    commands.spawn((
        Camera2d,
        WORLD_RENDER_LAYER,
        PanCam {
            grab_buttons: vec![MouseButton::Middle],
            move_keys: DirectionKeys {
                up: vec![KeyCode::KeyW],
                down: vec![KeyCode::KeyS],
                left: vec![KeyCode::KeyA],
                right: vec![KeyCode::KeyD],
            },
            speed: 800.,
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 1.,
            max_scale: 5.,
            min_x: settings.size.min.x,
            max_x: settings.size.max.x,
            min_y: settings.size.min.y,
            max_y: settings.size.max.y,
            mouse_wheel_sensitivity: 1.0,
            pinch_gesture_sensitivity: 1.0,
        },
    ));
}

/// Désactive le panoramique lorsque le curseur survole la barre de menu.
fn toggle_pancam_in_viewport(
    windows: Query<&Window>,
    mut query_camera: Query<&mut PanCam>,
    menu_settings: Res<MenuSettings>,
) {
    let Ok(mut pancam) = query_camera.single_mut() else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    pancam.enabled = cursor.y < window.height() - menu_settings.height;
}

/// Empêche la caméra de sortir des limites définies par `BackgroundSettings`.
fn clamp_camera_in_space(
    settings: Res<BackgroundSettings>,
    mut query_camera: Query<(&mut Transform, &Projection), With<PanCam>>,
) {
    let Ok((mut transform, projection)) = query_camera.single_mut() else {
        return;
    };

    let Projection::Orthographic(ortho) = projection else {
        return;
    };

    let area = ortho.area;
    let size = Vec2::new(area.max.x - area.min.x, area.max.y - area.min.y);

    transform.translation.x = transform.translation.x.clamp(
        settings.size.min.x + size.x / 2.0,
        settings.size.max.x - size.x / 2.0,
    );
    transform.translation.y = transform.translation.y.clamp(
        settings.size.min.y + size.y / 2.0,
        settings.size.max.y - size.y / 2.0,
    );
}
