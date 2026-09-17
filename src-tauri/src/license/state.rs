use super::{
    crypto::validate_token, storage::Storage, CachedLicense, ClientConfig, LicenseError,
    LicenseStatus,
};
use chrono::DateTime;
use serde_json::Value;
use std::{
    collections::HashSet,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
enum EditTarget {
    Novel(i64),
    Chapter(i64),
}
struct Session {
    cache: Option<CachedLicense>,
    anchor: Instant,
    mode: Option<String>,
    error: Option<String>,
    retry_deadline: Option<Instant>,
    startup_checked: bool,
    cache_failed: bool,
    opened_targets: HashSet<EditTarget>,
    was_authorized: bool,
    locked_at: Option<Instant>,
}
pub struct LicenseState {
    pub config: ClientConfig,
    pub storage: Storage,
    inner: Mutex<Session>,
    operation: tokio::sync::Mutex<()>,
}
impl LicenseState {
    pub fn new(config: ClientConfig, root: &Path, device: &str) -> Result<Self, LicenseError> {
        let storage = Storage::new(root, device)?;
        let loaded = storage.load();
        let error = loaded.as_ref().err().map(|e| e.code.clone());
        let cache_failed = loaded.is_err();
        let cache = loaded.unwrap_or(None);
        Ok(Self {
            config,
            storage,
            inner: Mutex::new(Session {
                cache,
                anchor: Instant::now(),
                mode: Some("offline".into()),
                error,
                retry_deadline: None,
                startup_checked: false,
                cache_failed,
                opened_targets: HashSet::new(),
                was_authorized: false,
                locked_at: None,
            }),
            operation: tokio::sync::Mutex::new(()),
        })
    }
    fn accept(
        &self,
        card: &str,
        response: Value,
        wall: i64,
    ) -> Result<LicenseStatus, LicenseError> {
        let invalid = || LicenseError::new("TOKEN_INVALID");
        let token = response["token"].as_str().ok_or_else(invalid)?;
        let claims = validate_token(token, &self.config, &self.storage.device_id, wall)?;
        let server_date =
            DateTime::parse_from_rfc3339(response["server_time"].as_str().ok_or_else(invalid)?)
                .map_err(|_| invalid())?;
        let server = server_date.timestamp();
        let activated =
            DateTime::parse_from_rfc3339(response["activated_at"].as_str().ok_or_else(invalid)?)
                .map_err(|_| invalid())?;
        let expires = match response.get("expires_at") {
            Some(Value::Null) => None,
            Some(Value::String(value)) => Some(
                DateTime::parse_from_rfc3339(value)
                    .map_err(|_| invalid())?
                    .timestamp(),
            ),
            _ => return Err(invalid()),
        };
        let plan_days = match claims.plan.as_str() {
            "7d" => Some(7),
            "30d" => Some(30),
            "365d" => Some(365),
            _ => None,
        };
        if activated.timestamp() < 0
            || activated > server_date
            || activated.timestamp() > claims.iat
            || plan_days.is_some_and(|days| {
                expires.and_then(|deadline| deadline.checked_sub(activated.timestamp()))
                    != Some(days * 86400)
            })
        {
            return Err(invalid());
        }
        if response["status"] != "active"
            || response["license_id"] != claims.sub
            || response["product_id"] != claims.aud
            || response["plan"] != claims.plan
            || response["device_hash"] != claims.device_hash
            || response["kid"] != self.config.kid
            || response["token_expires_at"].as_i64() != Some(claims.exp)
            || expires != claims.license_expires_at
            || server < claims.iat
            || server > claims.iat.saturating_add(10)
            || server >= claims.exp
        {
            return Err(invalid());
        }
        let cache = CachedLicense {
            card_key: card.into(),
            token: Some(token.into()),
            server_time: server,
            wall_at_verify: wall,
            observed_wall: wall,
            denial_code: None,
            plan: Some(claims.plan),
            expires_at: claims.license_expires_at,
            token_expires_at: Some(claims.exp),
        };
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?;
        if let Err(error) = self.storage.save(&cache) {
            inner.cache_failed = true;
            return Err(error);
        }
        inner.cache = Some(cache);
        inner.anchor = Instant::now();
        inner.mode = Some("online".into());
        inner.error = None;
        inner.retry_deadline = None;
        inner.startup_checked = true;
        inner.cache_failed = false;
        drop(inner);
        self.status_at(wall, 0)
    }
    fn failure(
        &self,
        code: &str,
        temporary: bool,
        wall: i64,
    ) -> Result<LicenseStatus, LicenseError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?;
        inner.error = Some(code.into());
        inner.startup_checked = true;
        let mut storage_failed = false;
        if temporary {
            inner.mode = Some("offline".into());
        } else {
            inner.mode = None;
            if let Some(cache) = &mut inner.cache {
                cache.token = None;
                cache.denial_code = Some(code.into());
                if self.storage.save(cache).is_err() {
                    // Best effort only: if both operations fail, persistent revocation
                    // is impossible until storage becomes writable or online succeeds.
                    let _ = std::fs::remove_file(self.storage.root.join("license.cache"));
                    storage_failed = true;
                }
            }
        }
        if storage_failed {
            inner.cache_failed = true;
        }
        let elapsed = inner.anchor.elapsed().as_secs();
        drop(inner);
        self.status_at(wall, elapsed)
    }
    fn status_at(&self, wall: i64, elapsed: u64) -> Result<LicenseStatus, LicenseError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?;
        let mut result = LicenseStatus {
            authorized: false,
            mode: None,
            code: Some("ACTIVATION_REQUIRED".into()),
            plan: None,
            expires_at: None,
            token_expires_at: None,
            last_verified_at: None,
            device_id: self.storage.device_id.clone(),
            masked_card_key: None,
            server_url: self.config.server_url.clone(),
            card_prefix: self.config.card_prefix.clone(),
            retry_after_seconds: inner
                .retry_deadline
                .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
                .map(|remaining| remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0))
                .filter(|seconds| *seconds > 0),
        };
        let mode = inner.mode.clone();
        let last_error = inner.error.clone();
        let checked = inner.startup_checked;
        let cache_failed = inner.cache_failed;
        let mut storage_failed = false;
        if let Some(cache) = &mut inner.cache {
            result.plan = cache.plan.clone();
            result.expires_at = cache.expires_at;
            result.token_expires_at = cache.token_expires_at;
            result.last_verified_at = Some(cache.server_time);
            let last = cache
                .card_key
                .chars()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>();
            result.masked_card_key = Some(format!("{}-••••-{last}", self.config.card_prefix));
            let effective = wall.max(
                cache
                    .server_time
                    .saturating_add(i64::try_from(elapsed).unwrap_or(i64::MAX)),
            );
            if cache_failed {
                result.code = Some("CACHE_ERROR".into());
            } else if !checked {
                result.code = Some("VERIFICATION_REQUIRED".into());
            } else if wall.saturating_add(5) < cache.observed_wall
                || wall.saturating_add(5) < cache.wall_at_verify
            {
                result.code = Some("CLOCK_ROLLBACK".into());
            } else if let Some(token) = &cache.token {
                match validate_token(token, &self.config, &self.storage.device_id, effective) {
                    Ok(_) => {
                        result.authorized = true;
                        result.mode = mode;
                        result.code = last_error;
                    }
                    Err(error) => result.code = Some(error.code),
                }
            } else {
                result.code = cache.denial_code.clone().or(last_error);
            }
            if checked && !cache_failed && wall > cache.observed_wall {
                let mut observed = cache.clone();
                observed.observed_wall = wall;
                if self.storage.save(&observed).is_err() {
                    storage_failed = true;
                } else {
                    cache.observed_wall = wall;
                }
            }
        } else if last_error.is_some() {
            result.code = last_error;
        }
        if storage_failed || cache_failed {
            inner.cache_failed = true;
            result.authorized = false;
            result.mode = None;
            result.code = Some("CACHE_ERROR".into());
        }
        if result.authorized {
            inner.was_authorized = true;
            inner.locked_at = None;
        } else {
            if inner.was_authorized && inner.locked_at.is_none() {
                inner.locked_at = Some(Instant::now());
            }
            inner.was_authorized = false;
        }
        Ok(result)
    }
    pub fn status(&self) -> Result<LicenseStatus, LicenseError> {
        let elapsed = self
            .inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?
            .anchor
            .elapsed()
            .as_secs();
        self.status_at(chrono::Utc::now().timestamp(), elapsed)
    }
    pub fn guard(&self, command: &str) -> Result<(), LicenseError> {
        self.guard_with_payload(command, None)
    }
    fn set_retry_after(&self, seconds: Option<u64>) -> Result<(), LicenseError> {
        let deadline = seconds
            .filter(|seconds| *seconds > 0)
            .and_then(|seconds| Instant::now().checked_add(Duration::from_secs(seconds.min(3600))));
        self.inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?
            .retry_deadline = deadline;
        Ok(())
    }
    pub fn guard_with_payload(
        &self,
        command: &str,
        payload: Option<&Value>,
    ) -> Result<(), LicenseError> {
        if public_command(command) {
            return Ok(());
        }
        let status = self.status()?;
        let target = payload.and_then(|body| match command {
            "get_novel_content" | "update_novel_content" => body
                .get("id")
                .and_then(Value::as_i64)
                .filter(|id| *id > 0)
                .map(EditTarget::Novel),
            "get_chapter_content" | "update_chapter_content" => body
                .get("chapterId")
                .and_then(Value::as_i64)
                .filter(|id| *id > 0)
                .map(EditTarget::Chapter),
            _ => None,
        });
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?;
        if status.authorized {
            if matches!(command, "get_novel_content" | "get_chapter_content") {
                if let Some(target) = target {
                    inner.opened_targets.insert(target);
                }
            }
            return Ok(());
        }
        if matches!(command, "update_novel_content" | "update_chapter_content")
            && target.is_some_and(|target| inner.opened_targets.contains(&target))
            && inner
                .locked_at
                .is_some_and(|locked| locked.elapsed() < Duration::from_secs(60))
        {
            return Ok(());
        }
        Err(LicenseError::new(
            status.code.as_deref().unwrap_or("ACTIVATION_REQUIRED"),
        ))
    }
    pub async fn verify(&self) -> Result<LicenseStatus, LicenseError> {
        let _operation = self.operation.lock().await;
        let card = self
            .inner
            .lock()
            .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?
            .cache
            .as_ref()
            .map(|cache| cache.card_key.clone());
        let Some(card) = card else {
            self.inner
                .lock()
                .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?
                .startup_checked = true;
            return self.status();
        };
        match self.request("verify", &card).await {
            Ok(body) => match self.accept(&card, body, chrono::Utc::now().timestamp()) {
                Ok(status) => Ok(status),
                Err(error) => self.failure(&error.code, false, chrono::Utc::now().timestamp()),
            },
            Err((error, temporary)) => {
                self.set_retry_after(error.retry_after_seconds)?;
                self.failure(&error.code, temporary, chrono::Utc::now().timestamp())
            }
        }
    }
    pub async fn activate(&self, key: &str) -> Result<LicenseStatus, LicenseError> {
        let card = key.trim().to_ascii_uppercase();
        if !(8..=128).contains(&card.len())
            || !card.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(LicenseError::new("INVALID_CARD"));
        }
        if !card.starts_with(&format!("{}-", self.config.card_prefix)) {
            return Err(LicenseError::new("WRONG_PRODUCT"));
        }
        let _operation = self.operation.lock().await;
        let result = match self.request("activate", &card).await {
            Ok(body) => self.accept(&card, body, chrono::Utc::now().timestamp()),
            Err((error, _)) => Err(error),
        };
        if let Err(error) = &result {
            let same = self
                .inner
                .lock()
                .map_err(|_| LicenseError::new("LICENSING_UNAVAILABLE"))?
                .cache
                .as_ref()
                .is_some_and(|cache| cache.card_key == card);
            if same {
                self.set_retry_after(error.retry_after_seconds)?;
                let temporary = matches!(
                    error.code.as_str(),
                    "NETWORK_UNAVAILABLE" | "SERVICE_UNAVAILABLE"
                );
                let _ = self.failure(&error.code, temporary, chrono::Utc::now().timestamp());
            }
        }
        result
    }
    async fn request(&self, action: &str, card: &str) -> Result<Value, (LicenseError, bool)> {
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5));
        if self.config.server_url.starts_with("http://127.0.0.1:") {
            builder = builder.no_proxy();
        }
        let client = builder
            .build()
            .map_err(|_| (LicenseError::new("LICENSING_UNAVAILABLE"), false))?;
        let mut response=client.post(format!("{}/api/v1/licenses/{action}",self.config.server_url)).json(&serde_json::json!({"card_key":card,"device_id":self.storage.device_id,"client_version":self.config.client_version})).send().await.map_err(|_|(LicenseError::new("NETWORK_UNAVAILABLE"),true))?;
        let status = response.status();
        let retry = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| seconds.min(3600));
        if status.as_u16() == 429 {
            let mut error = LicenseError::new("RATE_LIMITED");
            error.retry_after_seconds = retry;
            return Err((error, false));
        }
        if status.is_server_error() {
            return Err((LicenseError::new("SERVICE_UNAVAILABLE"), true));
        }
        let mut bytes = Vec::new();
        // Receiving non-5xx headers proves this is a response, not an outage.
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| (LicenseError::new("TOKEN_INVALID"), false))?
        {
            if bytes.len() + chunk.len() > 16384 {
                return Err((LicenseError::new("TOKEN_INVALID"), false));
            }
            bytes.extend_from_slice(&chunk);
        }
        let body: Value = serde_json::from_slice(&bytes)
            .map_err(|_| (LicenseError::new("TOKEN_INVALID"), false))?;
        if !status.is_success() {
            let code = if status.as_u16() == 429 {
                "RATE_LIMITED"
            } else {
                match body["error"]["code"].as_str() {
                    Some(
                        code @ ("INVALID_CARD"
                        | "ACTIVATION_REQUIRED"
                        | "LICENSE_DISABLED"
                        | "LICENSE_EXPIRED"
                        | "DEVICE_MISMATCH"
                        | "INVALID_REQUEST"),
                    ) => code,
                    _ => "TOKEN_INVALID",
                }
            };
            let mut error = LicenseError::new(code);
            error.retry_after_seconds = retry;
            return Err((error, false));
        }
        Ok(body)
    }
}

