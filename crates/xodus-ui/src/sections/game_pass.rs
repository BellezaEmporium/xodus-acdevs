use eframe::egui::{self, FontId, RichText};

use crate::app::GameLauncher;
use crate::theme::{MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn game_pass_page(&self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Game Pass")
                .font(FontId::proportional(36.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(16.0);
        ui.label(
            RichText::new("Xbox Game Pass ownership and catalogue access are not connected yet, so the catalogue is hidden.")
                .size(16.0)
                .color(MUTED),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("Once subscription checks are available, users without an eligible Game Pass version will see the catalogue restriction here.")
                .size(14.0)
                .color(MUTED),
        );
    }
}
