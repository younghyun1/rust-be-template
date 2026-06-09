/// Generate a URL-safe slug: lowercase ASCII alphanumerics, with each run of
/// other characters collapsed to a single '-', and leading/trailing '-' trimmed.
///
/// Note: non-ASCII characters are dropped, so a fully non-ASCII title yields an
/// empty slug. If non-Latin titles must remain meaningful, swap in a
/// transliteration crate (e.g. `deunicode`/`slug`).
pub fn generate_slug(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut prev_dash = true; // start true so leading separators are dropped
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}
