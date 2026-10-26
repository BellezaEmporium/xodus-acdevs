use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MSATokenRequest {
    pub client_id: String,
    #[serde(default, alias = "AllowUI")]
    pub allow_ui: bool,
    #[serde(default, alias = "MSAFullTrust")]
    pub msa_full_trust: bool,
}

#[derive(Serialize, Debug)]
pub struct MSATokenResponse {
    pub token: String,
    pub expiry: i64,
    #[serde(rename = "DeviceRPS")]
    pub device_rps: String,
    #[serde(rename = "DeviceExpiry")]
    pub device_expiry: i64,
}