//! Fond de scène avec grille rendue par un shader de matériau personnalisé.

use bevy::{
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

const BACKGROUND_GRID_SHADER: &str = "shaders/background_grid.wgsl";

/// Plugin Bevy gérant le fond quadrillé de la scène.
pub struct Background;

impl Plugin for Background {
    fn build(&self, app: &mut App) {
        app.insert_resource(BackgroundSettings::default())
            .add_plugins(Material2dPlugin::<BackgroundMaterial>::default())
            .add_systems(Startup, spawn_background);
    }
}

fn spawn_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    settings: Res<BackgroundSettings>,
) {
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_corners(settings.size.min, settings.size.max))),
        MeshMaterial2d(materials.add(BackgroundMaterial {
            background_color: settings.background_color,
            line_space: settings.line_space,
            line_thickness: settings.line_thickness,
            line_color: settings.line_color,
        })),
        Transform {
            // Positionné derrière tous les autres éléments
            translation: Vec3::new(0.0, 0.0, -1.0),
            ..default()
        },
    ));
}

// ── Paramètres ────────────────────────────────────────────────────────────────

/// Paramètres de la grille de fond, exposés comme ressource Bevy.
#[derive(Resource)]
pub struct BackgroundSettings {
    /// Emprise totale du fond (définit les limites du monde).
    pub size: Rect,
    /// Couleur de remplissage des cases.
    pub background_color: LinearRgba,
    /// Espacement entre deux lignes de grille.
    line_space: f32,
    /// Épaisseur des lignes de grille.
    line_thickness: f32,
    /// Couleur des lignes de grille.
    line_color: LinearRgba,
}

impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            size: Rect::new(-3500.0, -2500.0, 3500.0, 2500.0),
            background_color: Srgba::rgb_u8(51, 51, 51).into(),
            line_space: 100.0,
            line_thickness: 1.0,
            line_color: Srgba::rgb_u8(65, 65, 65).into(),
        }
    }
}

// ── Shader ────────────────────────────────────────────────────────────────────

/// Matériau WGSL pour le fond quadrillé.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BackgroundMaterial {
    #[uniform(0)]
    pub background_color: LinearRgba,
    #[uniform(1)]
    pub line_space: f32,
    #[uniform(2)]
    pub line_thickness: f32,
    #[uniform(3)]
    pub line_color: LinearRgba,
}

impl Material2d for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        BACKGROUND_GRID_SHADER.into()
    }
}
