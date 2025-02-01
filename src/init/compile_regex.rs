// use std::str::FromStr;

// use tracing::info;

// use crate::util::now::std_now;

// const EMAIL_REGEX: &'static str = r#"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])"#;

// pub fn get_email_regex() -> regex::Regex {
//     let start = std_now();
//     match regex::Regex::from_str(EMAIL_REGEX) {
//         Ok(rgx) => {
//             info!(?
//                 "Email regex compiled and validated in {:?}",
//                 start.elapsed()
//             );
//             rgx
//         }
//         Err(e) => {
//             panic!("Could not compile email regex: {:?}", e);
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_valid_email() {
//         let email_regex = get_email_regex();
//         assert!(email_regex.is_match("user.name+tag+sorting@example.com"));
//         assert!(email_regex.is_match("user@subdomain.example.com"));
//         assert!(email_regex.is_match("admin@mailserver1"));
//         assert!(email_regex.is_match("user@[192.168.1.1]"));
//         assert!(email_regex.is_match("\"Fred Bloggs\"@example.com"));
//     }

//     #[test]
//     fn test_invalid_email() {
//         let email_regex = get_email_regex();
//         assert!(!email_regex.is_match("example@.com"));
//         assert!(!email_regex.is_match("A@b@c@example.com"));
//         assert!(!email_regex.is_match("just\"not\"right@example.com"));
//         assert!(!email_regex.is_match(r#"this is\"not\allowed@example.com"#));
//     }

//     #[test]
//     fn test_regex_compilation() {
//         // Ensures that the regex compiles without error
//         let _ = get_email_regex();
//     }
// }
