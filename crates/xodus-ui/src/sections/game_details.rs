use eframe::egui::{self, Color32, FontId, RichText};

use crate::app::GameLauncher;
use crate::theme::{GREEN, MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn game_details_page(&mut self, ui: &mut egui::Ui) {
        let Some(selected_path) = self.selected_game.as_ref() else {
            ui.label(RichText::new("Select a game from your library.").color(MUTED));
            return;
        };
        let Some(game) = self.games.iter().find(|game| &game.path == selected_path) else {
            ui.label(
                RichText::new("This game is no longer in the configured games folder.")
                    .color(MUTED),
            );
            return;
        };
        let title = game.title.clone();
        let path = game.path.clone();
        let executable = game.executable.clone();
        let provider = game.provider.clone();

        ui.label(
            RichText::new(&title)
                .font(FontId::proportional(42.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(12.0);
        ui.label(RichText::new(path.display().to_string()).color(MUTED));
        ui.add_space(28.0);

        if let Some(ipc_name) = provider.ipc_name() {
            ui.label(
                RichText::new(format!(
                    "Interprocess communication for {ipc_name} isn't implemented yet."
                ))
                .size(18.0)
                .color(MUTED),
            );
        } else {
            let package_marker = path.join(".xodus-streaming.msixvc");
            if package_marker.is_file() {
                if ui
                    .add_sized(
                        [220.0, 48.0],
                        egui::Button::new(
                            RichText::new("Run")
                                .size(17.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(GREEN),
                    )
                    .clicked()
                {
                    self.start_game(path, executable);
                }
            } else {
                ui.label(RichText::new("This game is not installed yet.").color(MUTED));
            }
        }

        if let Some(result) = &self.settings_feedback {
            ui.add_space(16.0);
            let (message, color) = match result {
                Ok(message) => (message.as_str(), Color32::from_rgb(130, 205, 130)),
                Err(message) => (message.as_str(), Color32::from_rgb(255, 130, 130)),
            };
            ui.label(RichText::new(message).color(color));
        }
    }
}
