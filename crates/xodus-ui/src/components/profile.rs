use eframe::egui::{self, Color32, CornerRadius, FontId, Image, Margin, RichText, Vec2};

use crate::app::GameLauncher;
use crate::theme::TEXT;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn profile(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(avatar) = self.user.as_ref().and_then(|user| user.avatar.as_ref()) {
                ui.add(
                    Image::new(avatar)
                        .fit_to_exact_size(Vec2::splat(48.0))
                        .corner_radius(24.0),
                );
            } else {
                ui.allocate_ui(Vec2::splat(48.0), |ui| {
                    ui.painter().circle_filled(
                        ui.max_rect().center(),
                        22.0,
                        Color32::from_rgb(55, 57, 60),
                    );
                    ui.painter().text(
                        ui.max_rect().center(),
                        egui::Align2::CENTER_CENTER,
                        "X",
                        FontId::proportional(22.0),
                        TEXT,
                    );
                });
            }

            ui.add_space(8.0);
            ui.vertical(|ui| {
                let username = self
                    .user
                    .as_ref()
                    .map(|user| user.username.as_str())
                    .unwrap_or("Xbox account");

                ui.label(
                    RichText::new(username)
                        .font(FontId::proportional(20.0))
                        .strong()
                        .color(TEXT),
                );
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .corner_radius(CornerRadius::same(2))
                    .inner_margin(Margin::symmetric(4, 1))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("XODUS")
                                .font(FontId::proportional(10.0))
                                .strong()
                                .color(Color32::BLACK),
                        );
                    });
            });
        });
    }
}
