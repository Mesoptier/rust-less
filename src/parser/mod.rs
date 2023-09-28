use nom::branch::alt;
use nom::combinator::{cut, map, opt, value};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::IResult;

use crate::ast::*;
use crate::lexer::{at_keyword, ident, parse, symbol, token};
use crate::parser::mixin::{mixin_selector, mixin_simple_selector};
use crate::parser::selector::selector_group;
use crate::parser::value::{declaration_value, variable_declaration_value};

#[cfg(test)]
mod tests;

mod mixin;
mod selector;
mod string;
mod value;

fn parse_stylesheet(input: &str) -> IResult<&str, Stylesheet> {
    parse(stylesheet)(input)
}

fn stylesheet(input: &str) -> IResult<&str, Stylesheet> {
    let (input, items) = list_of_items(input)?;
    Ok((input, Stylesheet { items }))
}

fn guarded_block(input: &str) -> IResult<&str, GuardedBlock> {
    let (input, guard) = opt(delimited(
        preceded(symbol("when"), symbol("(")),
        // TODO: Parse actual boolean expression
        value(Guard, symbol("true")),
        symbol(")"),
    ))(input)?;
    let (input, items) = block_of_items(input)?;
    Ok((input, GuardedBlock { guard, items }))
}

fn block_of_items(input: &str) -> IResult<&str, Vec<Item>> {
    delimited(symbol("{"), cut(list_of_items), symbol("}"))(input)
}

fn list_of_items(input: &str) -> IResult<&str, Vec<Item>> {
    many0(item)(input)
}

fn item(input: &str) -> IResult<&str, Item> {
    // FIXME: There is a lot of backtracking going on here
    // TODO: Support regular function calls (specifically each(...) calls)
    alt((
        mixin_declaration,
        declaration,
        mixin_call,
        qualified_rule,
        variable_declaration,
        variable_call,
        //        at_rule,
    ))(input)
}

fn declaration(input: &str) -> IResult<&str, Item> {
    // TODO: Parse LESS property merge syntax

    let (input, name) = token(ident)(input)?;
    let (input, _) = symbol(":")(input)?;
    let (input, value) = declaration_value(input)?;
    let (input, important) = important(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((
        input,
        Item::Declaration {
            name,
            value,
            important,
        },
    ))
}

/// Parse an !important token
fn important(input: &str) -> IResult<&str, bool> {
    map(opt(symbol("!important")), |o| o.is_some())(input)
}

fn qualified_rule(input: &str) -> IResult<&str, Item> {
    let (input, selector_group) = selector_group(input)?;
    let (input, block) = guarded_block(input)?;
    Ok((
        input,
        Item::QualifiedRule {
            selector_group,
            block,
        },
    ))
}

//fn at_rule(input: &str) -> IResult<&str, Item> {
//    let (input, name) = at_keyword(input)?;
//}

fn mixin_declaration(input: &str) -> IResult<&str, Item> {
    // TODO: Parse arguments

    let (input, selector) = token(mixin_simple_selector)(input)?;
    let (input, _) = symbol("()")(input)?;
    let (input, block) = guarded_block(input)?;
    Ok((
        input,
        Item::MixinDeclaration {
            selector,
            arguments: vec![],
            block,
        },
    ))
}

fn mixin_call(input: &str) -> IResult<&str, Item> {
    // TODO: Parse arguments
    // TODO: Parse lookups

    let (input, selector) = mixin_selector(input)?;
    let (input, _) = symbol("()")(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((input, Item::MixinCall { selector }))
}

fn variable_declaration(input: &str) -> IResult<&str, Item> {
    let (input, name) = at_keyword(input)?;
    let (input, _) = symbol(":")(input)?;
    let (input, value) = variable_declaration_value(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((input, Item::VariableDeclaration { name, value }))
}

fn variable_call(input: &str) -> IResult<&str, Item> {
    let (input, name) = at_keyword(input)?;
    let (input, _) = symbol("()")(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((input, Item::VariableCall { name }))
}
