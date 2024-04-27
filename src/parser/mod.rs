use nom::branch::alt;
use nom::combinator::{cut, map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, terminated, tuple};

use crate::ast::*;
use crate::lexer::{at_keyword, ident, parse, symbol, token};
use crate::parser::expression::{
    boolean_expression, declaration_value, variable_declaration_value,
};
use crate::parser::mixin::{mixin_call_item, mixin_declaration};
use crate::parser::selector::selector_group;
use crate::ParseResult;

#[cfg(test)]
mod tests;

mod expression;
mod mixin;
mod selector;
mod string;

pub(crate) fn parse_stylesheet(input: &str) -> ParseResult<Stylesheet> {
    parse(stylesheet)(input)
}

fn stylesheet(input: &str) -> ParseResult<Stylesheet> {
    let (input, items) = list_of_items(input)?;
    Ok((input, Stylesheet { items }))
}

fn guarded_block(input: &str) -> ParseResult<GuardedBlock> {
    let (input, guard) = opt(delimited(
        preceded(symbol("when"), symbol("(")),
        boolean_expression,
        symbol(")"),
    ))(input)?;
    let (input, items) = block_of_items(input)?;
    Ok((input, GuardedBlock { guard, items }))
}

fn block_of_items(input: &str) -> ParseResult<Vec<Item>> {
    let (input, _) = symbol("{")(input)?;
    // We're definitely in a block, so we can use cut to prevent backtracking
    cut(terminated(list_of_items, symbol("}")))(input)
}

fn list_of_items(input: &str) -> ParseResult<Vec<Item>> {
    many0(item)(input)
}

fn item(input: &str) -> ParseResult<Item> {
    // FIXME: There is a lot of backtracking going on here
    // TODO: Support regular function calls (specifically each(...) calls)
    alt((
        mixin_declaration,
        declaration,
        mixin_call_item,
        qualified_rule,
        variable_declaration,
        variable_call,
        //        at_rule,
    ))(input)
}

fn declaration(input: &str) -> ParseResult<Item> {
    // TODO: Parse LESS property merge syntax

    let (input, name) = terminated(token(ident), symbol(":"))(input)?;
    // We're definitely in a declaration, so we can use cut to prevent backtracking
    let (input, (value, important, _)) =
        cut(tuple((declaration_value, important, symbol(";"))))(input)?;

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
fn important(input: &str) -> ParseResult<bool> {
    map(opt(symbol("!important")), |o| o.is_some())(input)
}

fn qualified_rule(input: &str) -> ParseResult<Item> {
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

//fn at_rule(input: &str) -> ParseResult<Item> {
//    let (input, name) = at_keyword(input)?;
//}

fn variable_declaration(input: &str) -> ParseResult<Item> {
    let (input, name) = terminated(at_keyword, symbol(":"))(input)?;
    // We're definitely in a declaration, so we can use cut to prevent backtracking
    let (input, value) = cut(terminated(variable_declaration_value, symbol(";")))(input)?;
    Ok((input, Item::VariableDeclaration { name, value }))
}

fn variable_call(input: &str) -> ParseResult<Item> {
    let (input, name) = at_keyword(input)?;
    let (input, _) = symbol("()")(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((input, Item::VariableCall { name }))
}
