#[derive(serde_derive::Deserialize)]
pub struct SignupRequest<'a, 'b, 'c> {
    pub name: &'a str,
    pub password: &'b str,
    pub email: &'c str,
}
