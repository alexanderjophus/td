use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::GameState;

use super::dice_physics::{DicePhysicsPlugin, ThrowPower};
use super::{DieRolledEvent, GamePlayState, GameResources, Rarity};

pub struct RollPlugin;

impl Plugin for RollPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DicePhysicsPlugin).add_systems(
            Update,
            rolling_ui.run_if(in_state(GameState::Game).and(in_state(GamePlayState::Rolling))),
        );
    }
}

fn rolling_ui(
    mut contexts: EguiContexts,
    mut game_resources: ResMut<GameResources>,
    mut ev_rolled: EventWriter<DieRolledEvent>,
    mut throw_power: ResMut<ThrowPower>,
    mut next_state: ResMut<NextState<GamePlayState>>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 10.0);

                ui.add(egui::Label::new(
                    egui::RichText::new("Roll Dice").size(32.0),
                ));

                ui.add_space(10.0);

                ui.add(egui::Label::new(
                    egui::RichText::new("Select a die to roll:").size(24.0),
                ));

                ui.add_space(10.0);

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    // Display die info in a frame
                    egui::Frame::dark_canvas(ui.style())
                        .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 200))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Die #{}",
                                    game_resources.highlighted_die + 1
                                ))
                                .size(18.0),
                            );
                            // Navigation and selection row
                            ui.horizontal(|ui| {
                                // Left button
                                if ui.button(egui::RichText::new("◀").size(24.0)).clicked() {
                                    game_resources.highlighted_die = (game_resources
                                        .highlighted_die
                                        + game_resources.dice.len()
                                        - 1)
                                        % game_resources.dice.len();
                                }
                                // Right button
                                if ui.button(egui::RichText::new("▶").size(24.0)).clicked() {
                                    game_resources.highlighted_die =
                                        (game_resources.highlighted_die + 1)
                                            % game_resources.dice.len();
                                }
                            });

                            ui.separator();

                            let current_die = &game_resources.dice[game_resources.highlighted_die];

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

                            ui.horizontal(|ui| {
                                ui.label("Throw Power:");
                                ui.add(
                                    egui::Slider::new(&mut throw_power.0, 0.1..=1.0)
                                        .show_value(true),
                                );
                            });

                            if let Some(result) = &current_die.result {
                                let color = match result.rarity {
                                    Rarity::Common => egui::Color32::WHITE,
                                    Rarity::Uncommon => egui::Color32::GREEN,
                                    Rarity::Rare => egui::Color32::BLUE,
                                    Rarity::Epic => egui::Color32::DARK_BLUE,
                                    Rarity::Unique => egui::Color32::ORANGE,
                                };

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Result:").strong());
                                    ui.label(
                                        egui::RichText::new(format!("{}", result.primary_type))
                                            .size(24.0)
                                            .color(color),
                                    );
                                });
                            }

                            // todo: fix can roll logic not working
                            if ui
                                .add_enabled(
                                    current_die.result.is_none() && !current_die.rolling,
                                    egui::Button::new("Roll"),
                                )
                                .clicked()
                            {
                                ev_rolled.send(DieRolledEvent(current_die.clone()));
                            }
                        });
                });

                // Start Placement button
                if ui
                    .add_enabled(
                        game_resources.towers.len() > 0,
                        egui::Button::new(egui::RichText::new("Continue to Placement").size(24.0))
                            .min_size(egui::vec2(250.0, 40.0)),
                    )
                    .clicked()
                {
                    next_state.set(GamePlayState::Placement);
                }
            });
        });
}
