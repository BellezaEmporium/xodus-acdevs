#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Xal(#[from] xal::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}
