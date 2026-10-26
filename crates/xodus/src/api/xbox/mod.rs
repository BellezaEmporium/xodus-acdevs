use crate::models::live::ExchangeUserTokenOutcome;
use crate::models::secrets::{LegacyToken, Token};
use crate::models::soap;
use crate::models::xbox::XstsResponse;

pub mod auth;
pub mod error;
pub mod title;

pub use auth::{authenticate_xbox_user, get_xsts_auth_header, request_xsts_token};
pub use error::*;

pub async fn run(
    client: &reqwest::Client,
    dev_token: LegacyToken,
    legacy: LegacyToken,
    relying_party: &str,
) -> Result<XstsResponse, AuthError> {
    let exchange = crate::api::live::exchange_user_token(
        client,
        legacy,
        "USERNAME".to_string(),
        dev_token,
        None,
        Some("Silent".to_string()),
        "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
        &[(
            "user.auth.xboxlive.com".to_owned(),
            Some(soap::PolicyReference::mbi_ssl()),
        )],
    )
    .await?;

    let token: Token = match exchange {
        ExchangeUserTokenOutcome::Issued(
            soap::BodyContent::RequestSecurityTokenResponseCollection(mut c),
        ) => c
            .security_tokens
            .remove(0)
            .try_into()
            .map_err(AuthError::Token)?,
        ExchangeUserTokenOutcome::Issued(soap::BodyContent::RequestSecurityTokenResponse(t)) => {
            (*t).try_into().map_err(AuthError::Token)?
        }
        ExchangeUserTokenOutcome::Issued(_) => {
            return Err(AuthError::UnexpectedBodyContent(
                "Recieved body content doesn't match any of the expected formats.".to_string(),
            ));
        }
        ExchangeUserTokenOutcome::Fault(f) => {
            return Err(AuthError::SoapFault(format!("{f:?}")));
        }
    };

    let Token::Compact(rps_ticket) = token else {
        return Err(AuthError::UnsupportedTokenFormat);
    };

    let user = authenticate_xbox_user(client, rps_ticket).await?;
    let xsts = request_xsts_token(client, user.token, relying_party).await?;
    Ok(xsts)
}
