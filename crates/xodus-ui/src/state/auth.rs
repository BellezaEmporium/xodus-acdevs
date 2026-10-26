use crate::models::session::Session;

#[derive(Debug)]
pub(crate) enum AuthState {
    LoggedOut,
    Refreshing,
    LoggingIn { message: String },
    LoggedIn(Session),
    Expired(String),
    Error(String),
}