use super::{ClientConfig, LicenseError};
use sha2::{Digest, Sha256};
#[path = "../../license_public_key.rs"]
mod license_public_key;

mod compiled {
    include!(concat!(env!("OUT_DIR"), "/license_config.rs"));
}

impl ClientConfig {
    pub fn compiled() -> Result<Self, LicenseError> {
        let public_key = license_public_key::parse_public_key(compiled::PUBLIC_KEY_PEM)
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?;
        let config = Self {
            server_url: compiled::SERVER_URL.trim_end_matches('/').into(),
            public_key,
            kid: format!("{:x}", Sha256::digest(public_key))[..16].into(),
            product_id: compiled::PRODUCT_ID.into(),
            issuer: compiled::ISSUER.into(),
            card_prefix: compiled::CARD_PREFIX.into(),
            client_version: env!("CARGO_PKG_VERSION").into(),
        };
        let url = reqwest::Url::parse(&config.server_url)
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?;
        let local = url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
        if !(url.scheme() == "https" || cfg!(debug_assertions) && local)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || config.product_id.is_empty()
            || config.issuer.is_empty()
            || config.card_prefix.is_empty()
            || !config
                .card_prefix
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Err(LicenseError::new("LICENSING_UNAVAILABLE"));
        }
        Ok(config)
    }
}
