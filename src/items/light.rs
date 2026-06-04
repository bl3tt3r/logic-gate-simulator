//! Sortie lumineuse — s'allume quand son entrée reçoit un signal actif.

use bevy::prelude::*;
use bevy_pancam::PanCam;

use crate::items::{
    despawn_previews, spawn_preview, GateSet, BASE_HEIGHT, BASE_WIDTH, ITEM_SCALE, NODE_RADIUS,
    Item, Node, NodeKind, Preview,
};
use crate::plugins::{ItemDropped, MenuItems, SelectedItem};

/// Plugin Bevy pour l'item Light (sortie lumineuse).
pub struct Light;

const PICTURE: &str = "gates/LIGHT.png";
const LIGHT_RADIUS: f32 = 42.0;

/// Identifiant de l'item Light dans le menu de sélection.
#[derive(Resource, Deref)]
pub struct OutputItemId(pub usize);

/// Composant marquant une entité comme sortie lumineuse.
#[derive(Component)]
pub struct OutputGate;

/// Marque le cercle lumineux enfant de l'`OutputGate`.
#[derive(Component)]
pub struct OutputLight;

impl Plugin for Light {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_menu_item)
            .add_systems(Update, (manage_preview, on_drop))
            .add_systems(
                Update,
                update_output_light
                    .in_set(GateSet::Render)
                    .after(crate::items::propagate_wires),
            );
    }
}

/// Met à jour la couleur du cercle lumineux selon l'état du nœud d'entrée.
fn update_output_light(
    gates: Query<&Children, With<OutputGate>>,
    node_kinds: Query<&NodeKind>,
    nodes: Query<&Node>,
    lights: Query<&MeshMaterial2d<ColorMaterial>, With<OutputLight>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for children in &gates {
        let input_active = children
            .iter()
            .filter(|c| node_kinds.get(*c).ok() == Some(&NodeKind::Input))
            .any(|c| nodes.get(c).is_ok_and(|n| n.active));

        for child in children.iter() {
            if let Ok(mat_handle) = lights.get(child)
                && let Some(mat) = materials.get_mut(mat_handle)
            {
                mat.color = if input_active {
                    Color::srgb(1.0, 0.95, 0.1)
                } else {
                    Color::NONE
                };
            }
        }
    }
}

fn add_menu_item(mut commands: Commands, mut menu_items: ResMut<MenuItems>) {
    let id = menu_items.insert("Light", PICTURE);
    commands.insert_resource(OutputItemId(id));
}

fn manage_preview(
    mut commands: Commands,
    selected: Res<SelectedItem>,
    item_id: Res<OutputItemId>,
    asset_server: Res<AssetServer>,
    preview_query: Query<Entity, With<Preview>>,
) {
    if !selected.is_changed() {
        return;
    }
    match selected.0 {
        Some(id) if id == **item_id => spawn_preview(&mut commands, &asset_server, PICTURE),
        _ => despawn_previews(&mut commands, &preview_query),
    }
}

fn on_drop(
    mut commands: Commands,
    mut reader: MessageReader<ItemDropped>,
    item_id: Res<OutputItemId>,
    asset_server: Res<AssetServer>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    preview_query: Query<Entity, With<Preview>>,
) {
    let Ok((camera, cam_transform)) = camera_query.single() else {
        return;
    };

    for event in reader.read() {
        if event.id != **item_id {
            continue;
        }
        despawn_previews(&mut commands, &preview_query);
        let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, event.position) else {
            continue;
        };

        let w = BASE_WIDTH * ITEM_SCALE;
        let h = BASE_HEIGHT * ITEM_SCALE;
        let input_pos = Vec2::new(-(w / 2.0), 0.0);
        let light_pos = Vec2::new(w * 0.205, 0.0);

        commands
            .spawn((
                Sprite {
                    image: asset_server.load(PICTURE),
                    custom_size: Some(Vec2::new(w, h)),
                    ..default()
                },
                Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                Item,
                OutputGate,
            ))
            .with_children(|parent| {
                let light = meshes.add(Circle::new(LIGHT_RADIUS));
                parent.spawn((
                    OutputLight,
                    Mesh2d(light),
                    MeshMaterial2d(materials.add(Color::srgb(0.15, 0.13, 0.03))),
                    Transform::from_xyz(light_pos.x, light_pos.y, 1.0),
                ));

                let node_circle = meshes.add(Circle::new(NODE_RADIUS));
                parent.spawn((
                    Node { active: false },
                    NodeKind::Input,
                    Mesh2d(node_circle),
                    MeshMaterial2d(materials.add(Color::WHITE)),
                    Transform::from_xyz(input_pos.x, input_pos.y, 1.0),
                ));
            });
    }
}
