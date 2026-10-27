use crate::models::session::Session;

// This section will be fully useful once the xodus
// crate's TokenManager is linked into this crate.
#[derive(Debug)]
pub(crate) enum AuthState {
    LoggedOut,
    /// A stored session is being restored on startup.
    #[allow(dead_code)]
    Refreshing,
    /// The user has submitted their credentials and is 
    /// waiting for the login process to complete.
    LoggingIn {
        message: String,
    },
    #[allow(dead_code)]
    LoggedIn(Session),
    /// The stored STS token expired and the user must sign in again.
    #[allow(dead_code)]
    Expired(String),
    Error(String),
}
