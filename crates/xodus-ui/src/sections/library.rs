use eframe::egui::{self, Color32, FontId, RichText};

use crate::app::GameLauncher;
use crate::sections::Page;
use crate::theme::{MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn library_page(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Library")
                .font(FontId::proportional(36.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Installed games in {}", self.games_directory))
                .size(14.0)
                .color(MUTED),
        );
        ui.add_space(20.0);

        if self.games.is_empty() {
            ui.label(
                RichText::new("No installed games found.")
                    .size(18.0)
                    .color(TEXT),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Remote Xbox ownership is not connected yet. Xodus games are detected when their folder contains .xodus-streaming.msixvc.")
                    .size(14.0)
                    .color(MUTED),
            );
        }

        for game in self.games.clone() {
            let title = game.title.clone();
            let path = game.path.clone();
            let provider = game.provider.clone();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&title)
                            .font(FontId::proportional(20.0))
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {}",
                            provider.display_name(),
                            path.display()
                        ))
                        .size(12.0)
                        .color(MUTED),
                    );
                });
                if ui.button("Open").clicked() {
                    self.selected_game = Some(path.clone());
                    self.page = Page::GameDetails;
                }
            });
            ui.add_space(12.0);
        }
        if let Some(Err(message)) = &self.settings_feedback {
            ui.label(
                RichText::new(message)
                    .size(14.0)
                    .color(Color32::from_rgb(255, 130, 130)),
            );
        }
    }
}