pub fn public_command(command: &str) -> bool {
    matches!(
        command,
        "get_license_status"
            | "verify_license"
            | "activate_license"
            | "get_app_info"
            | "get_all_settings"
            | "get_setting"
            | "set_setting"
            | "backup_database"
    )
}

#[tauri::command]
pub fn get_license_status(
    state: tauri::State<'_, LicenseState>,
) -> Result<LicenseStatus, LicenseError> {
    state.status()
}
#[tauri::command]
pub async fn verify_license(
    state: tauri::State<'_, LicenseState>,
) -> Result<LicenseStatus, LicenseError> {
    state.verify().await
}
#[tauri::command]
pub async fn activate_license(
    state: tauri::State<'_, LicenseState>,
    card_key: String,
) -> Result<LicenseStatus, LicenseError> {
    state.activate(&card_key).await
}

#[cfg(test)]
mod tests {
    use super::super::crypto::tests::{fixture, signed};
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    fn response(
        config: &ClientConfig,
        key: &ring::signature::Ed25519KeyPair,
        payload: &Value,
    ) -> Value {
        json!({"status":"active","license_id":payload["sub"],"product_id":payload["aud"],"plan":payload["plan"],"activated_at":"2023-11-14T22:13:20.123456Z","expires_at":Utc.timestamp_opt(payload["license_expires_at"].as_i64().unwrap(),0).unwrap().to_rfc3339(),"device_hash":payload["device_hash"],"server_time":"2023-11-14T22:13:21.123456Z","token":signed(config,key,payload),"token_expires_at":payload["exp"],"kid":config.kid})
    }
    #[test]
    fn online_success_offline_restart_and_fixed_deadline_preserve_state() {
        let (config, key, payload) = fixture();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
        assert!(
            state
                .accept(
                    "CY-EXAMPLE-LICENSE",
                    response(&config, &key, &payload),
                    1700000001
                )
                .unwrap()
                .authorized
        );
        let offline = state
            .failure("NETWORK_UNAVAILABLE", true, 1700000010)
            .unwrap();
        assert!(offline.authorized);
        assert_eq!(offline.mode.as_deref(), Some("offline"));
        let restarted = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
        assert!(!restarted.status_at(1700000020, 0).unwrap().authorized);
        assert!(
            restarted
                .failure("NETWORK_UNAVAILABLE", true, 1700000020)
                .unwrap()
                .authorized
        );
        assert!(!restarted.status_at(1700086400, 0).unwrap().authorized);
    }
    #[test]
    fn authoritative_denial_survives_restart_and_blocks_offline_reuse() {
        for code in [
            "LICENSE_DISABLED",
            "LICENSE_EXPIRED",
            "DEVICE_MISMATCH",
            "ACTIVATION_REQUIRED",
            "RATE_LIMITED",
        ] {
            let (config, key, payload) = fixture();
            let dir = super::super::test_dir();
            let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
            state
                .accept(
                    "CY-EXAMPLE-LICENSE",
                    response(&config, &key, &payload),
                    1700000001,
                )
                .unwrap();
            assert!(!state.failure(code, false, 1700000010).unwrap().authorized);
            let restarted = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
            assert!(
                !restarted
                    .failure("NETWORK_UNAVAILABLE", true, 1700000020)
                    .unwrap()
                    .authorized
            );
        }
    }
    #[test]
    fn response_mismatch_never_updates_cache_and_clock_rollback_requires_online() {
        let (config, key, payload) = fixture();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
        for (field, value) in [
            ("product_id", json!("XHS_Download")),
            ("license_id", json!("wrong")),
            ("device_hash", json!("wrong")),
            ("kid", json!("wrong")),
            ("plan", json!("7d")),
            ("token_expires_at", json!(1700086500)),
            ("expires_at", Value::Null),
            ("server_time", json!("2024-11-14T22:13:21Z")),
        ] {
            let mut r = response(&config, &key, &payload);
            r[field] = value;
            assert!(
                state.accept("CY-EXAMPLE-LICENSE", r, 1700000001).is_err(),
                "{field}"
            );
            assert!(state.storage.load().unwrap().is_none());
        }
        state
            .accept(
                "CY-EXAMPLE-LICENSE",
                response(&config, &key, &payload),
                1700000001,
            )
            .unwrap();
        assert!(state.status_at(1700000100, 100).unwrap().authorized);
        assert_eq!(
            state.status_at(1699999900, 101).unwrap().code.as_deref(),
            Some("CLOCK_ROLLBACK")
        );
        let restarted = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
        assert!(!restarted.status_at(1699999900, 0).unwrap().authorized);
    }
    #[tokio::test]
    async fn unlicensed_commands_are_blocked_but_activation_appearance_and_backup_remain_available()
    {
        let (config, _, _) = fixture();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
        for command in [
            "create_novel",
            "get_novel_content",
            "dict_lookup_english",
            "review_vocab_word",
            "export_pdf",
            "import_preset_vocab_book",
            "get_user_vocab_page",
            "update_chapter_content",
        ] {
            assert!(state.guard(command).is_err(), "{command}");
        }
        for command in [
            "get_license_status",
            "verify_license",
            "activate_license",
            "get_app_info",
            "get_all_settings",
            "set_setting",
            "backup_database",
        ] {
            assert!(state.guard(command).is_ok(), "{command}");
        }
        let inactive = state.verify().await.unwrap();
        assert!(!inactive.authorized);
        assert_eq!(inactive.code.as_deref(), Some("ACTIVATION_REQUIRED"));
    }
    #[tokio::test]
    async fn failed_replacement_and_other_product_prefix_do_not_replace_cached_license() {
        let (mut config, key, payload) = fixture();
        config.server_url = "http://127.0.0.1:1".into();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
        state
            .accept(
                "CY-EXAMPLE-LICENSE",
                response(&config, &key, &payload),
                1700000001,
            )
            .unwrap();
        assert_eq!(
            state.activate("XHS-EXAMPLE-CARD").await.unwrap_err().code,
            "WRONG_PRODUCT"
        );
        assert_eq!(
            state.activate("CY-DIFFERENT-CARD").await.unwrap_err().code,
            "NETWORK_UNAVAILABLE"
        );
        assert_eq!(
            state.storage.load().unwrap().unwrap().card_key,
            "CY-EXAMPLE-LICENSE"
        );
        assert!(state.status_at(1700000010, 0).unwrap().authorized);
        let json = serde_json::to_string(&state.status_at(1700000010, 0).unwrap()).unwrap();
        assert!(!json.contains("CY-EXAMPLE-LICENSE"));
        assert!(!json.contains("token\""));
    }

