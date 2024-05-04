pub mod ast;
mod lexer;

pub fn parse(input: &str) -> ast::Stylesheet {
    // let tokens = lexer::tokenize(input).unwrap();
    // let tokens = RefStream::new(&tokens);
    // parser::stylesheet.parse(tokens).unwrap()

    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let input = r#"
            @foo: bar;
            @bar: 1, 2, 3;
        
            @foo();
            @bar(1, 2, 3);
        "#;
        let stylesheet = parse(input);
        assert_eq!(
            stylesheet,
            ast::Stylesheet {
                items: vec![
                    ast::Item::VariableDeclaration {
                        name: "foo".into(),
                        value: vec![lexer::TokenTree::Token(lexer::Token::Ident("bar".into()))]
                    },
                    ast::Item::VariableDeclaration {
                        name: "bar".into(),
                        value: vec![
                            lexer::TokenTree::Token(lexer::Token::Number(1.0)),
                            lexer::TokenTree::Token(lexer::Token::Symbol(',')),
                            lexer::TokenTree::Token(lexer::Token::Whitespace),
                            lexer::TokenTree::Token(lexer::Token::Number(2.0)),
                            lexer::TokenTree::Token(lexer::Token::Symbol(',')),
                            lexer::TokenTree::Token(lexer::Token::Whitespace),
                            lexer::TokenTree::Token(lexer::Token::Number(3.0)),
                        ]
                    },
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
