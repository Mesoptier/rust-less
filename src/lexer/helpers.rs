/// https://www.w3.org/TR/css-syntax-3/#digit
pub fn is_digit(c: char) -> bool {
    nom::character::is_digit(c as u8)
}

/// https://www.w3.org/TR/css-syntax-3/#hex-digit
pub fn is_hex_digit(c: char) -> bool {
    nom::character::is_hex_digit(c as u8)
}

/// https://www.w3.org/TR/css-syntax-3/#letter
pub fn is_letter(c: char) -> bool {
    nom::character::is_alphabetic(c as u8)
}

/// https://www.w3.org/TR/css-syntax-3/#non-ascii-code-point
pub fn is_non_ascii(c: char) -> bool {
    c as u32 >= 0x80
}

/// https://www.w3.org/TR/css-syntax-3/#name-start-code-point
pub fn is_name_start(c: char) -> bool {
    is_letter(c) || is_non_ascii(c) || c == '_'
}

/// https://www.w3.org/TR/css-syntax-3/#name-start-code-point
pub fn is_name(c: char) -> bool {
    is_name_start(c) || is_digit(c) || c == '-'
}

/// https://www.w3.org/TR/css-syntax-3/#starts-with-a-valid-escape
pub fn is_valid_escape(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next() == Some('\\') && chars.next() != Some('\n')
}

/// https://www.w3.org/TR/css-syntax-3/#would-start-an-identifier
pub fn would_start_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some('-') => match chars.next() {
            Some(c) if is_name_start(c) => true,
            Some('-') => true,
            _ => is_valid_escape(&s[1..])
        },
        Some(c) if is_name_start(c) => true,
        Some('\\') => is_valid_escape(s),
        _ => false,
    }
}
