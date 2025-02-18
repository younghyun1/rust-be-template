pub const PASSWORD_RESET_EMAIL: &str = include_str!("./password_reset.html");

pub struct PasswordResetEmail {
    pub email: String,
}

impl Default for PasswordResetEmail {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordResetEmail {
    pub fn new() -> PasswordResetEmail {
        PasswordResetEmail {
            email: PASSWORD_RESET_EMAIL.to_string(),
        }
    }

    pub fn set_link(&mut self, link: &str) {
        self.email = self.email.replace("$1", link);
    }
}
