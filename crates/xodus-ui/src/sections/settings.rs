use eframe::egui::{self, Color32, FontId, RichText};

use crate::app::GameLauncher;
use crate::services::wine::{initialize_wine_prefix, verify_wine};
use crate::theme::{MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Settings")
                .font(FontId::proportional(36.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(24.0);

        ui.label(RichText::new("Installed games folder").strong().color(TEXT));
        ui.add(
            egui::TextEdit::singleline(&mut self.games_directory)
                .desired_width(520.0)
                .hint_text("Path to your Xodus games"),
        );
        ui.add_space(18.0);

        ui.label(RichText::new("Wine executable").strong().color(TEXT));
        ui.add(
            egui::TextEdit::singleline(&mut self.wine_path)
                .desired_width(520.0)
                .hint_text("wine"),
        );
        ui.add_space(18.0);

        ui.label(RichText::new("Wine prefix").strong().color(TEXT));
        ui.add(
            egui::TextEdit::singleline(&mut self.wine_prefix)
                .desired_width(520.0)
                .hint_text("Path to create or use as a Wine prefix"),
        );
        ui.label(
            RichText::new("Other-provider folders can be registered with a .xodus-game file containing title= and provider= (gog, ea, steam, ubisoft, or riot).")
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Save settings & refresh library").clicked() && self.save_settings() {
                self.refresh_library();
            }
            if ui.button("Check Wine").clicked() {
                let wine = self.wine_path.clone();
                self.start_operation(move || verify_wine(&wine));
            }
            if ui.button("Create / initialize prefix").clicked() && self.save_settings() {
                let wine = self.wine_path.clone();
                let prefix = self.wine_prefix.clone();
                self.start_operation(move || initialize_wine_prefix(&wine, &prefix));
            }
        });

        ui.add_space(16.0);
        if self.operation_result.is_some() {
            ui.label(RichText::new("Working…").color(MUTED));
        }
        if let Some(result) = &self.settings_feedback {
            let (message, color) = match result {
                Ok(message) => (message.as_str(), Color32::from_rgb(130, 205, 130)),
                Err(message) => (message.as_str(), Color32::from_rgb(255, 130, 130)),
            };
            ui.label(RichText::new(message).color(color));
        }
    }
}
