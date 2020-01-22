use super::*;

#[test]
fn test_token() {
    assert_eq!(
        token(tag("test"))("test  /* foo */ // bar"),
        Ok(("", "test"))
    );
}

#[test]
fn test_symbol() {
    assert_eq!(
        symbol("test")("test  /* foo */ // bar"),
        Ok(("", "test"))
    );
}

#[test]
fn test_name() {
    let cases = vec![
        ("name", Ok(("", "name".into()))),
        ("name rest", Ok((" rest", "name".into()))),
        ("-name", Ok(("", "-name".into()))),
        ("--name", Ok(("", "--name".into()))),
        ("-0", Err(Error(("-0", ErrorKind::Fix)))),
    ];

    for (input, expected) in cases {
        assert_eq!(ident(input), expected);
    }
}

#[test]
fn test_at_keyword() {
    let cases = vec![
        ("@name", Ok(("", "name".into()))),
        ("@name rest", Ok(("rest", "name".into()))),
    ];

    for (input, expected) in cases {
        assert_eq!(at_keyword(input), expected);
    }
}

#[test]
fn test_numeric() {
    let cases = vec![
        ("42", Ok(("", (42_f32, None)))),
        ("42%", Ok(("", (42_f32, Some("%".into()))))),
        ("42px", Ok(("", (42_f32, Some("px".into()))))),
    ];

    for (input, expected) in cases {
        assert_eq!(numeric(input), expected);
    }
}

#[test]
fn test_number() {
    let cases = vec![
        ("1", Ok(("", 1_f32))),
        ("-1", Ok(("", -1_f32))),
        ("3.141", Ok(("", 3.141_f32))),
        ("1.5e2", Ok(("", 150_f32))),
        (".707", Ok(("", 0.707_f32))),
    ];

    for (input, expected) in cases {
        assert_eq!(number(input), expected);
    }
}