use eframe::egui::{self, Color32, CornerRadius, FontId, Image, RichText, Sense, Vec2};

use crate::app::GameLauncher;
use crate::sections::Page;
use crate::theme::{GREEN, MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn nav_item(
        &mut self,
        ui: &mut egui::Ui,
        fallback_icon: &str,
        icon_path: &str,
        title: &str,
        page: Page,
    ) {
        let response = ui.allocate_response(Vec2::new(ui.available_width(), 48.0), Sense::click());

        if response.clicked() {
            self.page = page;
            if page == Page::Library {
                self.refresh_library();
            }
        }

        let selected = self.page == page;
        let color = if selected { TEXT } else { MUTED };
        let rect = response.rect;

        if selected {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(8),
                Color32::from_rgba_unmultiplied(105, 177, 62, 28),
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
                CornerRadius::same(2),
                GREEN,
            );
        } else if response.hovered() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(8),
                Color32::from_rgba_unmultiplied(255, 255, 255, 8),
            );
        }

        ui.scope_builder(
            egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(0.0, 4.0))),
            |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);
                    if !self.svg_icon(ui, icon_path, Vec2::splat(20.0)) {
                        ui.label(
                            RichText::new(fallback_icon)
                                .font(FontId::proportional(21.0))
                                .color(color),
                        );
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(title)
                            .font(FontId::proportional(16.0))
                            .color(color),
                    );
                });
            },
        );
    }

    pub(crate) fn svg_icon(&self, ui: &mut egui::Ui, path: &str, size: Vec2) -> bool {
        let Some(texture) = self.icons.get(path) else {
            return false;
        };

        ui.add(Image::new(texture).fit_to_exact_size(size));
        true
    }
}
