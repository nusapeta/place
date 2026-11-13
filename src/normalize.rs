//! Shared name normalization for builder and server.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

const ABBREVIATIONS: &[(&str, &str)] = &[
    ("jl", "jalan"),
    ("jln", "jalan"),
    ("kec", "kecamatan"),
    ("kab", "kabupaten"),
    ("kel", "kelurahan"),
    ("kp", "kampung"),
    ("no", "nomor"),
    ("rt", "rukun tetangga"),
    ("rw", "rukun warga"),
];

/// Normalize a name: NFKC, lowercase, strip diacritics, expand common
/// Indonesian abbreviations, keep alphanumerics, collapse whitespace.
pub fn normalize(input: &str) -> String {
    let folded: String = input
        .nfkc()
        .flat_map(char::to_lowercase)
        .nfd()
        .filter(|ch| !is_combining_mark(*ch))
        .collect();

    let mut normalized = String::with_capacity(folded.len());
    let mut token = String::new();
    for ch in folded.chars() {
        if ch.is_alphanumeric() {
            token.push(ch);
        } else if !token.is_empty() {
            push_token(&mut normalized, &token);
            token.clear();
        }
    }
    if !token.is_empty() {
        push_token(&mut normalized, &token);
    }
    normalized
}

fn push_token(output: &mut String, token: &str) {
    let replacement = ABBREVIATIONS
        .iter()
        .find_map(|(short, full)| (*short == token).then_some(*full));
    let value = replacement.unwrap_or(token);
    if !output.is_empty() {
        output.push(' ');
    }
    output.push_str(value);
}

pub fn tokens(input: &str) -> Vec<String> {
    let normalized = normalize(input);
    if normalized.is_empty() {
        Vec::new()
    } else {
        normalized.split(' ').map(ToOwned::to_owned).collect()
    }
}

pub fn prefix_upper_bound(prefix: &str) -> Option<String> {
    let mut bytes = prefix.as_bytes().to_vec();
    while let Some(last) = bytes.pop() {
        if last < 255 {
            bytes.push(last + 1);
            return String::from_utf8(bytes).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_diacritics_and_punctuation() {
        assert_eq!(normalize("  Jl.  Malioboro!! "), "jalan malioboro");
        assert_eq!(normalize("Café  Déjà"), "cafe deja");
        assert_eq!(normalize("Kec. Gondokusuman"), "kecamatan gondokusuman");
        assert_eq!(normalize("Kab. Sleman"), "kabupaten sleman");
        assert_eq!(normalize("Kel. Terban"), "kelurahan terban");
        assert_eq!(normalize("Kp. Baru"), "kampung baru");
        assert_eq!(normalize("No. 7"), "nomor 7");
        assert_eq!(normalize("a  b"), "a b");
        assert_eq!(normalize("Jalan   Malioboro"), "jalan malioboro");
        assert_eq!(normalize("Ｊａｋａｒｔａ"), "jakarta");
        assert_eq!(normalize("Kota\tContoh\nBaru"), "kota contoh baru");
        assert_eq!(normalize("..."), "");
        assert_eq!(tokens(""), Vec::<String>::new());
    }

    #[test]
    fn keeps_single_character_tokens() {
        assert_eq!(tokens("X Y"), vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn prefix_bound_is_strict() {
        assert_eq!(prefix_upper_bound("abc").as_deref(), Some("abd"));
        assert_eq!(prefix_upper_bound("\u{10ffff}"), None);
    }
}
