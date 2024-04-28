use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{cond, cut, fail, map_res, opt, value};
use nom::error::context;
use nom::multi::fold_many0;
use nom::sequence::{delimited, preceded};

use crate::ast::{
    Expression, Item, MixinCall, MixinCallArgument, MixinDeclarationArgument, SimpleSelector,
};
use crate::lexer::{ident, parse, symbol, token};
use crate::parser::expression::{
    comma_separated_arg_value, detached_ruleset, semicolon_separated_arg_value,
};
use crate::parser::selector::{class_selector, id_selector};
use crate::{parser, ParseResult};

pub fn mixin_declaration(input: &str) -> ParseResult<Item> {
    let (input, selector) = token(mixin_simple_selector)(input)?;
    let (input, arguments) =
        delimited(symbol("("), mixin_declaration_arguments, symbol(")"))(input)?;
    let (input, block) = parser::guarded_block(input)?;
    Ok((
        input,
        Item::MixinDeclaration {
            selector,
            arguments,
            block,
        },
    ))
}

fn mixin_call(input: &str) -> ParseResult<MixinCall> {
    // TODO: Parse arguments

    let (input, selector) = mixin_selector(input)?;
    let (input, _) = symbol("(")(input)?;
    let (input, arguments) = mixin_call_arguments(input)?;
    let (input, _) = symbol(")")(input)?;
    Ok((
        input,
        MixinCall {
            selector,
            arguments,
        },
    ))
}

pub fn mixin_call_item(input: &str) -> ParseResult<Item> {
    let (input, mixin_call) = mixin_call(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((input, Item::MixinCall(mixin_call)))
}

pub fn mixin_call_expression(input: &str) -> ParseResult<Expression> {
    let (input, mixin_call) = mixin_call(input)?;
    Ok((input, Expression::MixinCall(mixin_call, vec![])))
}

fn mixin_selector(input: &str) -> ParseResult<Vec<SimpleSelector>> {
    let (input, first) = token(mixin_simple_selector)(input)?;

    token(fold_many0(
        preceded(mixin_combinator, mixin_simple_selector),
        move || vec![first.clone()],
        |mut acc, item| {
            acc.push(item);
            acc
        },
    ))(input)
}

fn mixin_simple_selector(input: &str) -> ParseResult<SimpleSelector> {
    alt((id_selector, class_selector))(input)
}

/// Consume a LESS mixin combinator (e.g. ``, ` `, ` > `)
fn mixin_combinator(input: &str) -> ParseResult<()> {
    value((), parse(opt(symbol(">"))))(input)
}

enum MixinArgument<'i> {
    /// A variable name (e.g. `@color`), with an optional (default) value (e.g. `@color: blue`)
    Variable {
        name: Cow<'i, str>,
        value: Option<Expression<'i>>,
    },
    /// A literal value (e.g. `blue`)
    Literal { value: Expression<'i> },
    /// A variadic argument (e.g. `...`), optionally with a name (e.g. `@rest...`)
    Variadic { name: Option<Cow<'i, str>> },
}

/// Converts a list of comma-separated mixin arguments to a single semicolon-separated argument.
fn to_semicolon_separated(args: Vec<MixinArgument>) -> Result<MixinArgument, &'static str> {
    let mut args_it = args.into_iter();

    let mut values = vec![];

    // Handle the first argument separately, as it may be a named argument
    let name = match args_it.next() {
        Some(MixinArgument::Variable { name, value }) => {
            if let Some(value) = value {
                values.push(value);
            }
            Some(name)
        }
        Some(MixinArgument::Literal { value }) => {
            values.push(value);
            None
        }
        Some(MixinArgument::Variadic { .. }) => {
            return Err("Variadic arguments must be the last argument");
        }
        None => None,
    };

    // Handle the rest of the arguments
    for arg in args_it {
        match arg {
            MixinArgument::Literal { value } => {
                values.push(value);
            }
            MixinArgument::Variable { .. } => {
                return Err("Cannot mix comma-separated and semicolon-separated arguments");
            }
            MixinArgument::Variadic { .. } => {
                return Err("Variadic arguments must be the last argument");
            }
        }
    }

    // TODO: Special handling for detached rulesets?
    let value = if values.is_empty() {
        None
    } else {
        Some(Expression::CommaList(values))
    };
    let arg = match (name, value) {
        (Some(name), value) => MixinArgument::Variable { name, value },
        (None, Some(value)) => MixinArgument::Literal { value },
        _ => {
            return Err("No arguments provided");
        }
    };

    Ok(arg)
}

/// Parse a list of generic mixin arguments, to be transformed into declaration or call arguments.
fn mixin_arguments(mut input: &str) -> ParseResult<Vec<MixinArgument>> {
    enum Separator {
        Comma,
        Semicolon,
    }

    let mut args = vec![];
    let mut separator = Separator::Comma;

    loop {
        // Try parse a variable name
        let name_result = token(preceded(tag("@"), ident))(input);
        let name = name_result.ok().map(|(next_input, name)| {
            input = next_input;
            name
        });

        // Try parse a variadic argument
        if let Ok((next_input, _)) = token(tag("..."))(input) {
            input = next_input;
            args.push(MixinArgument::Variadic { name });

            // Variadic arguments must be the last argument
            break;
        }

        // Try parse a value
        let value_result = preceded(
            // If we have a name, we must have a colon before the value
            cond(name.is_some(), token(tag(":"))),
            // Colon must be followed by a value, so we can use cut to prevent backtracking
            cut(alt((
                // TODO: Detached ruleset should not be allowed in some cases. But maybe this can be
                //  handled when converting to explicit Mixin(Declaration/Call)Argument structs?
                detached_ruleset,
                match separator {
                    Separator::Comma => comma_separated_arg_value,
                    Separator::Semicolon => semicolon_separated_arg_value,
                },
            ))),
        )(input);
        let value = value_result.ok().map(|(next_input, value)| {
            input = next_input;
            value
        });

        // Push the argument to the list, or break if we've reached the end
        match (name, value) {
            // If we have a name, we have a variable
            (Some(name), value) => args.push(MixinArgument::Variable { name, value }),
            // If we have a value, we have a literal
            (None, Some(value)) => args.push(MixinArgument::Literal { value }),
            // If we have neither a name nor a value, we've reached the end
            _ => break,
        };

        // Parse a separator
        match separator {
            Separator::Comma => {
                if let Ok((next_input, _)) = token(tag(","))(input) {
                    input = next_input;
                } else if let Ok((next_input, _)) = token(tag(";"))(input) {
                    input = next_input;
                    separator = Separator::Semicolon;

                    // Adjust collected args for semicolon separation
                    match to_semicolon_separated(args) {
                        Ok(arg) => {
                            args = vec![arg];
                        }
                        Err(e) => {
                            // TODO: Better error handling
                            return context(e, fail)(input);
                        }
                    }
                } else {
                    // If we don't have a comma or semicolon, we've reached the end
                    break;
                }
            }
            Separator::Semicolon => {
                if let Ok((next_input, _)) = token(tag(";"))(input) {
                    input = next_input;
                } else {
                    // If we don't have a semicolon, we've reached the end
                    break;
                }
            }
        }
    }

    Ok((input, args))
}

impl<'i> TryFrom<MixinArgument<'i>> for MixinDeclarationArgument<'i> {
    type Error = ();

    fn try_from(value: MixinArgument<'i>) -> Result<Self, Self::Error> {
        match value {
            MixinArgument::Variable { name, value } => Ok(MixinDeclarationArgument::Variable {
                name,
                default: value,
            }),
            MixinArgument::Literal { value } => Ok(MixinDeclarationArgument::Literal { value }),
            MixinArgument::Variadic { name } => Ok(MixinDeclarationArgument::Variadic { name }),
        }
    }
}

