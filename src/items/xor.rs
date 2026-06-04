//! Porte logique XOR — sortie active si exactement une entrée l'est.

use bevy::prelude::*;
use bevy_pancam::PanCam;

use crate::items::{
    apply_gate_logic, despawn_previews, spawn_preview, spawn_standard_gate,
    two_input_node_layout, GateSet, Node, NodeKind, Preview,
};
use crate::plugins::{ItemDropped, MenuItems, SelectedItem};

/// Plugin Bevy pour la porte XOR.
pub struct Xor;

const PICTURE: &str = "gates/XOR.png";

/// Identifiant de l'item XOR dans le menu de sélection.
#[derive(Resource, Deref)]
pub struct XorItemId(pub usize);

/// Composant marquant une entité comme porte XOR.
#[derive(Component)]
pub struct XorGate;

impl Plugin for Xor {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_menu_item)
            .add_systems(Update, (manage_preview, on_drop))
            .add_systems(Update, compute_xor_gate.in_set(GateSet::Compute));
    }
}

/// Calcule la sortie XOR : active si exactement une entrée est active.
fn compute_xor_gate(
    gates: Query<&Children, With<XorGate>>,
    node_kinds: Query<&NodeKind>,
    mut nodes: Query<&mut Node>,
) {
    for children in &gates {
        apply_gate_logic(children, &node_kinds, &mut nodes, |active, _| active == 1);
    }
}

fn add_menu_item(mut commands: Commands, mut menu_items: ResMut<MenuItems>) {
    let id = menu_items.insert("XOR", PICTURE);
    commands.insert_resource(XorItemId(id));
}

fn manage_preview(
    mut commands: Commands,
    selected: Res<SelectedItem>,
    xor_id: Res<XorItemId>,
    asset_server: Res<AssetServer>,
    preview_query: Query<Entity, With<Preview>>,
) {
    if !selected.is_changed() {
        return;
    }
    match selected.0 {
        Some(id) if id == **xor_id => spawn_preview(&mut commands, &asset_server, PICTURE),
        _ => despawn_previews(&mut commands, &preview_query),
    }
}

fn on_drop(
    mut commands: Commands,
    mut reader: MessageReader<ItemDropped>,
    xor_id: Res<XorItemId>,
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
        if event.id != **xor_id {
            continue;
        }
        despawn_previews(&mut commands, &preview_query);
        let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, event.position) else {
            continue;
        };
        let (inputs, output_pos) = two_input_node_layout();
        spawn_standard_gate(
            &mut commands,
            &asset_server,
            &mut meshes,
            &mut materials,
            PICTURE,
            world_pos,
            XorGate,
            &inputs,
            output_pos,
        );
    }
}
