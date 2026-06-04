//! Porte logique NOT — inverseur : sortie active si l'entrée ne l'est pas.

use bevy::prelude::*;
use bevy_pancam::PanCam;

use crate::items::{
    apply_gate_logic, despawn_previews, spawn_preview, spawn_standard_gate, GateSet,
    BASE_WIDTH, ITEM_SCALE, Node, NodeKind, Preview,
};
use crate::plugins::{ItemDropped, MenuItems, SelectedItem};

/// Plugin Bevy pour la porte NOT.
pub struct Not;

const PICTURE: &str = "gates/NOT.png";

/// Identifiant de l'item NOT dans le menu de sélection.
#[derive(Resource, Deref)]
pub struct NotItemId(pub usize);

/// Composant marquant une entité comme porte NOT.
#[derive(Component)]
pub struct NotGate;

impl Plugin for Not {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_menu_item)
            .add_systems(Update, (manage_preview, on_drop))
            .add_systems(Update, compute_not_gate.in_set(GateSet::Compute));
    }
}

/// Calcule la sortie NOT : active si l'unique entrée est inactive.
fn compute_not_gate(
    gates: Query<&Children, With<NotGate>>,
    node_kinds: Query<&NodeKind>,
    mut nodes: Query<&mut Node>,
) {
    for children in &gates {
        apply_gate_logic(children, &node_kinds, &mut nodes, |active, _| active == 0);
    }
}

fn add_menu_item(mut commands: Commands, mut menu_items: ResMut<MenuItems>) {
    let id = menu_items.insert("NOT", PICTURE);
    commands.insert_resource(NotItemId(id));
}

fn manage_preview(
    mut commands: Commands,
    selected: Res<SelectedItem>,
    not_id: Res<NotItemId>,
    asset_server: Res<AssetServer>,
    preview_query: Query<Entity, With<Preview>>,
) {
    if !selected.is_changed() {
        return;
    }
    match selected.0 {
        Some(id) if id == **not_id => spawn_preview(&mut commands, &asset_server, PICTURE),
        _ => despawn_previews(&mut commands, &preview_query),
    }
}

fn on_drop(
    mut commands: Commands,
    mut reader: MessageReader<ItemDropped>,
    not_id: Res<NotItemId>,
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
        if event.id != **not_id {
            continue;
        }
        despawn_previews(&mut commands, &preview_query);
        let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, event.position) else {
            continue;
        };

        let x = BASE_WIDTH * ITEM_SCALE / 2.3;
        spawn_standard_gate(
            &mut commands,
            &asset_server,
            &mut meshes,
            &mut materials,
            PICTURE,
            world_pos,
            NotGate,
            &[Vec2::new(-x, 0.0)],
            Vec2::new(x, 0.0),
        );
    }
}
