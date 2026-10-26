use crate::hardware;
use crate::licensing::splicense::{SPLicense, SPLicenseParseError};
use crate::licensing::utils::{generate_string, parse_bcrypt_rsa_private};
use crate::models::devicecredential::{Authentication, ClientInfo, DeviceAddRequest, DeviceInfo};
use crate::models::secrets::{Device, TokenConversionError};
use crate::models::soap::BodyContent;
use crate::tokens::manager::TokenManager;
use crate::tokens::store::TokenStoreError;

#[derive(Debug, thiserror::Error)]
pub enum DeviceCredentialsError {
    #[error("network error while provisioning device: {0}")]
    Http(#[from] reqwest::Error),

    #[error("device authentication failed: {0}")]
    Auth(#[from] crate::api::live::rst::error::RSTError),

    #[error("failed to parse device license: {0}")]
    License(#[from] SPLicenseParseError),

    #[error("failed to parse device RSA key: {0}")]
    RsaKey(#[from] rsa::errors::Error),

    #[error("failed to store device credentials: {0}")]
    Store(#[from] TokenStoreError),

    #[error("device license is missing its CLEP sign state")]
    MissingClepSignState,

    #[error("device STS response is missing the token key name")]
    MissingKeyName,

    #[error("device STS response is not a security token response")]
    UnexpectedAuthResponse,

    #[error("device STS response could not be converted: {0}")]
    Token(#[from] TokenConversionError),
}

/// Provisions a device (if none is stored yet) or re-authenticates an existing
/// one (if its STS token is missing or expired), persisting the result through
/// `tokens`.
pub async fn ensure_device_credentials(
    client: &reqwest::Client,
    tokens: &TokenManager,
) -> Result<(), DeviceCredentialsError> {
    match tokens.get_device_license() {
        Err(_) => provision_device(client, tokens).await,
        Ok(license) if tokens.get_device_sts_token_fresh().is_err() => {
            reauthenticate_device(client, tokens, license).await
        }
        Ok(_) => Ok(()),
    }
}

async fn provision_device(
    client: &reqwest::Client,
    tokens: &TokenManager,
) -> Result<(), DeviceCredentialsError> {
    let username = format!("02{}", generate_string(14));
    let password = generate_string(20);
    let provision = DeviceAddRequest {
        client_info: ClientInfo::default(),
        authentication: Authentication::new(username.clone(), password.clone()),
        device_info: Some(DeviceInfo {
            id: "DeviceInfo".to_string(),
            components: hardware::probe_provision_components(),
            tpm_info: None,
        }),
    };

    let dev = crate::api::live::login_device_credential(client, provision).await?;

    let device = Device {
        username: username.clone(),
        password: password.clone(),
        puid: dev.puid,
        hwid: dev.hw_device_id,
        device_id: dev.license.binding.device_id.unwrap_or_default(),
        splicense: dev.license.splicense_block,
    };

    tokens.save_device_license(&device)?;

    reauthenticate_device(client, tokens, device).await
}

async fn reauthenticate_device(
    client: &reqwest::Client,
    tokens: &TokenManager,
    license: Device,
) -> Result<(), DeviceCredentialsError> {
    let sp_license = SPLicense::parse_base64(&license.splicense)?;
    let clep_sign_state = sp_license
        .clep_sign_state
        .ok_or(DeviceCredentialsError::MissingClepSignState)?;
    let key = clep_sign_state.get_rsa_key()?;
    let private_key = parse_bcrypt_rsa_private(&key)?;
    let resp = crate::api::live::authenticate_device(client, license.username, private_key).await?;

    if let BodyContent::RequestSecurityTokenResponse(resp) = resp.body.body {
        save_device_sts_token(tokens, resp)?;
        Ok(())
    } else {
        Err(DeviceCredentialsError::UnexpectedAuthResponse)
    }
}

fn save_device_sts_token(
    tokens: &TokenManager,
    resp: Box<crate::models::soap::RequestSecurityTokenResponse>,
) -> Result<(), DeviceCredentialsError> {
    let key_name = resp
        .requested_security_token
        .encrypted_data
        .as_ref()
        .and_then(|data| data.key_info.key_name.as_deref())
        .ok_or(DeviceCredentialsError::MissingKeyName)?
        .to_owned();
    let token = (*resp).try_into()?;
    tokens.save_device_token(key_name, token)?;
    Ok(())
}
