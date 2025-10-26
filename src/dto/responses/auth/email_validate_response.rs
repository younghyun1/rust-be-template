#[derive(serde_derive::Serialize)]
pub struct EmailValidateResponse {
    pub user_email: String,
    pub verified_at: chrono::DateTime<chrono::Utc>,
    pub time_to_process: std::time::Duration,
}

const EMAIL_VALIDATE_RESPONSE_PAGE: &'static str = include_str!("email_validate_response.html");

pub fn hydrate_email_validate_response_page(response: &EmailValidateResponse) -> String {
    let html = EMAIL_VALIDATE_RESPONSE_PAGE;
    let replacements: [(&'static str, &String); 3] = [
        ("{user_email}", &response.user_email),
        ("{verified_at}", &response.verified_at.to_rfc3339()),
        (
            "{time_to_process}",
            &format!("{:?}", response.time_to_process),
        ),
    ];
    let mut result = html.to_string();
    for (pat, val) in replacements.iter() {
        result = result.replace(pat, val);
    }
    result
}
