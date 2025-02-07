#[inline(always)]
pub fn validate_username(username: &str) -> bool {
    // Enhanced validation logic for username
    let is_non_empty = !username.is_empty();
    let is_valid_length = !username.is_empty() && username.len() <= 20;
    let is_valid_char = username.chars().all(|c| c.is_alphanumeric()); // includes hangul, etc

    is_non_empty && is_valid_length && is_valid_char
}

#[inline(always)]
pub fn validate_password_form(password: &str) -> bool {
    if password.len() < 8 {
        return false;
    }

    let mut has_lowercase: bool = false;
    let mut has_uppercase: bool = false;
    let mut has_ascii_digit: bool = false;

    for ch in password.chars() {
        if ch.is_lowercase() {
            has_lowercase = true;
        }
        if ch.is_uppercase() {
            has_uppercase = true;
        }
        if ch.is_ascii_digit() {
            has_ascii_digit = true;
        }
    }

    has_lowercase && has_uppercase && has_ascii_digit
}
