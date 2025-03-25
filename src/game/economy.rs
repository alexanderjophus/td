use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::{despawn_screen, GameState};

use super::{
    BaseElementType, Die, DieBuilder, DiePurchaseEvent, GamePlayState, GameResources, Rarity,
};

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        let shop_items = vec![
            DieBuilder::from_d6_type(BaseElementType::Fire).build(),
            DieBuilder::from_d6_type(BaseElementType::Water).build(),
            DieBuilder::from_d6_type(BaseElementType::Earth).build(),
            DieBuilder::from_d6_type(BaseElementType::Wind).build(),
        ];
        app.insert_resource(DieShop {
            highlighted: 0,
            items: shop_items,
        })
        .add_systems(
            Update,
            (
                economy_ui,
                update_shop_ui,
                update_economy_ui,
                spin_die,
                update_spinning_die,
                update_die_info,
            )
                .run_if(in_state(GamePlayState::Economy).and(in_state(GameState::Game))),
        )
        .add_systems(
            OnExit(GamePlayState::Economy),
            despawn_screen::<DieShopOverlay>,
        );
    }
}

#[derive(Resource, Debug, Clone, PartialEq)]
struct DieShop {
    items: Vec<Die>,
    highlighted: usize,
}

#[derive(Component)]
struct DieShopOverlay;

#[derive(Component)]
struct DieShopItem {
    item: Die,
}

#[derive(Component)]
struct MoneyText;

#[derive(Component)]
struct SpinningDie {
    rotation_speed: f32,
    die_data: Die,
}

#[derive(Component)]
struct DieInfoDisplay {
    die_index: usize,
}

fn economy_ui(
    mut contexts: EguiContexts,
    mut shop: ResMut<DieShop>,
    mut economy: ResMut<GameResources>,
    mut ev_die_purchase: EventWriter<DiePurchaseEvent>,
    mut next_state: ResMut<NextState<GamePlayState>>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 10.0);

                ui.add(egui::Label::new(egui::RichText::new("Die Shop").size(32.0)));

                ui.add_space(10.0);

                ui.add(egui::Label::new(
                    egui::RichText::new(format!("Money: {}", economy.money))
                        .background_color(egui::Color32::from_rgb(0, 0, 0))
                        .size(24.0),
                ));

                ui.add_space(10.0);

                ui.add(egui::Label::new(
                    egui::RichText::new("Select a die to purchase:").size(24.0),
                ));

                ui.add_space(10.0);

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    // Display die info in a frame
                    egui::Frame::dark_canvas(ui.style())
                        .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 200))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("Die #{}", shop.highlighted + 1))
                                    .size(18.0),
                            );
                            // Navigation and selection row
                            ui.horizontal(|ui| {
                                // Left button
                                if ui.button(egui::RichText::new("◀").size(24.0)).clicked() {
                                    shop.highlighted = (shop.highlighted + shop.items.len() - 1)
                                        % shop.items.len();
                                }
                                // Right button
                                if ui.button(egui::RichText::new("▶").size(24.0)).clicked() {
                                    shop.highlighted = (shop.highlighted + 1) % shop.items.len();
                                }
                            });
                            let current_die = &shop.items[shop.highlighted];

                            ui.label(format!("Cost: {}", current_die.value));

                            ui.separator();

                            // Show die faces
                            ui.label("Faces:");
                            for (i, face) in current_die.faces.iter().enumerate() {
                                let color = match face.rarity {
                                    Rarity::Common => egui::Color32::WHITE,
                                    Rarity::Uncommon => egui::Color32::GREEN,
                                    Rarity::Rare => egui::Color32::BLUE,
                                    Rarity::Epic => egui::Color32::DARK_BLUE,
                                    Rarity::Unique => egui::Color32::ORANGE,
                                };

                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}. {}",
                                        i + 1,
                                        face.primary_type
                                    ))
                                    .color(color),
                                );
                            }

                            ui.separator();

                            // Purchase button
                            let already_purchased = economy.dice.contains(current_die);
                            let can_purchase =
                                economy.money >= current_die.value && !already_purchased;

                            if ui
                                .add_enabled(can_purchase, egui::Button::new("Purchase"))
                                .clicked()
                            {
                                economy.money -= current_die.value;
                                ev_die_purchase.send(DiePurchaseEvent(current_die.clone()));
                            }

                            if already_purchased {
                                ui.label(
                                    egui::RichText::new("Already Purchased")
                                        .color(egui::Color32::YELLOW),
                                );
                            } else if !can_purchase {
                                ui.label(
                                    egui::RichText::new("Not enough money")
                                        .color(egui::Color32::RED),
                                );
                            }
                        });
                });

                if ui
                    .button(egui::RichText::new("Start Game").size(24.0))
                    .clicked()
                {
                    next_state.set(GamePlayState::Rolling);
                }
            });
        });
}

