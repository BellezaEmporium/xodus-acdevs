use eframe::egui;

use crate::app::GameLauncher;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn store_page(&self, ui: &mut egui::Ui) {
        self.placeholder_page(ui, "Store", "Store browsing is not connected yet.");
    }
}
