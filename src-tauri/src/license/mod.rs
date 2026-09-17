mod config;
pub mod crypto;
pub mod state;
pub mod storage;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub public_key: [u8; 32],
    pub kid: String,
    pub product_id: String,
    pub issuer: String,
    pub card_prefix: String,
    pub client_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LicenseError {
    pub code: String,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}
impl LicenseError {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.into(),
            message: "License verification failed".into(),
            retry_after_seconds: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    pub authorized: bool,
    pub mode: Option<String>,
    pub code: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<i64>,
    pub token_expires_at: Option<i64>,
    pub last_verified_at: Option<i64>,
    pub device_id: String,
    pub masked_card_key: Option<String>,
    pub server_url: String,
    pub card_prefix: String,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedLicense {
    pub card_key: String,
    pub token: Option<String>,
    pub server_time: i64,
    pub wall_at_verify: i64,
    pub observed_wall: i64,
    pub denial_code: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<i64>,
    pub token_expires_at: Option<i64>,
}

#[cfg(test)]
fn test_dir() -> std::path::PathBuf {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut random = [0; 16];
    SystemRandom::new().fill(&mut random).unwrap();
    let name = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root = std::env::temp_dir().join(format!("novel-words-license-test-{name}"));
    std::fs::create_dir(&root).unwrap();
    root
}
