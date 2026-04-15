/// Converts a string to a URL-safe slug.
/// "My Cool Route" → "my-cool-route"
pub fn slugify(s: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = false;

    for ch in s.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen && !slug.is_empty() {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    // Trim trailing hyphen
    if slug.ends_with('-') {
        slug.pop();
    }

    slug
}
