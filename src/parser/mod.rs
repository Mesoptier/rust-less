use alloc::vec::Vec;
use combine::{Parser, RangeStream, ParseError, satisfy};
use combine::parser::{
    token::token,
    choice::choice,
    repeat::{
        many1,
    }
};
use crate::grammar::*;

mod selector;

//fn ruleset<'i, I>() -> impl Parser<Input = I, Output = Ruleset<'i>>
//{
//
//}

fn variable<'i, I>() -> impl Parser<I, Output=DeclarationName>
where
    I: RangeStream<Token=char, Range=&'i str>,
    I::Error: ParseError<I::Token, I::Range, I::Position>
{
    (
        token('@'),
        many1(satisfy(|c: char| c.is_alphanumeric() || c == '-'))
    )
        .map(|(_, name)| DeclarationName::Variable(name))
        .expected("variable")
}

fn property<'i, I>() -> impl Parser<I, Output=DeclarationName>
where
    I: RangeStream<Token=char, Range=&'i str>,
    I::Error: ParseError<I::Token, I::Range, I::Position>
{
    many1(satisfy(|c: char| c.is_alphanumeric() || c == '-'))
        .map(|name| DeclarationName::Property(name))
        .expected("property")
}

//fn declaration_name<'i, I>() -> impl Parser<I, Output = DeclarationName<'i>>
//    where
//        I: RangeStream<Token = char, Range = &'i str>,
//        I::Error: ParseError<I::Token, I::Range, I::Position>
//{
//    choice::or(variable, property)
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable() {
        let text = "@test-rust";
        let expected = DeclarationName::Variable("test-rust".to_string());

        let result = variable().parse(text);
        assert_eq!(result, Ok((expected, "")));
    }
}

//fn declaration<'i, I>() -> impl Parser<I, Output = Declaration<'i>>
//where
//    I: RangeStream<Token = char, Range = &'i str>,
//    I::Error: ParseError<I::Token, I::Range, I::Position>
//{
//
//}

///// (ruleset | declaration)+
//fn primary_vec<'i, I>() -> impl Parser<Input = I, Output = Vec<Primary<'i>>>
//where
//    I: RangeStream<Token = char, Range = &'i str>,
//    I::Error: ParseError<I::Token, I::Range, I::Position>
//{
//    repeat::many(choice::or(
//        combinator::attempt(ruleset()),
//        combinator::attempt(declaration())
//    ))
//}
//
//pub fn less<'i, I>() -> impl Parser<Input = I, Output = Ruleset<'i>>
//where
//    I: RangeStream<Token = char, Range = &'i str>,
//    I::Error: ParseError<I::Token, I::Range, I::Position>
//{
//
//}