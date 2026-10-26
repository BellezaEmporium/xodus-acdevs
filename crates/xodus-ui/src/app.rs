use eframe::egui;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::Receiver,
};
use eframe::egui::TextureHandle;

use crate::models::game::Game;
use crate::sections::Page;
use crate::state::auth::AuthState;
use crate::state::settings::Settings;
use crate::theme::*;

pub struct GameLauncher {
    pub(crate) user: Option<crate::models::session::User>,
    pub(crate) page: Page,
    pub(crate) auth: AuthState,
    pub(crate) login_background: Option<TextureHandle>,
    pub(crate) login_logo: Option<TextureHandle>,
    pub(crate) login_result: Option<Receiver<Result<(), String>>>,
    pub(crate) operation_result: Option<Receiver<Result<String, String>>>,
    pub(crate) icons: HashMap<&'static str, TextureHandle>,
    pub(crate) games: Vec<Game>,
    pub(crate) selected_game: Option<PathBuf>,
    pub(crate) games_directory: String,
    pub(crate) wine_path: String,
    pub(crate) wine_prefix: String,
    pub(crate) last_played: Option<PathBuf>,
    pub(crate) settings_feedback: Option<Result<String, String>>,
    pub(crate) pending_game: Option<PathBuf>,
}

impl GameLauncher {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let icons = [
            (ICON_HOME, ICON_HOME),
            (ICON_LIBRARY, ICON_LIBRARY),
            (ICON_GAME_PASS, ICON_GAME_PASS),
            (ICON_STORE, ICON_STORE),
            (ICON_SETTINGS, ICON_SETTINGS),
        ]
        .into_iter()
        .filter_map(|(key, path)| {
            load_svg_texture(&cc.egui_ctx, key, path).map(|texture| (key, texture))
        })
        .collect();

    
        let login_background = load_texture(
            &cc.egui_ctx,
            "login-background",
            "assets/SocialCard/SocialCard1280.jpg",
        );
        let login_logo = load_texture(&cc.egui_ctx, "login-logo", ICON_LOGO);
        let (settings, settings_feedback) = match load_settings() {
            Ok(settings) => (settings, None),
            Err(error) => (
                Settings::defaults(),
                Some(Err(format!("Could not load settings: {error}"))),
            ),
        };
        let (games, settings_feedback) =
            match discover_games(Path::new(&settings.games_directory)) {
                Ok(games) => (games, settings_feedback),
                Err(error) => (
                    Vec::new(),
                    Some(Err(format!("Could not read games folder: {error}"))),
                ),
            };

