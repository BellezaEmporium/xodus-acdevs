use eframe::egui::TextureHandle;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

// Represents the current (connected) user in the application.
pub(crate) struct User {
    username: String,
    avatar: Option<TextureHandle>,
}

// Represents the current session in the application, including the user and authentication token.
pub(crate) struct Session {
    pub(crate) user: Option<User>,
    pub(crate) token: Option<String>,
}