use nom::combinator::{fail, success};
use nom::IResult;

/// Tests whether a predicate `f` holds true for the given input string,
/// without consuming it.
///
/// Result is `Ok((I, ())` if predicate returns true, `Err(...)` otherwise.
pub fn peek_pred<'i, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, ()>
    where F: Fn(&'i str) -> bool
{
    move |input: &'i str| {
        if f(input) {
            success(())(input)
        } else {
            fail(input)
        }
    }
}
