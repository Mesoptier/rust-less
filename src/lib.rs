use winnow::Parser;

pub mod ast;
mod lexer;
mod parser;

pub fn parse(input: &str) -> ast::Stylesheet {
    let tokens = lexer::tokenize(input).unwrap();
    parser::stylesheet.parse(tokens.as_slice()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let input = r#"
            @foo();
            @bar(1, 2, 3);
        "#;
        let stylesheet = parse(input);
        assert_eq!(
            stylesheet,
            ast::Stylesheet {
                items: vec![
                    ast::Item::VariableCall {
                        name: "foo".into(),
                        arguments: vec![]
                    },
                    ast::Item::VariableCall {
                        name: "bar".into(),
                        arguments: vec![
                            lexer::TokenTree::Token(lexer::Token::Number(1.0)),
                            lexer::TokenTree::Token(lexer::Token::Symbol(',')),
                            lexer::TokenTree::Token(lexer::Token::Whitespace),
                            lexer::TokenTree::Token(lexer::Token::Number(2.0)),
                            lexer::TokenTree::Token(lexer::Token::Symbol(',')),
                            lexer::TokenTree::Token(lexer::Token::Whitespace),
                            lexer::TokenTree::Token(lexer::Token::Number(3.0)),
                        ]
                    }
                ]
            }
        );
    }
}
