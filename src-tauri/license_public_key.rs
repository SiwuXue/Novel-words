use base64::{engine::general_purpose::STANDARD, Engine};

pub fn parse_public_key(pem: &str) -> Result<[u8; 32], &'static str> {
    const ERROR: &str = "Expected an Ed25519 SPKI public PEM key";
    let lines = pem.trim().lines().map(str::trim).collect::<Vec<_>>();
    if lines.first() != Some(&"-----BEGIN PUBLIC KEY-----")
        || lines.last() != Some(&"-----END PUBLIC KEY-----")
        || lines.len() < 3
    {
        return Err(ERROR);
    }
    let der = STANDARD
        .decode(lines[1..lines.len() - 1].join(""))
        .map_err(|_| ERROR)?;
    const PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    if der.len() != 44 || der[..12] != PREFIX {
        return Err(ERROR);
    }
    der[12..].try_into().map_err(|_| ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAQgbUyT1MwMwOKV2ylsLPee1wsDY2IliSpxFYSiA4oFc=\n-----END PUBLIC KEY-----\n";
    #[test]
    fn accepts_only_ed25519_spki_public_pem() {
        assert!(parse_public_key(PUBLIC).is_ok());
        assert!(parse_public_key(&PUBLIC.replace("PUBLIC KEY", "PRIVATE KEY")).is_err());
        assert!(parse_public_key(&PUBLIC.replace("AyEA", "AyEB")).is_err());
        assert!(parse_public_key(&(PUBLIC.to_owned() + PUBLIC)).is_err());
        assert!(parse_public_key("not a public key").is_err());
    }
}
