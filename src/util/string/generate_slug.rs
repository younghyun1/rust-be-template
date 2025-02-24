#[inline(always)]
pub fn generate_slug(title: &str) -> String {
    title.to_lowercase().replace(' ', "-")
}