    fn live_response(
        config: &ClientConfig,
        key: &ring::signature::Ed25519KeyPair,
        payload: &mut Value,
    ) -> Value {
        let issued = Utc::now().timestamp() - 1;
        payload["iat"] = json!(issued);
        payload["nbf"] = json!(issued);
        payload["exp"] = json!(issued + 86400);
        payload["license_expires_at"] = json!(issued + 30 * 86400);
        let mut body = response(config, key, payload);
        body["activated_at"] = json!(Utc.timestamp_opt(issued, 0).unwrap().to_rfc3339());
        body["server_time"] = json!(Utc.timestamp_opt(issued + 1, 0).unwrap().to_rfc3339());
        body
    }

    #[test]
    fn successful_responses_require_explicit_nullable_expiry_and_valid_activation_time() {
        let (config, key, payload) = fixture();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
        for value in [
            Value::Null,
            json!(17),
            json!("invalid"),
            json!("2023-11-15T22:13:20Z"),
        ] {
            let mut body = response(&config, &key, &payload);
            body["activated_at"] = value;
            assert_eq!(
                state
                    .accept("CY-EXAMPLE-LICENSE", body, 1700000001)
                    .unwrap_err()
                    .code,
                "TOKEN_INVALID"
            );
        }
        let mut body = response(&config, &key, &payload);
        body.as_object_mut().unwrap().remove("activated_at");
        assert!(state
            .accept("CY-EXAMPLE-LICENSE", body, 1700000001)
            .is_err());
        let mut lifetime = payload;
        lifetime["plan"] = json!("lifetime");
        lifetime["license_expires_at"] = Value::Null;
        let mut body = response(&config, &key, &fixture().2);
        body["plan"] = json!("lifetime");
        body["expires_at"] = Value::Null;
        body["token"] = json!(signed(&config, &key, &lifetime));
        body.as_object_mut().unwrap().remove("expires_at");
        assert!(state
            .accept("CY-EXAMPLE-LICENSE", body, 1700000001)
            .is_err());
    }

