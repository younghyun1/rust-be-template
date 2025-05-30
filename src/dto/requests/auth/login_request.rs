use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(serde_derive::Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct LoginRequest {
    pub user_email: String,
    pub user_password: String,
}
