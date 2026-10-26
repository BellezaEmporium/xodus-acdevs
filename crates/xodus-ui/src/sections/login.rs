use eframe::egui::{self, Color32, CornerRadius, FontId, Image, RichText, Vec2};
use std::{sync::mpsc, thread};
use crate::app::GameLauncher;
use crate::services::cli::run_cli_login;
use crate::state::auth::AuthState;
use crate::theme::{BG, MUTED, TEXT};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn login_page(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();

        // Full-screen background
        if let Some(texture) = &self.login_background {
            let image_size = texture.size_vec2();
            let image_aspect = image_size.x / image_size.y;
            let view_aspect = rect.width() / rect.height();

            let uv = if image_aspect > view_aspect {
                let visible_width = view_aspect / image_aspect;
                let x = (1.0 - visible_width) * 0.5;
                egui::Rect::from_min_max(egui::pos2(x, 0.0), egui::pos2(1.0 - x, 1.0))
            } else {
                let visible_height = image_aspect / view_aspect;
                let y = (1.0 - visible_height) * 0.5;
                egui::Rect::from_min_max(egui::pos2(0.0, y), egui::pos2(1.0, 1.0 - y))
            };

            ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
        } else {
            ui.painter().rect_filled(rect, CornerRadius::ZERO, BG);
        }

        // Dark overlay
        ui.painter().rect_filled(rect, CornerRadius::ZERO, Color32::from_rgba_unmultiplied(0, 0, 0, 45));

        // Keep the login panel fixed to the left quarter of the viewport.
        let sidebar_width = rect.width() * 0.25;
        let sidebar_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + sidebar_width, rect.bottom()),
        );
        ui.painter().rect_filled(
            sidebar_rect,
            CornerRadius::ZERO,
            Color32::from_rgba_unmultiplied(20, 21, 23, 242),
        );

        ui.scope_builder(
            egui::UiBuilder::new().max_rect(
                sidebar_rect.shrink2(Vec2::new(28.0, 20.0)),
            ),
            |ui| {
                let available_height = ui.available_height();

                ui.add_space(available_height * 0.22);

                ui.vertical_centered(|ui| {
                    if let Some(logo) = &self.login_logo {
                        ui.add(Image::new(logo).fit_to_exact_size(Vec2::splat(84.0)));
                    }

                    ui.add_space(20.0);

                    ui.label(
                        RichText::new("The greatest gaming migration for Linux & macOS.")
                            .font(FontId::proportional(26.0))
                            .strong()
                            .color(TEXT),
                    );

                    ui.add_space(12.0);

                    ui.label(
                        RichText::new(
                            "Sign in with your Microsoft account\nto access your Xbox library.",
                        )
                            .size(16.0)
                            .color(MUTED),
                    );

                    ui.add_space(36.0);

                    let mut retry_login = false;

                    match &self.auth {
                        AuthState::LoggingIn { .. } => {
                            ui.spinner();
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("Waiting for Microsoft sign-in…")
                                    .size(16.0)
                                    .color(TEXT),
                            );
                        }

                        AuthState::LoggedOut | AuthState::LoggedIn => {
                            let button_width = ui.available_width().min(280.0);
                            let button_height = 48.0;
                            let button = egui::Button::new(
                                RichText::new("Login with Xodus-CLI")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(16, 104, 206))
                            .corner_radius(CornerRadius::same(7));

                            retry_login = ui
                                .add_sized([button_width, button_height], button)
                                .clicked();
                        }

                        AuthState::Error(message) => {
                            ui.label(
                                RichText::new(message)
                                    .size(15.0)
                                    .color(Color32::from_rgb(255, 130, 130)),
                            );
                            ui.add_space(12.0);
                            retry_login = ui.button("Try again").clicked();
                        }
                    }

                    if retry_login {
                        self.start_login();
                    }

                    ui.add_space(available_height * 0.08);

                    ui.label(
                        RichText::new("Authentication is handled by Xodus CLI.")
                            .size(12.0)
                            .color(Color32::from_rgb(145, 147, 151)),
                    );

                    ui.add_space(available_height * 0.05);
                });
            },
        );
    }
}

pub(crate) fn start_login(&mut self) {
    if matches!(&self.auth, AuthState::LoggingIn { .. }) {
        return;
    }

    let (sender, receiver) = mpsc::channel();
    self.login_result = Some(receiver);
    self.auth = AuthState::LoggingIn;

    // Do not block egui's UI thread while the CLI opens the Microsoft sign-in webview.
    thread::spawn(move || {
        let result = run_cli_login();
        let _ = sender.send(result);
    });
}