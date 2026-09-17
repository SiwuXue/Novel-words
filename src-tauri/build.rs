mod license_public_key;

fn main() {
    use std::{env, fs, path::PathBuf};
    let default_pem="-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAQgbUyT1MwMwOKV2ylsLPee1wsDY2IliSpxFYSiA4oFc=\n-----END PUBLIC KEY-----\n";
    let variables = [
        (
            "NOVEL_WORDS_LICENSE_URL",
            "SERVER_URL",
            "https://license.wuyiuou.top",
        ),
        ("NOVEL_WORDS_LICENSE_PRODUCT_ID", "PRODUCT_ID", "NovelWords"),
        (
            "NOVEL_WORDS_LICENSE_ISSUER",
            "ISSUER",
            "PrismKey_NovelWords",
        ),
        ("NOVEL_WORDS_LICENSE_CARD_PREFIX", "CARD_PREFIX", "CY"),
    ];
    let mut source = String::new();
    for (variable, name, fallback) in variables {
        println!("cargo:rerun-if-env-changed={variable}");
        let value = env::var(variable).unwrap_or_else(|_| fallback.into());
        if name == "SERVER_URL"
            && env::var("PROFILE").as_deref() == Ok("release")
            && !value.starts_with("https://")
        {
            panic!("Release license endpoint must use HTTPS");
        }
        source.push_str(&format!("pub const {name}: &str = {value:?};\n"));
    }
    println!("cargo:rerun-if-env-changed=NOVEL_WORDS_LICENSE_PUBLIC_KEY_FILE");
    let pem = match env::var("NOVEL_WORDS_LICENSE_PUBLIC_KEY_FILE") {
        Ok(path) => {
            println!("cargo:rerun-if-changed={path}");
            fs::read_to_string(path).expect("Cannot read Ed25519 public key file")
        }
        Err(_) => default_pem.into(),
    };
    // Validate before writing any key material to generated source or artifacts.
    license_public_key::parse_public_key(&pem)
        .expect("License key must be an Ed25519 SPKI public PEM key");
    source.push_str(&format!("pub const PUBLIC_KEY_PEM: &str = {pem:?};\n"));
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("license_config.rs"),
        source,
    )
    .unwrap();
    tauri_build::build()
}
