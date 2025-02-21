#[inline(always)]
pub fn generate_slug(title: &str) -> String {
    let slug = title.to_lowercase().replace(' ', "-");
    slug
}