    #[test]
    fn cached_startup_requires_an_actual_online_attempt_before_authorizing() {
        let (config, key, payload) = fixture();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
        state
            .accept(
                "CY-EXAMPLE-LICENSE",
                response(&config, &key, &payload),
                1700000001,
            )
            .unwrap();
        let restarted = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
        let pending = restarted.status_at(1700000020, 0).unwrap();
        assert!(!pending.authorized);
        assert_eq!(pending.code.as_deref(), Some("VERIFICATION_REQUIRED"));
        assert!(
            restarted
                .failure("NETWORK_UNAVAILABLE", true, 1700000020)
                .unwrap()
                .authorized
        );
    }

    #[test]
    fn cache_write_failure_stays_locked_until_successful_online_accept() {
        let (config, key, payload) = fixture();
        let dir = super::super::test_dir();
        let state = LicenseState::new(config.clone(), &dir, "test-device-0123456789").unwrap();
        state
            .accept(
                "CY-EXAMPLE-LICENSE",
                response(&config, &key, &payload),
                1700000001,
            )
            .unwrap();
        let root = &state.storage.root;
        let moved = dir.join("moved-license");
        std::fs::rename(root, &moved).unwrap();
        std::fs::write(root, b"not-a-directory").unwrap();
        assert!(!state.status_at(1700000010, 0).unwrap().authorized);
        let repeated = state.status_at(1700000010, 0).unwrap();
        assert!(!repeated.authorized);
        assert_eq!(repeated.code.as_deref(), Some("CACHE_ERROR"));
        std::fs::remove_file(root).unwrap();
        std::fs::rename(moved, root).unwrap();
        assert!(
            !state
                .failure("NETWORK_UNAVAILABLE", true, 1700000011)
                .unwrap()
                .authorized
        );
        assert!(
            state
                .accept(
                    "CY-EXAMPLE-LICENSE",
                    response(&config, &key, &payload),
                    1700000011
                )
                .unwrap()
                .authorized
        );
    }

