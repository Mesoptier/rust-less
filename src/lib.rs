pub mod ast;
mod lexer;
mod parser;
mod util;

pub fn parse(input: &str) -> Result<ast::Stylesheet, String> {
    match nom::combinator::all_consuming(parser::parse_stylesheet)(input) {
        Ok((_, stylesheet)) => Ok(stylesheet),
        Err(err) => Err(format!("{}", err)),
    }
}