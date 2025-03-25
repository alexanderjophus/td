use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::GameState;

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
            economy_ui.run_if(in_state(GamePlayState::Economy).and(in_state(GameState::Game))),
        );
    }
}

#[derive(Resource, Debug, Clone, PartialEq)]
struct DieShop {
    items: Vec<Die>,
    highlighted: usize,
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
                    .add_enabled(
                        economy.dice.len() > 0,
                        egui::Button::new(egui::RichText::new("Start Game").size(24.0)),
                    )
                    .clicked()
                {
                    next_state.set(GamePlayState::Rolling);
                }
            });
        });
}
