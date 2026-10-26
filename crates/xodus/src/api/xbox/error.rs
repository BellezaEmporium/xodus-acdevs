use crate::api::live::rst::error::RSTError;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("SOAP fault: {0}")]
    SoapFault(String),

    #[error("unsupported token format")]
    UnsupportedTokenFormat,

    #[error("unexpected SOAP body content: {0}")]
    UnexpectedBodyContent(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Rst(#[from] RSTError),

    #[error("XSTS failed: code={code} redirect={redirect:?}")]
    Xsts { code: u32, redirect: Option<String> },

    #[error("missing claim: {0}")]
    MissingClaim(&'static str),
}