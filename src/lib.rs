use nom::Finish;

pub mod ast;
mod lexer;
mod parser;
mod tokenizer;
mod util;

type ParseError<'i> = nom::error::VerboseError<&'i str>;
type ParseResult<'i, O> = nom::IResult<&'i str, O, ParseError<'i>>;

pub fn parse(input: &str) -> Result<ast::Stylesheet, ParseError> {
    nom::combinator::all_consuming(parser::parse_stylesheet)(input)
        .finish()
        .map(|(_, stylesheet)| stylesheet)
}