        Self {
            games,
            user: None,
            page: Page::Home,
            auth: AuthState::LoggedOut,
            login_background: login_background,
            login_logo,
            login_result: None,
            operation_result: None,
            icons,
            games_directory: settings.games_directory,
            wine_path: settings.wine_path,
            wine_prefix: settings.wine_prefix,
            last_played: settings.last_played,
            selected_game: None,
            settings_feedback,
            pending_game: None,
        }
    }

    pub fn sidebar(&mut self, ui: &mut egui::Ui) {
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

    pub fn profile(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(avatar) = self.user.as_ref().and_then(|user| user.avatar.as_ref()) {
                ui.add(
                    Image::new(avatar)
                        .fit_to_exact_size(Vec2::splat(48.0))
                        .corner_radius(24.0),
                );
            } else {
                ui.allocate_ui(Vec2::splat(48.0), |ui| {
                    ui.painter().circle_filled(
                        ui.max_rect().center(),
                        22.0,
                        Color32::from_rgb(55, 57, 60),
                    );
                    ui.painter().text(
                        ui.max_rect().center(),
                        egui::Align2::CENTER_CENTER,
                        "X",
                        FontId::proportional(22.0),
                        TEXT,
                    );
                });
            }

            ui.add_space(8.0);
            ui.vertical(|ui| {
                let username = self
                    .user
                    .as_ref()
                    .map(|user| user.username.as_str())
                    .unwrap_or("Xbox account");

                ui.label(
                    RichText::new(username)
                        .font(FontId::proportional(20.0))
                        .strong()
                        .color(TEXT),
                );
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .corner_radius(CornerRadius::same(2))
                    .inner_margin(Margin::symmetric(4, 1))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("XODUS")
                                .font(FontId::proportional(10.0))
                                .strong()
                                .color(Color32::BLACK),
                        );
                    });
            });
        });
    }

    pub fn nav_item(
        &mut self,
        ui: &mut egui::Ui,
        fallback_icon: &str,
        icon_path: &str,
        title: &str,
        page: Page,
    ) {
        let response = ui.allocate_response(
            Vec2::new(ui.available_width(), 48.0),
            Sense::click(),
        );

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

    pub fn main_content(&mut self, ui: &mut egui::Ui) {
        ui.painter().rect_filled(ui.max_rect(), CornerRadius::ZERO, BG);
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
                        Page::Store => self.placeholder_page(
                            ui,
                            "Store",
                            "Store browsing is not connected yet.",
                        ),
                        Page::Settings => self.settings_page(ui),
                    });
                });
            });
    }

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

    pub fn library_page(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Library")
                .font(FontId::proportional(36.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "Installed games in {}",
                self.games_directory
            ))
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

    pub fn game_details_page(&mut self, ui: &mut egui::Ui) {
        let Some(selected_path) = self.selected_game.as_ref() else {
            ui.label(RichText::new("Select a game from your library.").color(MUTED));
            return;
        };
        let Some(game) = self.games.iter().find(|game| &game.path == selected_path) else {
            ui.label(RichText::new("This game is no longer in the configured games folder.").color(MUTED));
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

    pub fn game_pass_page(&self, ui: &mut egui::Ui) {
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

    pub fn settings_page(&mut self, ui: &mut egui::Ui) {
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
            if ui.button("Save settings & refresh library").clicked() {
                if self.save_settings() {
                    self.refresh_library();
                }
            }
            if ui.button("Check Wine").clicked() {
                let wine = self.wine_path.clone();
                self.start_operation(move || verify_wine(&wine));
            }
            if ui.button("Create / initialize prefix").clicked() {
                if self.save_settings() {
                    let wine = self.wine_path.clone();
                    let prefix = self.wine_prefix.clone();
                    self.start_operation(move || initialize_wine_prefix(&wine, &prefix));
                }
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

    pub fn save_settings(&mut self) -> bool {
        if self.games_directory.trim().is_empty() {
            self.settings_feedback = Some(Err("Enter an installed games folder path.".to_owned()));
            return false;
        }
        match save_settings(&Settings {
            games_directory: self.games_directory.clone(),
            wine_path: self.wine_path.clone(),
            wine_prefix: self.wine_prefix.clone(),
            last_played: self.last_played.clone(),
        }) {
            Ok(()) => {
                self.settings_feedback = Some(Ok("Settings saved.".to_owned()));
                true
            }
            Err(error) => {
                self.settings_feedback = Some(Err(format!("Could not save settings: {error}")));
                false
            }
        }
    }

    pub fn refresh_library(&mut self) {
        match discover_games(Path::new(&self.games_directory)) {
            Ok(games) => {
                self.games = games;
                self.settings_feedback = Some(Ok(format!(
                    "Found {} installed game(s).",
                    self.games.len()
                )));
            }
            Err(error) => {
                self.settings_feedback = Some(Err(format!("Could not read games folder: {error}")));
            }
        }
    }

    pub fn start_operation(
        &mut self,
        operation: impl FnOnce() -> Result<String, String> + Send + 'static,
    ) -> bool {
        if self.operation_result.is_some() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        self.operation_result = Some(receiver);
        thread::spawn(move || {
            let _ = sender.send(operation());
        });
        true
    }

    pub fn start_game(&mut self, path: PathBuf, executable: Option<String>) {
        let wine = self.wine_path.clone();
        let prefix = self.wine_prefix.clone();
        let path_for_command = path.clone();
        let started = self.start_operation(move || {
            launch_game(&path_for_command, executable.as_deref(), &wine, &prefix)
        });
        if started {
            self.pending_game = Some(path);
        }
    }

    pub fn poll_operation_result(&mut self) {
        let result = match self.operation_result.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("The background operation ended without a result.".to_owned()))
                }
            },
            None => None,
        };

        let Some(result) = result else {
            return;
        };
        self.operation_result = None;

        if result.is_ok()
            && let Some(path) = self.pending_game.take()
        {
            self.last_played = Some(path);
            if !self.save_settings() {
                return;
            }
        } else {
            self.pending_game = None;
        }
        self.settings_feedback = Some(result);
    }

    pub fn placeholder_page(&self, ui: &mut egui::Ui, title: &str, description: &str) {
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

    pub fn svg_icon(&self, ui: &mut egui::Ui, path: &str, size: Vec2) -> bool {
        let Some(texture) = self.icons.get(path) else {
            return false;
        };

        ui.add(Image::new(texture).fit_to_exact_size(size));
        true
    }

    pub fn login_page(&mut self, ui: &mut egui::Ui) {
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
                        RichText::new("Your games, in one place.")
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
                        AuthState::LoggingIn => {
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


    pub fn start_login(&mut self) {
        if matches!(&self.auth, AuthState::LoggingIn) {
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

    pub fn poll_login_result(&mut self, ctx: &egui::Context) {
        let result = match self.login_result.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(
                    "The Xodus CLI login task ended without returning a result.".to_owned(),
                )),
            },
            None => None,
        };

        let Some(result) = result else {
            return;
        };

        self.login_result = None;
        match result {
            Ok(()) => {
                let avatar = load_texture(ctx, "avatar", "assets/avatar.jpg");
                self.user = Some(User {
                    // TokenManager should supply the actual gamertag once wired into this crate.
                    username: "Xbox account".to_owned(),
                    avatar,
                });
                self.auth = AuthState::LoggedIn;
                self.page = Page::Home;
            }
            Err(message) => self.auth = AuthState::Error(message),
        }
    }


}
impl eframe::App for GameLauncher {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_login_result(&ctx);
        self.poll_operation_result();

        if matches!(&self.auth, AuthState::LoggingIn) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.operation_result.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        if !self.is_logged_in() {
            self.login_page(ui);
            return;
        }

        self.render_shell(ui);
    }
}