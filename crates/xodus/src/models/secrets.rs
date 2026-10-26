use serde::{Deserialize, Serialize};

use crate::models::soap::{self, Timestamp};

#[derive(Debug, thiserror::Error)]
pub enum TokenConversionError {
    #[error("unsupported token type in SOAP response: {0}")]
    UnsupportedTokenType(String),

    #[error("SOAP response is missing the binary security token")]
    MissingBinarySecurityToken,

    #[error("SOAP response is missing the encrypted data section")]
    MissingEncryptedData,

    #[error("failed to serialize legacy token payload: {0}")]
    Serialize(#[from] quick_xml::SeError),
}

impl Timestamp {
    /// Expiry instant advertised by the STS in the `wsu:Expires` field.
    pub fn expires_at(&self) -> chrono::ParseResult<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(&self.expires).map(|t| t.with_timezone(&chrono::Utc))
    }
}

impl Token {
    /// Expiry time carried by the token payload, when it advertises one.
    ///
    /// Compact tokens embed their expiry in a binary format that is not
    /// parsed here, so they report `None` (treated as "unknown").
    pub fn expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match self {
            Token::Legacy(legacy) => legacy.lifetime.expires_at().ok(),
            Token::Compact(_) => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub puid: String,
    pub hwid: String,
    pub device_id: String,
    pub splicense: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LegacyToken {
    pub key_name: Option<String>,
    pub token: String,
    pub binary_secret: Option<String>,
    #[serde(default)]
    pub tpm_key: Option<String>,
    pub lifetime: Timestamp,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Token {
    Legacy(LegacyToken),
    Compact(String),
}

impl TryFrom<soap::RequestSecurityTokenResponse> for Token {
    type Error = TokenConversionError;

    fn try_from(value: soap::RequestSecurityTokenResponse) -> Result<Self, Self::Error> {
        match value.token_type.as_str() {
            "urn:passport:legacy" => {
                let encrypted_data = value
                    .requested_security_token
                    .encrypted_data
                    .ok_or(TokenConversionError::MissingEncryptedData)?;
                let key_name = encrypted_data.key_info.key_name.clone();
                let token = quick_xml::se::to_string(&encrypted_data)?;
                let binary_secret = value
                    .requested_proof_token
                    .as_ref()
                    .map(|t| t.binary_secret.clone());
                let tpm_key = value
                    .requested_proof_token
                    .and_then(|t| t.encrypted_key.map(|k| k.cipher_data.cipher_value));
                Ok(Self::Legacy(LegacyToken {
                    key_name,
                    token,
                    binary_secret,
                    tpm_key,
                    lifetime: value.lifetime,
                }))
            }
            "urn:passport:compact" => {
                let token = value
                    .requested_security_token
                    .binary_security_token
                    .ok_or(TokenConversionError::MissingBinarySecurityToken)?
                    .value;
                Ok(Self::Compact(token))
            }
            "urn:passport:delegationcompact" => {
                let token = value
                    .requested_security_token
                    .binary_security_token
                    .ok_or(TokenConversionError::MissingBinarySecurityToken)?
                    .value;
                Ok(Self::Compact(format!("d={token}")))
            }
            other => Err(TokenConversionError::UnsupportedTokenType(
                other.to_string(),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStore {
    #[serde(flatten)]
    pub tokens: std::collections::HashMap<String, Token>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub puid: String,
    pub username: String,
}
