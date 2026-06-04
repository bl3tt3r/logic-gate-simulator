//! Barre de menu horizontale permettant de sélectionner et déposer des items.

use std::collections::HashMap;

use bevy::{camera::visibility::RenderLayers, input::mouse::AccumulatedMouseScroll, prelude::*};

use crate::plugins::BackgroundSettings;

/// Calque de rendu du HUD (menu, minimap labels).
pub const HUD_RENDER_LAYER: RenderLayers = RenderLayers::layer(2);

fn next_id() -> usize {
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Composant marquant le conteneur principal du menu.
#[derive(Component)]
pub struct Menu;

/// Données d'affichage d'un item dans le menu.
pub struct MenuItem {
    icon: String,
    name: String,
}

/// Registre de tous les items disponibles dans le menu.
#[derive(Resource, Default)]
pub struct MenuItems {
    items: HashMap<usize, MenuItem>,
}

impl MenuItems {
    /// Insère un nouvel item dans le menu et retourne son identifiant unique.
    pub fn insert(&mut self, name: &str, icon: &str) -> usize {
        let id = next_id();
        self.items.insert(
            id,
            MenuItem {
                icon: icon.to_string(),
                name: name.to_string(),
            },
        );
        id
    }
}

/// Composant marquant la caméra du HUD (rendu par-dessus la scène).
#[derive(Component)]
pub struct HudCamera;

/// Composant portant l'identifiant de l'item associé à un slot de menu.
#[derive(Component, Deref)]
pub struct ItemSlot(pub usize);

/// Identifiant de l'item actuellement sélectionné dans le menu.
#[derive(Resource, Default)]
pub struct SelectedItem(pub Option<usize>);

/// Message émis quand un item est déposé sur la scène depuis le menu.
#[derive(Message)]
pub struct ItemDropped {
    /// Identifiant de l'item déposé.
    pub id: usize,
    /// Position écran au moment du dépôt.
    pub position: Vec2,
}

/// Plugin Bevy gérant la barre de menu inférieure.
impl Plugin for Menu {
    fn build(&self, app: &mut App) {
        app.insert_resource(MenuSettings::default())
            .insert_resource(MenuItems::default())
            .insert_resource(SelectedItem::default())
            .add_message::<ItemDropped>()
            .add_systems(PostStartup, setup_menu)
            .add_systems(Update, (scroll_menu, hover_item_slot));
    }
}

fn setup_menu(
    mut commands: Commands,
    menu_settings: Res<MenuSettings>,
    menu_items: Res<MenuItems>,
    _bg_settings: Res<BackgroundSettings>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
            Camera2d,
            Camera {
                order: 2,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            HUD_RENDER_LAYER,
            HudCamera,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(menu_settings.height),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                overflow: Overflow::scroll_x(),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(menu_settings.column_gap)),
                column_gap: Val::Px(menu_settings.column_gap),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Menu,
        ))
        .with_children(|parent| {
            for (i, item) in &menu_items.items {
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            flex_shrink: 0.0,
                            row_gap: Val::Px(4.0),
                            padding: UiRect::all(Val::Px(menu_settings.font_size)),
                            border: UiRect::all(Val::Px(menu_settings.border_thickness)),
                            border_radius: BorderRadius::all(Val::Percent(5.0)),
                            ..default()
                        },
                        BorderColor::all(menu_settings.border_color),
                        BackgroundColor(menu_settings.background_color),
                        Interaction::default(),
                        ItemSlot(*i),
                    ))
                    .with_children(|col| {
                        col.spawn((
                            Node {
                                width: Val::Px(menu_settings.case_size.0),
                                height: Val::Px(menu_settings.case_size.1),
                                ..default()
                            },
                            ImageNode::new(asset_server.load(&item.icon)),
                        ));
                        col.spawn((
                            Text::new(item.name.to_string()),
                            TextFont {
                                font_size: menu_settings.font_size,
                                ..default()
                            },
                            TextColor(menu_settings.font_color),
                        ));
                    });
            }
        });
}

/// Fait défiler le menu horizontalement avec la molette de la souris.
fn scroll_menu(
    mut query: Query<(&mut ScrollPosition, &ComputedNode), With<Menu>>,
    scroll: Option<Res<AccumulatedMouseScroll>>,
    menu_settings: Res<MenuSettings>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.y < window.height() - menu_settings.height {
        return;
    }

    let Some(scroll) = scroll else { return };
    let Ok((mut scroll_pos, computed)) = query.single_mut() else {
        return;
    };

    let max_scroll = (computed.content_size().x - computed.size().x).max(0.0);
    scroll_pos.0.x = (scroll_pos.0.x - scroll.delta.y * 20.0).clamp(0.0, max_scroll);
}

/// Gère les états hover/press des slots de menu et émet `ItemDropped` au relâchement.
fn hover_item_slot(
    mut query: Query<
        (
            &Interaction,
            &mut BorderColor,
            &mut BackgroundColor,
            &Children,
            &ItemSlot,
        ),
        Changed<Interaction>,
    >,
    mut text_query: Query<&mut TextColor>,
    menu_settings: Res<MenuSettings>,
    mut selected_item: ResMut<SelectedItem>,
    mut dropped_events: MessageWriter<ItemDropped>,
    windows: Query<&Window>,
) {
    let cursor_pos = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .unwrap_or(Vec2::ZERO);

    for (interaction, mut border, mut bg, children, slot) in &mut query {
        let (border_color, bg_color, font_color) = match interaction {
            Interaction::Hovered | Interaction::Pressed => (
                menu_settings.hover_border_color,
                menu_settings.hover_background_color,
                menu_settings.font_hover_color,
            ),
            _ => (
                menu_settings.border_color,
                menu_settings.background_color,
                menu_settings.font_color,
            ),
        };

        border.set_all(border_color);
        bg.0 = bg_color;

        for child in children.iter() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = font_color;
            }
        }

        match interaction {
            Interaction::Pressed => {
                selected_item.0 = Some(**slot);
            }
            Interaction::Hovered | Interaction::None => {
                if let Some(id) = selected_item.0.take() {
                    dropped_events.write(ItemDropped {
                        id,
                        position: cursor_pos,
                    });
                }
            }
        }
    }
}

// ── Paramètres ────────────────────────────────────────────────────────────────

/// Paramètres visuels de la barre de menu.
#[derive(Resource)]
pub struct MenuSettings {
    /// Hauteur de la barre en pixels.
    pub height: f32,
    column_gap: f32,
    case_size: (f32, f32),
    font_size: f32,
    border_thickness: f32,
    border_color: Color,
    background_color: Color,
    hover_border_color: Color,
    hover_background_color: Color,
    font_color: Color,
    font_hover_color: Color,
}

impl Default for MenuSettings {
    fn default() -> Self {
        Self {
            height: 160.0,
            column_gap: 30.0,
            case_size: (120.0, 70.0),
            font_size: 11.0,
            border_thickness: 1.0,
            border_color: Srgba::rgba_u8(255, 255, 255, 30).into(),
            background_color: Srgba::rgba_u8(255, 255, 255, 5).into(),
            hover_border_color: Srgba::rgba_u8(173, 215, 255, 95).into(),
            hover_background_color: Srgba::rgba_u8(24, 95, 165, 30).into(),
            font_color: Srgba::rgba_u8(255, 255, 255, 95).into(),
            font_hover_color: Srgba::rgba_u8(173, 215, 255, 100).into(),
        }
    }
}
