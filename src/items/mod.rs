//! Module principal des items du circuit logique.
//!
//! Contient les composants, ressources, systèmes partagés et fonctions
//! utilitaires utilisés par toutes les portes logiques.

use bevy::prelude::*;
use bevy_pancam::PanCam;

pub mod and;
pub mod input;
pub mod light;
pub mod nand;
pub mod nor;
pub mod not;
pub mod or;
pub mod xnor;
pub mod xor;

pub use and::*;
pub use input::*;
pub use light::*;
pub use nand::*;
pub use nor::*;
pub use not::*;
pub use or::*;
pub use xnor::*;
pub use xor::*;

/// Largeur de base d'une image de porte avant mise à l'échelle.
pub const BASE_WIDTH: f32 = 120.0;
/// Hauteur de base d'une image de porte avant mise à l'échelle.
pub const BASE_HEIGHT: f32 = 70.0;
/// Facteur d'échelle appliqué aux items lors du placement.
pub const ITEM_SCALE: f32 = 1.3;
/// Rayon des cercles de connexion (nœuds).
pub const NODE_RADIUS: f32 = 5.0;
/// Épaisseur d'un fil électrique en pixels monde.
const WIRE_THICKNESS: f32 = 2.0;
/// Distance maximale (pixels monde) pour sélectionner un nœud au clic.
const NODE_HIT_RADIUS: f32 = 10.0;

// ── System sets ──────────────────────────────────────────────────────────────

/// Ordonnancement des systèmes logiques sur chaque frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GateSet {
    /// Réinitialisation, sources clavier et premier passage des fils.
    Update,
    /// Calcul logique de chaque porte (AND, OR, …).
    Compute,
    /// Second passage des fils et rendu.
    Render,
}

// ── Plugin ───────────────────────────────────────────────────────────────────

/// Plugin principal qui enregistre toutes les portes logiques et les systèmes
/// de simulation de circuit.
pub struct Items;

impl Plugin for Items {
    fn build(&self, app: &mut App) {
        app.add_plugins((Input, And, Or, Nand, Nor, Not, Xor, Xnor, Light))
            .insert_resource(DraggedItem::default())
            .insert_resource(PendingWire::default())
            .configure_sets(
                Update,
                (GateSet::Update, GateSet::Compute, GateSet::Render).chain(),
            )
            .add_systems(Startup, setup_wire_mesh)
            .add_systems(
                Update,
                (
                    drag_items,
                    update_preview_position,
                    handle_wire_drawing,
                    delete_dragged_item,
                )
                    .in_set(GateSet::Update),
            )
            .add_systems(Update, update_signals.in_set(GateSet::Update))
            .add_systems(
                Update,
                (
                    propagate_wires,
                    update_node_color,
                    update_wire_visuals,
                    draw_pending_wire_gizmo,
                )
                    .chain()
                    .in_set(GateSet::Render),
            );
    }
}

// ── Components & Resources ───────────────────────────────────────────────────

/// Marque une entité comme item posable sur le circuit.
#[derive(Component)]
pub struct Item;

/// Marque l'entité fantôme qui suit le curseur avant placement.
#[derive(Component)]
pub struct Preview;

/// Nœud de connexion d'une porte (entrée ou sortie).
#[derive(Component)]
pub struct Node {
    /// `true` si le nœud porte un signal actif (niveau haut).
    pub active: bool,
}

/// Type d'un nœud de connexion.
#[derive(Component, PartialEq, Eq, Clone, Copy)]
pub enum NodeKind {
    /// Reçoit un signal d'une autre porte via un fil.
    Input,
    /// Émet le signal calculé par la porte.
    Output,
}

/// Connexion électrique entre deux nœuds.
#[derive(Component)]
pub struct Wire {
    /// Première extrémité du fil.
    pub a: Entity,
    /// Seconde extrémité du fil.
    pub b: Entity,
}

/// Item en cours de déplacement et offset souris au moment du clic.
#[derive(Resource, Default)]
struct DraggedItem(Option<(Entity, Vec2)>);

/// Nœud de départ d'un fil en cours de création.
#[derive(Resource, Default)]
struct PendingWire(Option<Entity>);

