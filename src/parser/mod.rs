use crate::stream::{Stream, PeekAt};
use crate::tokenizer::Token;

#[derive(Debug)]
pub enum Node {
    Stylesheet {
        rules: Vec<Node>,
    },
    AtRule {
        name: String,
        prelude: Vec<Node>,
        block: Box<Option<Node>>,
    },
    Rule {},
    Declaration {},
    VariableDeclaration {
        name: String,
    },
    VariableCall {
        name: String,
    },

    Block {
        value: Vec<Node>,
    },
    Token(Token),
}

type Tokens<L: Iterator<Item=Token>> = Stream<Token, L, [Option<Token>; 3]>;

struct Parser<I> where I: Iterator<Item=Token> {
    input: Tokens<I>,
}

impl<I> Parser<I> where I: Iterator<Item=Token> {
    pub fn new(input: I) -> Self {
        Self {
            input: Tokens::new(input),
        }
    }

    pub fn parse_stylesheet(&mut self) -> Node {
        Node::Stylesheet {
            rules: self.consume_primary(None),
        }
    }

    /// In LESS blocks at all levels can contain rules/declarations/etc,
    /// so we handle those blocks with this 'primary' rule.
    fn consume_primary(&mut self, ending: Option<Token>) -> Vec<Node> {
        let mut rules: Vec<Node> = vec![];

        loop {
            match self.input.consume() {
                Some(Token::Whitespace) => {}
                t if t == ending => return rules,
                None => /* parse error */ return rules,
                // TODO: Handle CDO/CDC tokens (https://www.w3.org/TR/css-syntax-3/#consume-list-of-rules)
                Some(Token::AtKeyword { .. }) => {
                    self.input.reconsume_current();
                    if let Some(rule) = self.consume_at_rule() {
                        rules.push(rule);
                    }
                }
                _ => {}
            }
        }
    }

    fn consume_block(&mut self, ending: Option<Token>) -> Node {
        Node::Block { value: self.consume_primary(ending) }
    }

    fn consume_at_rule(&mut self) -> Option<Node> {
        if let Some(Token::AtKeyword { value }) = self.input.consume() {
            // Skip whitespace
            while let Some(Token::Whitespace) = self.input.peek_at(0) {
                self.input.consume();
            }

            // @var: value;
            // @var: { ... };
            // @var();
            // @var; -> error
            // @var[...]
            // @media ... { ... }
            // @import ...;
            // etc.
            match self.input.consume() {
                Some(Token::Colon) => self.consume_variable_declaration(value),
                Some(Token::LeftParenthesis) => {
                    self.input.reconsume_current();
                    self.consume_variable_call(value)
                }
                Some(Token::LeftSquareBracket) => {
                    // parse error
                    self.input.reconsume_current();
                    None
                }
                _ => {
                    self.input.reconsume_current();
                    Some(self.consume_css_at_rule(value))
                }
            }
        } else {
            panic!();
        }
    }

    fn consume_variable_declaration(&mut self, name: String) -> Option<Node> {
        Some(Node::VariableDeclaration { name })
    }

    fn consume_variable_call(&mut self, name: String) -> Option<Node> {
        Some(Node::VariableCall { name })
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-at-rule
    fn consume_css_at_rule(&mut self, name: String) -> Node {
        let mut prelude: Vec<Node> = vec![];
        let mut block: Box<Option<Node>> = Box::from(None);

        loop {
            match self.input.consume() {
                Some(Token::Semicolon) => return Node::AtRule { name, prelude, block },
                None => /* parse error */ return Node::AtRule { name, prelude, block },
                Some(Token::LeftCurlyBracket) => {
                    block = Box::from(Some(self.consume_block(Some(Token::RightCurlyBracket))));
                    return Node::AtRule { name, prelude, block };
                }
                _ => {
                    self.input.reconsume_current();
                    prelude.push(self.consume_component_value())
                }
            }
        }
    }

    fn consume_component_value(&mut self) -> Node {
        match self.input.consume() {
            Some(Token::LeftCurlyBracket) => self.consume_block(Some(Token::RightCurlyBracket)),
            Some(Token::LeftSquareBracket) => self.consume_block(Some(Token::RightSquareBracket)),
            Some(Token::LeftParenthesis) => self.consume_block(Some(Token::RightParenthesis)),
            Some(token) => Node::Token(token),
            None => panic!() // TODO: Handle properly
        }
    }

    fn consume_qualified_rule(&mut self) -> Option<Node> {
        Some(Node::Rule {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::Tokenizer;

    #[test]
    fn test() {
        let input: &str = r#"
@var : value;
@var();
@var[];
@{var} {}
@media print {
    color: blue;
}
"#;
        let mut tokenizer = Tokenizer::new(input.chars());
        let mut parser = Parser::new(tokenizer);
        println!("{:#?}", parser.parse_stylesheet());
    }
}