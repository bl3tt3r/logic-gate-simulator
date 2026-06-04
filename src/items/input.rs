//! Source d'entrée clavier — émet un signal tant qu'une touche est maintenue.

use bevy::prelude::*;
use bevy_pancam::PanCam;

use crate::items::{
    BASE_HEIGHT, BASE_WIDTH, ITEM_SCALE, Item, NODE_RADIUS, Node, NodeKind, Preview,
    despawn_previews, spawn_preview,
};
use crate::plugins::{ItemDropped, MenuItems, SelectedItem};

/// Plugin Bevy pour l'item Input (source clavier).
pub struct Input;

const PICTURE: &str = "gates/INPUT.png";
const DOUBLE_CLICK_SECS: f64 = 0.3;

/// Identifiant de l'item Input dans le menu de sélection.
#[derive(Resource, Deref)]
pub struct InputItemId(pub usize);

/// État d'une porte d'entrée : touche liée et mode édition.
#[derive(Component)]
pub struct InputGate {
    /// Touche clavier déclenchant le signal, ou `None` si non configurée.
    pub bound_key: Option<KeyCode>,
    /// `true` pendant la capture d'une nouvelle touche.
    pub editing: bool,
}

/// Marque le label textuel affichant la touche liée.
#[derive(Component)]
struct KeyLabel;

impl Plugin for Input {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_menu_item).add_systems(
            Update,
            (
                manage_preview,
                on_drop,
                handle_double_click,
                capture_key_binding,
                update_key_label,
            ),
        );
    }
}

fn add_menu_item(mut commands: Commands, mut menu_items: ResMut<MenuItems>) {
    let id = menu_items.insert("Input", PICTURE);
    commands.insert_resource(InputItemId(id));
}

fn manage_preview(
    mut commands: Commands,
    selected: Res<SelectedItem>,
    item_id: Res<InputItemId>,
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
    item_id: Res<InputItemId>,
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

        commands
            .spawn((
                Sprite {
                    image: asset_server.load(PICTURE),
                    custom_size: Some(Vec2::new(w, h)),
                    ..default()
                },
                Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                Item,
                InputGate {
                    bound_key: None,
                    editing: false,
                },
            ))
            .with_children(|parent| {
                let circle = meshes.add(Circle::new(NODE_RADIUS));
                let material = materials.add(Color::WHITE);
                parent.spawn((
                    Node { active: false },
                    NodeKind::Output,
                    Mesh2d(circle),
                    MeshMaterial2d(material),
                    Transform::from_xyz(w / 2.8, 0.0, 1.0),
                ));
                parent.spawn((
                    Text2d::new("?"),
                    TextFont {
                        font_size: 20.0 * ITEM_SCALE,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(-(w / 8.0), 0.0, 2.0),
                    KeyLabel,
                ));
            });
    }
}

/// Détecte le double-clic sur une porte Input pour entrer en mode édition de touche.
fn handle_double_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    mut gates: Query<(Entity, &Transform, &Sprite, &mut InputGate)>,
    mut last_click: Local<(Option<Entity>, f64)>,
    time: Res<Time>,
) {
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

    let now = time.elapsed_secs_f64();

    for (entity, transform, sprite, mut gate) in &mut gates {
        let size = sprite.custom_size.unwrap_or(Vec2::ONE);
        let pos = transform.translation.truncate();
        let half = size / 2.0;

        let hovered = cursor_world.x >= pos.x - half.x
            && cursor_world.x <= pos.x + half.x
            && cursor_world.y >= pos.y - half.y
            && cursor_world.y <= pos.y + half.y;

        if hovered {
            let is_double = last_click.0 == Some(entity) && now - last_click.1 < DOUBLE_CLICK_SECS;
            if is_double {
                gate.editing = true;
                *last_click = (None, 0.0);
            } else {
                *last_click = (Some(entity), now);
            }
            return;
        }
    }

    // Clic en dehors : ferme le mode édition sur toutes les portes
    for (_, _, _, mut gate) in &mut gates {
        gate.editing = false;
    }
}

/// Capture la prochaine touche pressée et la lie à la porte en cours d'édition.
fn capture_key_binding(keys: Res<ButtonInput<KeyCode>>, mut gates: Query<&mut InputGate>) {
    for mut gate in &mut gates {
        if !gate.editing {
            continue;
        }
        if let Some(key) = keys.get_just_pressed().next() {
            gate.bound_key = Some(*key);
            gate.editing = false;
        }
    }
}

/// Met à jour l'affichage du label de touche selon l'état de la porte.
fn update_key_label(
    gates: Query<(&InputGate, &Children)>,
    mut labels: Query<(&mut Text2d, &mut TextColor), With<KeyLabel>>,
) {
    for (gate, children) in &gates {
        for child in children.iter() {
            let Ok((mut text, mut color)) = labels.get_mut(child) else {
                continue;
            };
            if gate.editing {
                text.0 = "_".to_string();
                color.0 = Color::srgb(0.4, 0.8, 1.0);
            } else {
                text.0 = gate.bound_key.map_or("?".to_string(), keycode_label);
                color.0 = Color::WHITE;
            }
        }
    }
}

/// Convertit un `KeyCode` en étiquette courte affichable (3 caractères max).
fn keycode_label(key: KeyCode) -> String {
    match key {
        KeyCode::Space => "SPC".to_string(),
        KeyCode::Enter => "ENT".to_string(),
        KeyCode::Backspace => "BSP".to_string(),
        KeyCode::ArrowUp => "↑".to_string(),
        KeyCode::ArrowDown => "↓".to_string(),
        KeyCode::ArrowLeft => "←".to_string(),
        KeyCode::ArrowRight => "→".to_string(),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "SHF".to_string(),
        KeyCode::ControlLeft | KeyCode::ControlRight => "CTL".to_string(),
        KeyCode::AltLeft | KeyCode::AltRight => "ALT".to_string(),
        _ => format!("{:?}", key)
            .replace("Key", "")
            .replace("Digit", "")
            .chars()
            .take(3)
            .collect(),
    }
}
