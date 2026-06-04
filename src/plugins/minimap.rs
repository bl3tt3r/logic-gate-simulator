//! Minimap dans le coin supérieur droit avec indicateur de zone visible et navigation au clic.

use crate::plugins::{BackgroundSettings, WORLD_RENDER_LAYER};
use bevy::{
    camera::{ScalingMode, Viewport, visibility::RenderLayers},
    color::palettes::css::{GRAY, RED},
    prelude::*,
    window::WindowResized,
};
use bevy_pancam::PanCam;

/// Calque de rendu réservé à la minimap.
pub const MINIMAP_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

/// Composant marquant la caméra de la minimap.
#[derive(Component)]
pub struct Minimap;

/// Groupe de gizmos pour l'indicateur de zone visible sur la minimap.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct MinimapZoneIndicator;

/// Plugin Bevy gérant la minimap et la navigation par clic.
impl Plugin for Minimap {
    fn build(&self, app: &mut App) {
        app.insert_resource(MinimapSettings::default())
            .init_gizmo_group::<MinimapZoneIndicator>()
            .add_systems(
                Startup,
                (
                    spawn_minimap_camera,
                    setup_minimap,
                    setup_minimap_zone_indicator,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    lock_minimap_location,
                    update_minimap_zone_indicator,
                    handle_minimap_click,
                ),
            );
    }
}

fn spawn_minimap_camera(
    mut commands: Commands,
    background_settings: Res<BackgroundSettings>,
    minimap_settings: Res<MinimapSettings>,
) {
    let size = background_settings.size.size();
    commands.spawn((
        Camera2d,
        WORLD_RENDER_LAYER.union(&MINIMAP_RENDER_LAYER),
        Camera {
            order: 1,
            viewport: Some(Viewport {
                physical_position: UVec2::new(0, 0),
                physical_size: UVec2::new(
                    (size.x * minimap_settings.scale) as u32,
                    (size.y * minimap_settings.scale) as u32,
                ),
                ..default()
            }),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(
                size.x * minimap_settings.scale + minimap_settings.border_thickness * 2.0,
            ),
            height: Val::Px(
                size.y * minimap_settings.scale + minimap_settings.border_thickness * 2.0,
            ),
            border: UiRect::all(Val::Px(minimap_settings.border_thickness)),
            ..default()
        },
        BorderColor::all(minimap_settings.border_color),
        BackgroundColor(Color::NONE),
        Minimap,
    ));
}

/// Configure la projection orthographique de la minimap pour couvrir l'ensemble du monde.
fn setup_minimap(
    background_settings: Res<BackgroundSettings>,
    mut query: Query<&mut Projection, With<Minimap>>,
) {
    let size = background_settings.size.size();
    for mut projection in &mut query {
        *projection = Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: size.x,
                height: size.y,
            },
            ..OrthographicProjection::default_2d()
        });
    }
}

/// Repositionne la minimap dans le coin supérieur droit après chaque redimensionnement de fenêtre.
fn lock_minimap_location(
    mut resize_reader: MessageReader<WindowResized>,
    window: Query<&Window>,
    minimap_settings: Res<MinimapSettings>,
    mut query: Query<(&mut Camera, &mut Node), With<Minimap>>,
) {
    if !resize_reader.is_empty() {
        resize_reader.clear();

        let Ok(window) = window.single() else { return };
        let Ok((mut camera, mut node)) = query.single_mut() else {
            return;
        };
        let Some(viewport) = &mut camera.viewport else {
            return;
        };

        let width = window.width() as u32;
        let minimap_width = viewport.physical_size.x;

        debug!(
            "window width: {}, minimap width: {}, margin: {}",
            width, minimap_width, minimap_settings.margin
        );

        viewport.physical_position = UVec2 {
            x: (width as i32 - (minimap_width as i32 + minimap_settings.margin as i32)).max(0)
                as u32,
            y: minimap_settings.margin,
        };

        node.left = Val::Px(
            width as f32
                - minimap_width as f32
                - minimap_settings.margin as f32
                - minimap_settings.border_thickness,
        );
        node.top =
            Val::Px(minimap_settings.margin as f32 - minimap_settings.border_thickness);
    }
}

// ── Navigation ────────────────────────────────────────────────────────────────

/// Téléporte la caméra principale à la position cliquée sur la minimap.
fn handle_minimap_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window: Query<&Window>,
    minimap_camera_query: Query<(&Camera, &GlobalTransform), With<Minimap>>,
    background_settings: Res<BackgroundSettings>,
    mut query_camera: Query<&mut Transform, With<PanCam>>,
) {
    let Ok(mut transform) = query_camera.single_mut() else {
        return;
    };
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor_pos) = window
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
    else {
        return;
    };

    let Ok((camera, camera_transform)) = minimap_camera_query.single() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    if !background_settings.size.contains(world_pos) {
        return;
    }

    transform.translation.x = world_pos.x;
    transform.translation.y = world_pos.y;
}

// ── Indicateur de zone ────────────────────────────────────────────────────────

/// Assigne le calque de rendu de la minimap aux gizmos de l'indicateur.
fn setup_minimap_zone_indicator(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<MinimapZoneIndicator>();
    config.render_layers = MINIMAP_RENDER_LAYER;
}

/// Dessine le rectangle indiquant la zone actuellement visible par la caméra principale.
fn update_minimap_zone_indicator(
    query_camera: Query<(&Transform, &Projection), With<PanCam>>,
    mut gizmos: Gizmos<MinimapZoneIndicator>,
) {
    let Ok((cam_transform, projection)) = query_camera.single() else {
        return;
    };

    if let Projection::Orthographic(ortho) = projection {
        let area = ortho.area;
        let pos = cam_transform.translation.truncate();
        let size = Vec2::new(area.max.x - area.min.x, area.max.y - area.min.y);
        gizmos.rect_2d(Isometry2d::from_translation(pos), size, Color::Srgba(RED));
    }
}

// ── Paramètres ────────────────────────────────────────────────────────────────

/// Paramètres d'affichage de la minimap.
#[derive(Resource)]
pub struct MinimapSettings {
    /// Facteur de réduction par rapport à la taille du monde.
    pub scale: f32,
    /// Marge en pixels par rapport au bord de la fenêtre.
    pub margin: u32,
    /// Couleur de la bordure de la minimap.
    pub border_color: Color,
    /// Épaisseur de la bordure en pixels.
    pub border_thickness: f32,
}

impl Default for MinimapSettings {
    fn default() -> Self {
        Self {
            scale: 0.03,
            margin: 10,
            border_color: Color::Srgba(GRAY),
            border_thickness: 1.0,
        }
    }
}
