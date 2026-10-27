use eframe::egui::{self, TextureHandle};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use crate::models::game::Game;
use crate::models::session::User;
use crate::sections::Page;
use crate::services::assets::{load_svg_texture, load_texture};
use crate::services::games::{discover_games, launch_game};
use crate::state::auth::AuthState;
use crate::state::settings::{Settings, load_settings, save_settings};
use crate::theme::*;

pub struct GameLauncher {
    pub(crate) user: Option<User>,
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
        let (games, settings_feedback) = match discover_games(Path::new(&settings.games_directory))
        {
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
            login_background,
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

    pub fn is_logged_in(&self) -> bool {
        matches!(self.auth, AuthState::LoggedIn(_))
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
                self.settings_feedback =
                    Some(Ok(format!("Found {} installed game(s).", self.games.len())));
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
                Err(TryRecvError::Disconnected) => Some(Err(
                    "The background operation ended without a result.".to_owned(),
                )),
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
}

impl eframe::App for GameLauncher {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_login_result(&ctx);
        self.poll_operation_result();

        if matches!(
            self.auth,
            AuthState::LoggingIn { .. } | AuthState::Refreshing
        ) {
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
