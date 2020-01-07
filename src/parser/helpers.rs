/// https://www.w3.org/TR/css-syntax-3/#digit
pub fn is_digit(c: char) -> bool {
    ('0'..='9').contains(&c)
}

/// https://www.w3.org/TR/css-syntax-3/#hex-digit
pub fn is_hex_digit(c: char) -> bool {
    is_digit(c) || ('a'..='f').contains(&c) || ('A'..='F').contains(&c)
}

/// https://www.w3.org/TR/css-syntax-3/#whitespace
pub fn is_whitespace(c: char) -> bool {
    c == '\n' || c == '\t' || c == ' '
}

/// https://www.w3.org/TR/css-syntax-3/#uppercase-letter
pub fn is_uppercase_letter(c: char) -> bool {
    ('A'..='Z').contains(&c)
}

/// https://www.w3.org/TR/css-syntax-3/#lowercase-letter
pub fn is_lowercase_letter(c: char) -> bool {
    ('a'..='z').contains(&c)
}

/// https://www.w3.org/TR/css-syntax-3/#letter
pub fn is_letter(c: char) -> bool {
    is_uppercase_letter(c) || is_lowercase_letter(c)
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
pub fn is_valid_escape(c1: char, c2: char) -> bool {
    c1 == '\\' && c2 != '\n'
}

/// https://www.w3.org/TR/css-syntax-3/#would-start-an-identifier
pub fn would_start_identifier(c1: char, c2: char, c3: char) -> bool {
    match c1 {
        '-' => is_name_start(c2) || c2 == '-' || is_valid_escape(c2, c3),
        _ if is_name_start(c1) => true,
        '\\' => is_valid_escape(c1, c2),
        _ => false,
    }
}

/// https://www.w3.org/TR/css-syntax-3/#starts-with-a-number
pub fn would_start_number(c1: char, c2: char, c3: char) -> bool {
    match c1 {
        '+' | '-' => is_digit(c2) || (c2 == '.' && is_digit(c3)),
        '.' => is_digit(c2),
        c => is_digit(c),
    }
}