fn update_shop_ui(
    shop: ResMut<DieShop>,
    game_resources: Res<GameResources>,
    mut query: Query<(&mut BackgroundColor, &DieShopItem, Entity), Without<Text>>,
    text_query: Query<&mut Text>,
    mut commands: Commands,
) {
    for (mut bg_color, item, entity) in query.iter_mut() {
        // First update background colors for selection highlighting
        if item.item == shop.items[shop.highlighted] {
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));
        } else {
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8));
        }

        // Check if this item is purchased and needs the "Purchased" label
        let is_purchased = game_resources.dice.contains(&item.item);

        // Check if this item already has the "Purchased" label
        let has_purchased_label = text_query
            .get(entity)
            .map(|text| text.0 == "Purchased")
            .unwrap_or(false);

        // Add "Purchased" label if needed and not already present
        if is_purchased && !has_purchased_label {
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Percent(80.),
                        height: Val::Px(30.),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.5, 0.0, 0.0, 0.7)),
                    Text::new("Purchased"),
                ));
            });
        }
    }
}

fn update_economy_ui(
    game_resources: Res<GameResources>,
    mut query: Query<&mut Text, With<MoneyText>>,
) {
    for mut text in query.iter_mut() {
        text.0 = game_resources.money.to_string();
    }
}

// System to rotate the die
fn spin_die(time: Res<Time>, mut query: Query<(&mut Transform, &SpinningDie)>) {
    for (mut transform, spinning_die) in query.iter_mut() {
        transform.rotate_axis(Dir3::Y, spinning_die.rotation_speed * time.delta_secs());
        transform.rotate_axis(
            Dir3::X,
            spinning_die.rotation_speed * 0.5 * time.delta_secs(),
        );
    }
}

// System to update the spinning die when selection changes
fn update_spinning_die(shop: Res<DieShop>, mut query: Query<(&mut SpinningDie, &mut Transform)>) {
    if shop.is_changed() {
        for (mut spinning_die, mut transform) in query.iter_mut() {
            spinning_die.die_data = shop.items[shop.highlighted].clone();
            // Reset rotation when changing dies for a cleaner transition
            transform.rotation = Quat::IDENTITY;
        }
    }
}

// System to update die info display
fn update_die_info(
    shop: Res<DieShop>,
    mut query: Query<(&mut DieInfoDisplay, Entity)>,
    mut commands: Commands,
) {
    if shop.is_changed() {
        for (mut info_display, entity) in query.iter_mut() {
            info_display.die_index = shop.highlighted;

            // Clear existing children
            commands.entity(entity).despawn_descendants();

            // Add new info based on highlighted die
            let die = &shop.items[shop.highlighted];

            commands.entity(entity).with_children(|parent| {
                parent.spawn(Text::new(format!("Price: {}", die.value)));

                // Add die face info
                for (i, face) in die.faces.iter().enumerate() {
                    parent.spawn(Text::new(format!(
                        "Face {}: {} ({})",
                        i + 1,
                        face.primary_type,
                        face.rarity
                    )));
                }
            });
        }
    }
}