/// Handle vers le mesh rectangulaire réutilisé par tous les fils.
#[derive(Resource)]
struct WireBaseMesh(Handle<Mesh>);

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Crée l'aperçu fantôme d'un item sous le curseur.
pub fn spawn_preview(commands: &mut Commands, asset_server: &AssetServer, picture: &'static str) {
    let w = BASE_WIDTH * ITEM_SCALE;
    let h = BASE_HEIGHT * ITEM_SCALE;
    commands.spawn((
        Sprite {
            image: asset_server.load(picture),
            custom_size: Some(Vec2::new(w, h)),
            color: Color::srgba(1.0, 1.0, 1.0, 0.4),
            ..default()
        },
        Transform::from_xyz(f32::MIN, f32::MIN, 10.0),
        Preview,
    ));
}

/// Supprime toutes les entités de prévisualisation existantes.
pub fn despawn_previews(commands: &mut Commands, query: &Query<Entity, With<Preview>>) {
    for entity in query {
        commands.entity(entity).despawn();
    }
}

/// Applique la logique booléenne d'une porte à ses nœuds enfants.
///
/// `logic` reçoit `(active_count, total_inputs)` et renvoie l'état de la sortie.
pub fn apply_gate_logic(
    children: &Children,
    node_kinds: &Query<&NodeKind>,
    nodes: &mut Query<&mut Node>,
    logic: impl Fn(usize, usize) -> bool,
) {
    let mut inputs: Vec<Entity> = Vec::new();
    let mut output: Option<Entity> = None;

    for child in children.iter() {
        match node_kinds.get(child).ok() {
            Some(NodeKind::Input) => inputs.push(child),
            Some(NodeKind::Output) => output = Some(child),
            _ => {}
        }
    }

    let active_count = inputs
        .iter()
        .filter(|&&c| nodes.get(c).is_ok_and(|n| n.active))
        .count();

    if let Some(out) = output {
        if let Ok(mut node) = nodes.get_mut(out) {
            node.active = logic(active_count, inputs.len());
        }
    }
}

/// Instancie une porte logique standard avec ses nœuds d'entrée et de sortie.
pub fn spawn_standard_gate<G: Component>(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    picture: &'static str,
    world_pos: Vec2,
    gate: G,
    input_positions: &[Vec2],
    output_pos: Vec2,
) {
    let w = BASE_WIDTH * ITEM_SCALE;
    let h = BASE_HEIGHT * ITEM_SCALE;
    let circle = meshes.add(Circle::new(NODE_RADIUS));

    commands
        .spawn((
            Sprite {
                image: asset_server.load(picture),
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
            Item,
            gate,
        ))
        .with_children(|parent| {
            for &pos in input_positions {
                parent.spawn((
                    Node { active: false },
                    NodeKind::Input,
                    Mesh2d(circle.clone()),
                    MeshMaterial2d(materials.add(Color::WHITE)),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                ));
            }
            parent.spawn((
                Node { active: false },
                NodeKind::Output,
                Mesh2d(circle.clone()),
                MeshMaterial2d(materials.add(Color::WHITE)),
                Transform::from_xyz(output_pos.x, output_pos.y, 1.0),
            ));
        });
}

/// Retourne les positions des nœuds pour une porte standard à deux entrées.
///
/// Renvoie `(positions_entrées, position_sortie)`.
pub fn two_input_node_layout() -> ([Vec2; 2], Vec2) {
    let w = BASE_WIDTH * ITEM_SCALE;
    let h = BASE_HEIGHT * ITEM_SCALE;
    (
        [
            Vec2::new(-(w / 2.3), h / 5.0),
            Vec2::new(-(w / 2.3), -(h / 5.0)),
        ],
        Vec2::new(w / 2.3, 0.0),
    )
}

// ── Setup ────────────────────────────────────────────────────────────────────

fn setup_wire_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let handle = meshes.add(Rectangle::new(1.0, 1.0));
    commands.insert_resource(WireBaseMesh(handle));
}

// ── Drag ─────────────────────────────────────────────────────────────────────

