use nom::IResult;
use nom::bytes::complete::{tag, take_until, take_while};
use nom::multi::{many0, many1};
use nom::branch::alt;
use nom::combinator::{cut, value};
use nom::character::complete::multispace1;

fn whitespace(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace1(input)?;
    Ok((input, ()))
}

fn line_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("//")(input)?;
    let (input, _) = take_while(|c| c != '\n')(input)?;
    Ok((input, ()))
}

fn block_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("/*")(input)?;
    let (input, _) = cut(take_until("*/"))(input)?;
    let (input, _) = tag("*/")(input)?;
    Ok((input, ()))
}

fn junk(input: &str) -> IResult<&str, ()> {
    value((), alt((whitespace, line_comment, block_comment)))(input)
}

pub fn junk0(input: &str) -> IResult<&str, ()> {
    value((), many0(junk))(input)
}

pub fn junk1(input: &str) -> IResult<&str, ()> {
    value((), many1(junk))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::Err::{Failure};
    use nom::error::ErrorKind::TakeUntil;

    #[test]
    fn test_whitespace() {
        let cases = vec![
            (" ", Ok(("", ()))),
            ("\t", Ok(("", ()))),
            ("\n", Ok(("", ()))),
        ];

        for (input, expected) in cases {
            assert_eq!(whitespace(input), expected);
        }
    }

    #[test]
    fn test_line_comment() {
        let cases = vec![
            ("//", Ok(("", ()))),
            ("// comment", Ok(("", ()))),
            ("// comment\n", Ok(("\n", ()))),
        ];

        for (input, expected) in cases {
            assert_eq!(line_comment(input), expected);
        }
    }

    #[test]
    fn test_block_comment() {
        let cases = vec![
            ("/**/", Ok(("", ()))),
            ("/* multiline \n comment */", Ok(("", ()))),
            ("/* eof", Err(Failure((" eof", TakeUntil)))),
        ];

        for (input, expected) in cases {
            assert_eq!(block_comment(input), expected);
        }
    }

}