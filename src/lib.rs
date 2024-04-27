pub mod ast;
mod lexer;
mod parser;
mod util;

type ParseError<'i> = nom::error::Error<&'i str>;
type ParseResult<'i, O> = nom::IResult<&'i str, O, ParseError<'i>>;

pub fn parse(input: &str) -> Result<ast::Stylesheet, String> {
    match nom::combinator::all_consuming(parser::parse_stylesheet)(input) {
        Ok((_, stylesheet)) => Ok(stylesheet),
        Err(err) => Err(format!("{}", err)),
    }
}