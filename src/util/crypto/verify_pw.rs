use anyhow::Result;
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
pub async fn verify_pw(password: &str, expected_hash: &str) -> Result<bool> {
    let password = password.to_owned();
    let expected_hash = expected_hash.to_owned();
    tokio::task::spawn_blocking(move || {
        let argon2 = Argon2::default();

        let parsed_hash = PasswordHash::new(&expected_hash).map_err(|e| anyhow::anyhow!(e))?;

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    })
    .await?
}