fn drag_items(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    mut items: Query<(Entity, &mut Transform, &Sprite), With<Item>>,
    mut dragged: ResMut<DraggedItem>,
    mut pancam_query: Query<&mut PanCam>,
    nodes: Query<&GlobalTransform, With<Node>>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };
    let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        let on_node = nodes
            .iter()
            .any(|gt| gt.translation().truncate().distance(cursor_world) <= NODE_HIT_RADIUS);

        if !on_node {
            for (entity, transform, sprite) in &items {
                let size = sprite.custom_size.unwrap_or(Vec2::ONE);
                let pos = transform.translation.truncate();
                let half = size / 2.0;
                if cursor_world.x >= pos.x - half.x
                    && cursor_world.x <= pos.x + half.x
                    && cursor_world.y >= pos.y - half.y
                    && cursor_world.y <= pos.y + half.y
                {
                    dragged.0 = Some((entity, cursor_world - pos));
                    if let Ok(mut pancam) = pancam_query.single_mut() {
                        pancam.enabled = false;
                    }
                    break;
                }
            }
        }
    }

    if mouse.pressed(MouseButton::Left)
        && let Some((entity, offset)) = dragged.0
        && let Ok((_, mut transform, _)) = items.get_mut(entity)
    {
        let new_pos = cursor_world - offset;
        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
    }

    if mouse.just_released(MouseButton::Left)
        && dragged.0.take().is_some()
        && let Ok(mut pancam) = pancam_query.single_mut()
    {
        pancam.enabled = true;
    }
}

// ── Delete ────────────────────────────────────────────────────────────────────

fn delete_dragged_item(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut dragged: ResMut<DraggedItem>,
    mut pending_wire: ResMut<PendingWire>,
    children_query: Query<&Children>,
    wires: Query<(Entity, &Wire)>,
    mut pancam_query: Query<&mut PanCam>,
) {
    if !keys.just_pressed(KeyCode::Delete) {
        return;
    }

    let Some((item_entity, _)) = dragged.0.take() else {
        return;
    };

    if let Ok(mut pancam) = pancam_query.single_mut() {
        pancam.enabled = true;
    }

    let child_nodes: Vec<Entity> = children_query
        .get(item_entity)
        .map(|ch| ch.iter().collect())
        .unwrap_or_default();

    // Annule le fil en cours si son nœud de départ appartient à l'item supprimé
    if let Some(start) = pending_wire.0 {
        if child_nodes.contains(&start) {
            pending_wire.0 = None;
        }
    }

    for (wire_entity, wire) in &wires {
        if child_nodes.contains(&wire.a) || child_nodes.contains(&wire.b) {
            commands.entity(wire_entity).despawn();
        }
    }

    for child in child_nodes {
        commands.entity(child).despawn();
    }
    commands.entity(item_entity).despawn();
}

// ── Preview ───────────────────────────────────────────────────────────────────

fn update_preview_position(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    mut preview_query: Query<&mut Transform, With<Preview>>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };
    let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) else {
        return;
    };

    for mut transform in &mut preview_query {
        transform.translation.x = cursor_world.x;
        transform.translation.y = cursor_world.y;
    }
}

// ── Wire drawing ──────────────────────────────────────────────────────────────

fn handle_wire_drawing(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    nodes: Query<(Entity, &GlobalTransform), With<Node>>,
    mut pending: ResMut<PendingWire>,
    wire_mesh: Res<WireBaseMesh>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Clic droit : annule le fil en cours de création
    if mouse.just_pressed(MouseButton::Right) {
        pending.0 = None;
        return;
    }

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };
    let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) else {
        return;
    };

    let hit = nodes
        .iter()
        .filter(|(_, gt)| gt.translation().truncate().distance(cursor_world) <= NODE_HIT_RADIUS)
        .min_by(|(_, a), (_, b)| {
            let da = a.translation().truncate().distance(cursor_world);
            let db = b.translation().truncate().distance(cursor_world);
            da.partial_cmp(&db).unwrap()
        })
        .map(|(e, _)| e);

    let Some(clicked) = hit else {
        pending.0 = None;
        return;
    };

    match pending.0 {
        None => {
            pending.0 = Some(clicked);
        }
        Some(start) if start != clicked => {
            commands.spawn((
                Wire {
                    a: start,
                    b: clicked,
                },
                Mesh2d(wire_mesh.0.clone()),
                MeshMaterial2d(materials.add(Color::WHITE)),
                Transform::default(),
            ));
            pending.0 = None;
        }
        _ => {
            pending.0 = None;
        }
    }
}

// ── Signals ───────────────────────────────────────────────────────────────────

