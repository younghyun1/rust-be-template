use chrono::Utc;
use uuid::Uuid;

pub const DEFAULT_SESSION_DURATION: chrono::Duration = chrono::Duration::hours(1);

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_country: i32,
    pub user_language: i32,
    pub is_email_verified: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
}

impl Session {
    pub fn is_unexpired(&self) -> bool {
        let now = Utc::now();

        self.created_at < now && self.expires_at > now
    }

    pub fn get_user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn get_user_name(&self) -> &str {
        &self.user_name
    }

    pub fn get_user_country(&self) -> i32 {
        self.user_country
    }

    pub fn get_user_language(&self) -> i32 {
        self.user_language
    }

    pub fn get_is_email_verified(&self) -> bool {
        self.is_email_verified
    }
}
