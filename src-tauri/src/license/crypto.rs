use super::{ClientConfig, LicenseError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub device_hash: String,
    pub plan: String,
    pub grant_version: i64,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
    pub license_expires_at: Option<i64>,
}

pub fn validate_token(
    token: &str,
    config: &ClientConfig,
    device: &str,
    now: i64,
) -> Result<Claims, LicenseError> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use ring::signature::{UnparsedPublicKey, ED25519};
    use sha2::{Digest, Sha256};
    let invalid = || LicenseError::new("TOKEN_INVALID");
    if token.len() > 16384 {
        return Err(invalid());
    }
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(invalid());
    }
    let header: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
    if header["alg"] != "EdDSA" || header["typ"] != "JWT" || header["kid"] != config.kid {
        return Err(invalid());
    }
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).map_err(|_| invalid())?;
    UnparsedPublicKey::new(&ED25519, &config.public_key)
        .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
        .map_err(|_| invalid())?;
    let payload: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
    // Nullable is distinct from missing: serde Option alone accepts missing fields.
    if !payload
        .as_object()
        .is_some_and(|p| p.contains_key("license_expires_at"))
    {
        return Err(invalid());
    }
    let claims: Claims = serde_json::from_value(payload).map_err(|_| invalid())?;
    if claims.iss != config.issuer || claims.aud != config.product_id {
        return Err(LicenseError::new("WRONG_PRODUCT"));
    }
    if claims.device_hash != format!("{:x}", Sha256::digest(device.trim().as_bytes())) {
        return Err(LicenseError::new("DEVICE_MISMATCH"));
    }
    if uuid::Uuid::parse_str(&claims.sub).is_err()
        || claims.grant_version < 1
        || claims.iat < 0
        || claims.nbf < claims.iat
        || claims.nbf >= claims.exp
        || claims
            .exp
            .checked_sub(claims.iat)
            .is_none_or(|span| span <= 0 || span > 86400)
    {
        return Err(invalid());
    }
    match (claims.plan.as_str(), claims.license_expires_at) {
        ("lifetime", None) => {}
        ("7d" | "30d" | "365d", Some(deadline))
            if deadline > claims.iat && claims.exp <= deadline => {}
        _ => return Err(invalid()),
    }
    if claims.iat > now.saturating_add(5) || claims.nbf > now.saturating_add(5) {
        return Err(LicenseError::new("CLOCK_ROLLBACK"));
    }
    if claims
        .license_expires_at
        .is_some_and(|deadline| now >= deadline)
    {
        return Err(LicenseError::new("LICENSE_EXPIRED"));
    }
    if now >= claims.exp {
        return Err(LicenseError::new("VERIFICATION_REQUIRED"));
    }
    Ok(claims)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};

    pub fn fixture() -> (ClientConfig, Ed25519KeyPair, Value) {
        let key = Ed25519KeyPair::from_seed_unchecked(&[19; 32]).unwrap();
        let public_key: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
        let kid = format!("{:x}", Sha256::digest(public_key))[..16].to_string();
        let config = ClientConfig {
            server_url: "http://127.0.0.1:18100".into(),
            public_key,
            kid,
            product_id: "NovelWords".into(),
            issuer: "PrismKey_NovelWords".into(),
            card_prefix: "CY".into(),
            client_version: "0.1.0".into(),
        };
        let payload = json!({"iss":config.issuer,"aud":config.product_id,"sub":"e424c45d-7c0f-40e0-a178-c5468783a1b3","device_hash":format!("{:x}",Sha256::digest(b"test-device-0123456789")),"plan":"30d","grant_version":1,"iat":1700000000,"nbf":1700000000,"exp":1700086400,"license_expires_at":1702592000});
        (config, key, payload)
    }
    pub fn signed(config: &ClientConfig, key: &Ed25519KeyPair, payload: &Value) -> String {
        signed_header(
            key,
            &json!({"alg":"EdDSA","typ":"JWT","kid":config.kid}),
            payload,
        )
    }
    fn signed_header(key: &Ed25519KeyPair, header: &Value, payload: &Value) -> String {
        let data = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(header).unwrap()),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload).unwrap())
        );
        format!(
            "{}.{}",
            data,
            URL_SAFE_NO_PAD.encode(key.sign(data.as_bytes()).as_ref())
        )
    }
    #[test]
    fn accepts_only_signed_device_bound_credentials_and_plan_deadlines() {
        let (config, key, mut payload) = fixture();
        let claims = validate_token(
            &signed(&config, &key, &payload),
            &config,
            "test-device-0123456789",
            1700000001,
        )
        .unwrap();
        assert_eq!(claims.aud, "NovelWords");
        payload["plan"] = json!("lifetime");
        payload["license_expires_at"] = Value::Null;
        assert!(validate_token(
            &signed(&config, &key, &payload),
            &config,
            "test-device-0123456789",
            1700000001
        )
        .is_ok());
        assert!(validate_token(
            &signed(&config, &key, &payload),
            &config,
            "other-device-0123456789",
            1700000001
        )
        .is_err());
    }
    #[test]
    fn rejects_algorithm_key_product_issuer_and_signature_substitution() {
        let (config, key, payload) = fixture();
        for header in [
            json!({"alg":"none","typ":"JWT","kid":config.kid}),
            json!({"alg":"HS256","typ":"JWT","kid":config.kid}),
            json!({"alg":"EdDSA","typ":"JWT","kid":"unknown"}),
        ] {
            assert!(validate_token(
                &signed_header(&key, &header, &payload),
                &config,
                "test-device-0123456789",
                1700000001
            )
            .is_err());
        }
        let other = Ed25519KeyPair::from_seed_unchecked(&[20; 32]).unwrap();
        assert!(validate_token(
            &signed(&config, &other, &payload),
            &config,
            "test-device-0123456789",
            1700000001
        )
        .is_err());
        for field in ["aud", "iss"] {
            let mut p = payload.clone();
            p[field] = json!("other-product");
            assert!(validate_token(
                &signed(&config, &key, &p),
                &config,
                "test-device-0123456789",
                1700000001
            )
            .is_err());
        }
    }
    #[test]
    fn rejects_missing_typed_fields_future_issued_and_oversized_lifetimes() {
        let (config, key, payload) = fixture();
        for field in [
            "iss",
            "aud",
            "sub",
            "device_hash",
            "plan",
            "grant_version",
            "iat",
            "nbf",
            "exp",
            "license_expires_at",
        ] {
            let mut p = payload.clone();
            p.as_object_mut().unwrap().remove(field);
            assert!(
                validate_token(
                    &signed(&config, &key, &p),
                    &config,
                    "test-device-0123456789",
                    1700000001
                )
                .is_err(),
                "{field}"
            );
        }
        for (field, value) in [
            ("iat", json!(1700000100)),
            ("nbf", json!(1700000100)),
            ("exp", json!(1700086401)),
            ("exp", json!("1700086400")),
            ("license_expires_at", json!(1700000100)),
            ("plan", json!("forever")),
            ("sub", json!("")),
            ("grant_version", json!(0)),
        ] {
            let mut p = payload.clone();
            p[field] = value;
            assert!(
                validate_token(
                    &signed(&config, &key, &p),
                    &config,
                    "test-device-0123456789",
                    1700000001
                )
                .is_err(),
                "{field}"
            );
        }
        assert!(validate_token(
            &signed(&config, &key, &payload),
            &config,
            "test-device-0123456789",
            1700086400
        )
        .is_err());
    }
}
