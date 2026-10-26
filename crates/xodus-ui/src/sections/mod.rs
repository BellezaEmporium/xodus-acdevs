use eframe::egui;
use egui::{Color32, FontId, RichText};
use crate::app::GameLauncher;
use crate::theme::*;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Library,
    GameDetails,
    GamePass,
    Store,
    Settings,
}

mod game_details;
mod game_pass;
mod home;
mod library;
mod login;
mod settings;
mod store;

impl GameLauncher {
    pub(crate) fn main_content(&mut self, ui: &mut egui::Ui) {
        ui.painter().rect_filled(
            ui.max_rect(),
            egui::CornerRadius::ZERO,
            crate::theme::BG,
        );
        egui::ScrollArea::vertical()
            .id_salt("main-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(36.0);
                ui.horizontal(|ui| {
                    ui.add_space(44.0);
                    ui.vertical(|ui| match self.page {
                        Page::Home => self.home_page(ui),
                        Page::Library => self.library_page(ui),
                        Page::GameDetails => self.game_details_page(ui),
                        Page::GamePass => self.game_pass_page(ui),
                        Page::Store => self.store_page(ui),
                        Page::Settings => self.settings_page(ui),
                    });
                });
            });
    }

    pub(crate) fn placeholder_page(&self, ui: &mut egui::Ui, title: &str, description: &str) {
        ui.add_space(48.0);
        ui.horizontal(|ui| {
            ui.add_space(44.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(title)
                        .font(FontId::proportional(36.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(description).size(16.0).color(MUTED));
                ui.add_space(24.0);
                ui.label(
                    RichText::new("This page is connected to the navigation; its data view can be added next.")
                        .size(13.0)
                        .color(Color32::from_rgb(100, 102, 106)),
                );
            });
        });
    }
}