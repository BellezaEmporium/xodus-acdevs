use crate::app::GameLauncher;
use crate::sections::Page;
use crate::theme::{ICON_GAME_PASS, ICON_HOME, ICON_LIBRARY, ICON_SETTINGS, ICON_STORE, SIDEBAR};
use eframe::egui::{self, Align, CornerRadius, FontId, Layout, RichText, Vec2};
use egui::Color32;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn sidebar(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, CornerRadius::ZERO, SIDEBAR);
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(26.0, 25.0))),
            |ui| {
                ui.set_min_size(ui.available_size());
                ui.vertical(|ui| {
                    self.profile(ui);
                    ui.add_space(42.0);

                    self.nav_item(ui, "◢", ICON_HOME, "Home", Page::Home);
                    self.nav_item(ui, "✣", ICON_LIBRARY, "Library", Page::Library);
                    self.nav_item(ui, "◉", ICON_GAME_PASS, "Game Pass", Page::GamePass);
                    self.nav_item(ui, "▣", ICON_STORE, "Store", Page::Store);

                    if let Some(last_played) = self.last_played.clone()
                        && let Some(game) = self.games.iter().find(|game| game.path == last_played)
                    {
                        let recent_title = game.title.clone();
                        ui.add_space(28.0);
                        ui.label(
                            RichText::new("RECENTLY PLAYED")
                                .font(FontId::proportional(11.0))
                                .strong()
                                .color(Color32::from_rgb(100, 101, 105)),
                        );
                        ui.add_space(12.0);
                        if ui.button(recent_title).clicked() {
                            self.selected_game = Some(last_played);
                            self.page = Page::GameDetails;
                        }
                    }

                    // The bottom-up layout keeps Settings at the bottom of the sidebar.
                    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                        self.nav_item(ui, "⚙", ICON_SETTINGS, "Settings", Page::Settings);
                    });
                });
            },
        );
    }
}
