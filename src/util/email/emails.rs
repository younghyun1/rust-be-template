pub const PASSWORD_RESET_EMAIL: &str = include_str!("./password_reset.html");
pub const VALIDATE_EMAIL_EMAIL: &str = include_str!("./validate_email.html");

pub struct PasswordResetEmail {
    pub email: String,
}

impl Default for PasswordResetEmail {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordResetEmail {
    pub fn new() -> Self {
        PasswordResetEmail {
            email: PASSWORD_RESET_EMAIL.to_string(),
        }
    }

    pub fn set_link(mut self, link: &str) -> Self {
        self.email = self.email.replace("$1", link);
        self
    }

    pub fn to_message(self, user_email: &str) -> lettre::Message {
        lettre::Message::builder()
            .from("Cyhdev Forums <donotreply@cyhdev.com>".parse().unwrap())
            .to(user_email.parse().unwrap())
            .subject("Reset Your Password")
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(self.email)
            .unwrap()
    }
}

pub struct ValidateEmailEmail {
    pub email: String,
}

impl Default for ValidateEmailEmail {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidateEmailEmail {
    pub fn new() -> Self {
        ValidateEmailEmail {
            email: VALIDATE_EMAIL_EMAIL.to_string(),
        }
    }

    pub fn set_fields(
        mut self,
        valid_until: chrono::DateTime<chrono::Utc>,
        token_id: uuid::Uuid,
    ) -> Self {
        self.email = self
            .email
            .replace(
                "$1",
                &format!(
                    "https://cyhdev.com/account/signup/validate-email?email_validation_token_id={token_id}"
                ),
            )
            .replace("$2", &valid_until.to_string());
        self
    }

    pub fn to_message(self, user_email: &str) -> lettre::Message {
        lettre::Message::builder()
            .from("Cyhdev Forums <donotreply@cyhdev.com>".parse().unwrap())
            .to(user_email.parse().unwrap())
            .subject("Validate Your Email")
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(self.email)
            .unwrap()
    }
}
