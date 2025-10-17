use anyhow::Result;
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};

pub async fn hash_pw(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut rand::rngs::OsRng);
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|password_hash| password_hash.to_string())
            .map_err(|e| anyhow::anyhow!(e))
    })
    .await?
}
