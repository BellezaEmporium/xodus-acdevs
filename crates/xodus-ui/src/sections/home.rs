use eframe::egui::{self, FontId, RichText};
use crate::app::GameLauncher;
use crate::theme::{MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub fn home_page(&self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Welcome to Xodus")
                .font(FontId::proportional(36.0))
                .strong()
                .color(TEXT),
        );

        if let Some(path) = &self.last_played {
            if let Some(game) = self.games.iter().find(|game| &game.path == path) {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("Last played: {}", game.title))
                        .size(16.0)
                        .color(MUTED),
                );
            }
        }
    }
}