    #[test]
    fn locked_autosave_cannot_use_an_unscoped_previous_grant() {
        let (config, key, mut payload) = fixture();
        let body = live_response(&config, &key, &mut payload);
        let dir = super::super::test_dir();
        let state = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
        state
            .accept("CY-EXAMPLE-LICENSE", body, Utc::now().timestamp())
            .unwrap();
        state
            .failure("LICENSE_DISABLED", false, Utc::now().timestamp())
            .unwrap();
        assert!(state.guard("update_novel_content").is_err());
        assert!(state.guard("update_chapter_content").is_err());
    }

    #[test]
    fn locked_autosave_is_limited_to_previously_read_targets_and_first_sixty_seconds() {
        let (config, key, mut payload) = fixture();
        let body = live_response(&config, &key, &mut payload);
        let dir = super::super::test_dir();
        let state = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
        state
            .accept("CY-EXAMPLE-LICENSE", body, Utc::now().timestamp())
            .unwrap();
        let novel = json!({"id":7});
        let chapter = json!({"chapterId":9});
        assert!(state
            .guard_with_payload("get_novel_content", Some(&novel))
            .is_ok());
        assert!(state
            .guard_with_payload("get_chapter_content", Some(&chapter))
            .is_ok());
        state
            .failure("LICENSE_DISABLED", false, Utc::now().timestamp())
            .unwrap();
        assert!(state
            .guard_with_payload("update_novel_content", Some(&novel))
            .is_ok());
        assert!(state
            .guard_with_payload("update_chapter_content", Some(&chapter))
            .is_ok());
        assert!(state
            .guard_with_payload("update_novel_content", Some(&json!({"id":9})))
            .is_err());
        assert!(state
            .guard_with_payload("update_chapter_content", Some(&json!({"chapterId":7})))
            .is_err());
        assert!(state
            .guard_with_payload("get_novel_content", Some(&json!({"id":8})))
            .is_err());
        assert!(state
            .guard_with_payload("update_novel_content", Some(&json!({"id":8})))
            .is_err());
        assert!(state
            .guard_with_payload("update_chapter_content", Some(&json!({"chapterId":"9"})))
            .is_err());
        let first = state.inner.lock().unwrap().locked_at.unwrap();
        state
            .failure("NETWORK_UNAVAILABLE", true, Utc::now().timestamp())
            .unwrap();
        assert_eq!(state.inner.lock().unwrap().locked_at, Some(first));
        state.inner.lock().unwrap().locked_at = Some(Instant::now() - Duration::from_secs(60));
        assert!(state
            .guard_with_payload("update_novel_content", Some(&novel))
            .is_err());
        assert!(state
            .guard_with_payload("update_chapter_content", Some(&chapter))
            .is_err());
        assert!(state.guard("export_pdf").is_err());
    }

