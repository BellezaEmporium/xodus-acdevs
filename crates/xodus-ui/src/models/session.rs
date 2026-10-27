use eframe::egui::TextureHandle;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

// Represents the current (connected) user in the application.
//
// `Debug` is implemented manually because `TextureHandle` is not `Debug`.
#[derive(Clone)]
pub(crate) struct User {
    pub(crate) username: String,
    pub(crate) avatar: Option<TextureHandle>,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

// Represents the current session in the application, including the user and
// authentication token.
//
// The fields stay unread until this crate links directly into the xodus
// crate's TokenManager instead of driving the xodus-cli binary.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Session {
    pub(crate) user: Option<User>,
    pub(crate) token: Option<String>,
}
