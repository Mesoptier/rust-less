pub fn is_hex_digit(c: char) -> bool {
    match c {
        '0'..='9' => true,
        'a'..='f' => true,
        'A'..='F' => true,
        _ => false,
    }
}

pub fn is_whitespace(c: char) -> bool {
    c == '\n' || c == '\t' || c == ' '
}