    fn truncated_http(status: u16) -> (String, std::thread::JoinHandle<()>) {
        truncated_http_retry(status, 30)
    }
    fn truncated_http_retry(status: u16, retry: u64) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 8192];
            let _ = stream.read(&mut request).unwrap();
            let bytes=format!("HTTP/1.1 {status} Test\r\nContent-Length: 1000\r\nContent-Type: application/json\r\nRetry-After: {retry}\r\nConnection: close\r\n\r\n{{\"error\":");
            stream.write_all(bytes.as_bytes()).unwrap();
            stream.flush().unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
        });
        (format!("http://{address}"), server)
    }

    #[tokio::test]
    async fn received_http_headers_never_turn_a_truncated_response_into_offline_permission() {
        for (status, expected, temporary) in [
            (200, "TOKEN_INVALID", false),
            (403, "TOKEN_INVALID", false),
            (429, "RATE_LIMITED", false),
            (503, "SERVICE_UNAVAILABLE", true),
        ] {
            let (url, server) = truncated_http(status);
            let (mut config, _, _) = fixture();
            config.server_url = url;
            let dir = super::super::test_dir();
            let state = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
            let (error, actual) = state
                .request("verify", "CY-EXAMPLE-LICENSE")
                .await
                .unwrap_err();
            server.join().unwrap();
            assert_eq!(error.code, expected, "{status}");
            assert_eq!(actual, temporary, "{status}");
            if status == 429 {
                assert_eq!(error.retry_after_seconds, Some(30));
            }
        }
    }

    #[tokio::test]
    async fn retry_after_counts_down_without_status_polling_extending_the_deadline() {
        for action in ["verify", "activate"] {
            let (url, server) = truncated_http_retry(429, 1);
            let (mut config, key, mut payload) = fixture();
            config.server_url = url;
            let body = live_response(&config, &key, &mut payload);
            let dir = super::super::test_dir();
            let state = LicenseState::new(config, &dir, "test-device-0123456789").unwrap();
            state
                .accept("CY-EXAMPLE-LICENSE", body, Utc::now().timestamp())
                .unwrap();
            if action == "verify" {
                let limited = state.verify().await.unwrap();
                assert_eq!(limited.code.as_deref(), Some("RATE_LIMITED"));
            } else {
                let error = state.activate("CY-EXAMPLE-LICENSE").await.unwrap_err();
                assert_eq!(error.code, "RATE_LIMITED");
            }
            server.join().unwrap();
            assert_eq!(
                state.status().unwrap().retry_after_seconds,
                Some(1),
                "{action}"
            );
            for _ in 0..4 {
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = state.status().unwrap();
            }
            assert_eq!(
                state.status().unwrap().retry_after_seconds,
                None,
                "{action}"
            );
            assert_eq!(
                state.status().unwrap().code.as_deref(),
                Some("RATE_LIMITED")
            );
        }
    }
}