fn mixin_declaration_arguments(input: &str) -> ParseResult<Vec<MixinDeclarationArgument>> {
    map_res(mixin_arguments, |args| {
        args.into_iter().map(TryInto::try_into).collect()
    })(input)
}

impl<'i> TryFrom<MixinArgument<'i>> for MixinCallArgument<'i> {
    type Error = ();

    fn try_from(value: MixinArgument<'i>) -> Result<Self, Self::Error> {
        match value {
            MixinArgument::Variable {
                name,
                value: Some(value),
            } => Ok(MixinCallArgument {
                name: Some(name),
                value,
            }),
            MixinArgument::Variable { name, value: _ } => Ok(MixinCallArgument {
                name: None,
                value: Expression::Variable(name),
            }),
            MixinArgument::Literal { value } => Ok(MixinCallArgument { name: None, value }),
            MixinArgument::Variadic { .. } => Err(()),
        }
    }
}

fn mixin_call_arguments(input: &str) -> ParseResult<Vec<MixinCallArgument>> {
    map_res(mixin_arguments, |args| {
        args.into_iter().map(TryInto::try_into).collect()
    })(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::{Expression, GuardedBlock, Item, MixinDeclarationArgument, SimpleSelector};
    use crate::parser::mixin::{mixin_declaration, mixin_declaration_arguments};

    #[test]
    fn test_mixin_declaration_arguments() {
        // Single values
        assert_eq!(
            mixin_declaration_arguments("@color"),
            Ok((
                "",
                vec![MixinDeclarationArgument::Variable {
                    name: "color".into(),
                    default: None,
                }]
            ))
        );
        assert_eq!(
            mixin_declaration_arguments("@color: blue"),
            Ok((
                "",
                vec![MixinDeclarationArgument::Variable {
                    name: "color".into(),
                    default: Some(Expression::SpaceList(vec![Expression::Ident(
                        "blue".into()
                    )])),
                }]
            ))
        );
        assert_eq!(
            mixin_declaration_arguments("blue"),
            Ok((
                "",
                vec![MixinDeclarationArgument::Literal {
                    value: Expression::SpaceList(vec![Expression::Ident("blue".into())])
                }]
            ))
        );
        assert_eq!(
            mixin_declaration_arguments("..."),
            Ok(("", vec![MixinDeclarationArgument::Variadic { name: None }]))
        );
        assert_eq!(
            mixin_declaration_arguments("@rest..."),
            Ok((
                "",
                vec![MixinDeclarationArgument::Variadic {
                    name: Some("rest".into())
                }]
            ))
        );

        // Comma separated values
        assert_eq!(
            mixin_declaration_arguments("@width, @height"),
            Ok((
                "",
                vec![
                    MixinDeclarationArgument::Variable {
                        name: "width".into(),
                        default: None,
                    },
                    MixinDeclarationArgument::Variable {
                        name: "height".into(),
                        default: None,
                    },
                ]
            ))
        );
        assert_eq!(
            mixin_declaration_arguments("@width: 50px, @height: @global-height, @rest..."),
            Ok((
                "",
                vec![
                    MixinDeclarationArgument::Variable {
                        name: "width".into(),
                        default: Some(Expression::SpaceList(vec![Expression::Numeric(
                            50.0,
                            Some("px".into())
                        )])),
                    },
                    MixinDeclarationArgument::Variable {
                        name: "height".into(),
                        default: Some(Expression::SpaceList(vec![Expression::Variable(
                            "global-height".into()
                        )])),
                    },
                    MixinDeclarationArgument::Variadic {
                        name: Some("rest".into())
                    }
                ]
            ))
        );
        assert_eq!(
            mixin_declaration_arguments("@colors: red, green, blue"),
            Ok((
                "",
                vec![
                    MixinDeclarationArgument::Variable {
                        name: "colors".into(),
                        default: Some(Expression::SpaceList(vec![Expression::Ident("red".into())])),
                    },
                    MixinDeclarationArgument::Literal {
                        value: Expression::SpaceList(vec![Expression::Ident("green".into())]),
                    },
                    MixinDeclarationArgument::Literal {
                        value: Expression::SpaceList(vec![Expression::Ident("blue".into())]),
                    }
                ]
            ))
        );

        // Semicolon separated values
        assert_eq!(
            mixin_declaration_arguments("@colors: red, green, blue;"),
            Ok((
                "",
                vec![MixinDeclarationArgument::Variable {
                    name: "colors".into(),
                    default: Some(Expression::CommaList(vec![
                        Expression::SpaceList(vec![Expression::Ident("red".into())]),
                        Expression::SpaceList(vec![Expression::Ident("green".into())]),
                        Expression::SpaceList(vec![Expression::Ident("blue".into())]),
                    ])),
                },]
            ))
        );
    }

    #[test]
    fn test_mixin_declaration() {
        assert_eq!(
            mixin_declaration("#lib() { }"),
            Ok((
                "",
                Item::MixinDeclaration {
                    selector: SimpleSelector::Id("lib".into()),
                    arguments: vec![],
                    block: GuardedBlock {
                        guard: None,
                        items: vec![]
                    },
                },
            ))
        );
        assert_eq!(
            mixin_declaration(".test () { }"),
            Ok((
                "",
                Item::MixinDeclaration {
                    selector: SimpleSelector::Class("test".into()),
                    arguments: vec![],
                    block: GuardedBlock {
                        guard: None,
                        items: vec![]
                    },
                },
            ))
        );
        assert_eq!(
            mixin_declaration(".guarded() when (true) { }"),
            Ok((
                "",
                Item::MixinDeclaration {
                    selector: SimpleSelector::Class("guarded".into()),
                    arguments: vec![],
                    block: GuardedBlock {
                        guard: Some(Expression::Ident("true".into())),
                        items: vec![]
                    },
                },
            ))
        );
        assert_eq!(
            mixin_declaration(".test(@color: blue) { }"),
            Ok((
                "",
                Item::MixinDeclaration {
                    selector: SimpleSelector::Class("test".into()),
                    arguments: vec![MixinDeclarationArgument::Variable {
                        name: "color".into(),
                        default: Some(Expression::SpaceList(vec![Expression::Ident(
                            "blue".into()
                        )]))
                    }],
                    block: GuardedBlock {
                        guard: None,
                        items: vec![]
                    },
                },
            ))
        );
    }
}