/// Réinitialise les entrées, applique les sources clavier et effectue
/// le premier passage de propagation des fils.
pub fn update_signals(
    keys: Res<ButtonInput<KeyCode>>,
    input_gates: Query<(&InputGate, &Children)>,
    wires: Query<&Wire>,
    mut nodes: Query<(&mut Node, &NodeKind)>,
) {
    for (mut node, kind) in &mut nodes {
        if *kind == NodeKind::Input {
            node.active = false;
        }
    }

    for (gate, children) in &input_gates {
        let active = gate.bound_key.is_some_and(|k| keys.pressed(k));
        for child in children.iter() {
            if let Ok((mut node, _)) = nodes.get_mut(child) {
                node.active = active;
            }
        }
    }

    apply_wire_signals(&wires, &mut nodes);
}

/// Second passage de propagation après le calcul des portes, pour transmettre
/// les nouvelles sorties vers les entrées en aval.
pub fn propagate_wires(wires: Query<&Wire>, mut nodes: Query<(&mut Node, &NodeKind)>) {
    apply_wire_signals(&wires, &mut nodes);
}

/// Propage les signaux des nœuds Output vers les nœuds Input connectés.
///
/// La direction est toujours Output → Input, quel que soit l'ordre de connexion.
fn apply_wire_signals(wires: &Query<&Wire>, nodes: &mut Query<(&mut Node, &NodeKind)>) {
    let signals: Vec<(Entity, bool)> = wires
        .iter()
        .filter_map(|wire| {
            let a_is_out = nodes
                .get(wire.a)
                .is_ok_and(|(_, k)| *k == NodeKind::Output);
            let b_is_out = nodes
                .get(wire.b)
                .is_ok_and(|(_, k)| *k == NodeKind::Output);
            match (a_is_out, b_is_out) {
                (true, false) => {
                    let active = nodes.get(wire.a).is_ok_and(|(n, _)| n.active);
                    Some((wire.b, active))
                }
                (false, true) => {
                    let active = nodes.get(wire.b).is_ok_and(|(n, _)| n.active);
                    Some((wire.a, active))
                }
                _ => None,
            }
        })
        .collect();

    for (target, active) in signals {
        if active {
            if let Ok((mut node, _)) = nodes.get_mut(target) {
                node.active = true;
            }
        }
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

fn update_node_color(
    query: Query<(&Node, &MeshMaterial2d<ColorMaterial>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (node, handle) in &query {
        if let Some(mat) = materials.get_mut(handle) {
            mat.color = if node.active {
                Color::from(Srgba::RED)
            } else {
                Color::WHITE
            };
        }
    }
}

fn update_wire_visuals(
    mut wires: Query<(Entity, &Wire, &MeshMaterial2d<ColorMaterial>)>,
    nodes: Query<(&GlobalTransform, &Node)>,
    mut transforms: Query<&mut Transform>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (wire_entity, wire, mat_handle) in &mut wires {
        let Ok((gt_a, node_a)) = nodes.get(wire.a) else {
            continue;
        };
        let Ok((gt_b, node_b)) = nodes.get(wire.b) else {
            continue;
        };

        let pos_a = gt_a.translation().truncate();
        let pos_b = gt_b.translation().truncate();
        let delta = pos_b - pos_a;
        let length = delta.length();
        if length < 0.001 {
            continue;
        }

        if let Ok(mut t) = transforms.get_mut(wire_entity) {
            t.translation = ((pos_a + pos_b) / 2.0).extend(5.0);
            t.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
            t.scale = Vec3::new(length, WIRE_THICKNESS, 1.0);
        }

        if let Some(mat) = materials.get_mut(mat_handle) {
            mat.color = if node_a.active || node_b.active {
                Color::from(Srgba::RED)
            } else {
                Color::WHITE
            };
        }
    }
}

fn draw_pending_wire_gizmo(
    pending: Res<PendingWire>,
    nodes: Query<&GlobalTransform, With<Node>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    mut gizmos: Gizmos,
) {
    let Some(start_entity) = pending.0 else {
        return;
    };
    let Ok(start_gt) = nodes.get(start_entity) else {
        return;
    };
    let start_pos = start_gt.translation().truncate();

    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };
    let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) else {
        return;
    };

    gizmos.line_2d(start_pos, cursor_world, Color::srgba(0.4, 0.8, 1.0, 0.8));
}
