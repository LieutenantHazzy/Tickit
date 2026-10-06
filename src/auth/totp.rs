use totp_rs::{Algorithm, Secret, TOTP};

pub fn generate_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

pub fn generate_totp_url(secret: &str, email: &str, issuer: &str) -> anyhow::Result<String> {
    let secret = Secret::Encoded(secret.to_string());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes()?,
        Some(issuer.to_string()),
        email.to_string(),
    )?;
    Ok(totp.get_url())
}

pub fn verify_totp(secret: &str, code: &str) -> bool {
    let Ok(secret_bytes) = Secret::Encoded(secret.to_string()).to_bytes() else {
        return false;
    };
    let Ok(totp) = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        "tickit".to_string(),
    ) else {
        return false;
    };
    totp.check_current(code).unwrap_or(false)
}
