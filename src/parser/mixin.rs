use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{cond, fail, opt, value};
use nom::multi::fold_many0;
use nom::sequence::{delimited, preceded};
use nom::IResult;

use crate::ast::{Expression, Item, MixinDeclarationArgument, SimpleSelector};
use crate::lexer::{ident, parse, symbol, token};
use crate::parser;
use crate::parser::expression::{comma_separated_arg_value, semicolon_separated_arg_value};
use crate::parser::selector::{class_selector, id_selector};

pub fn mixin_declaration(input: &str) -> IResult<&str, Item> {
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

pub fn mixin_call(input: &str) -> IResult<&str, Item> {
    // TODO: Parse arguments
    // TODO: Parse lookups

    let (input, selector) = mixin_selector(input)?;
    let (input, _) = symbol("()")(input)?;
    let (input, _) = symbol(";")(input)?;
    Ok((input, Item::MixinCall { selector }))
}

fn mixin_selector(input: &str) -> IResult<&str, Vec<SimpleSelector>> {
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

fn mixin_simple_selector(input: &str) -> IResult<&str, SimpleSelector> {
    alt((id_selector, class_selector))(input)
}

/// Consume a LESS mixin combinator (e.g. ``, ` `, ` > `)
fn mixin_combinator(input: &str) -> IResult<&str, ()> {
    value((), parse(opt(symbol(">"))))(input)
}

fn mixin_declaration_arguments(mut input: &str) -> IResult<&str, Vec<MixinDeclarationArgument>> {
    let mut args = vec![];

    let mut is_semicolon_separated = false;

    loop {
        let name_result = token(preceded(tag("@"), ident))(input);
        let name = match name_result {
            Ok((next_input, name)) => {
                input = next_input;
                Some(name)
            }
            Err(_) => None,
        };

        if let Ok((next_input, _)) = token(tag("..."))(input) {
            input = next_input;
            args.push(MixinDeclarationArgument::Variadic { name });
            break;
        }

        let value_result = preceded(
            cond(name.is_some(), token(tag(":"))),
            alt((if !is_semicolon_separated {
                comma_separated_arg_value
            } else {
                semicolon_separated_arg_value
            },)),
        )(input);
        let value = match value_result {
            Ok((next_input, value)) => {
                input = next_input;
                Some(value)
            }
            Err(_) => None,
        };

        let mut reached_end = match (name, value) {
            (Some(name), default) => {
                args.push(MixinDeclarationArgument::Variable { name, default });
                false
            }
            (None, Some(value)) => {
                args.push(MixinDeclarationArgument::Literal { value });
                false
            }
            _ => true,
        };

        if is_semicolon_separated {
            if let Ok((next_input, _)) = token(tag(";"))(input) {
                input = next_input;
            } else {
                reached_end = true;
            }
        } else {
            if let Ok((next_input, _)) = token(tag(","))(input) {
                input = next_input;
            } else if let Ok((next_input, _)) = token(tag(";"))(input) {
                input = next_input;
                is_semicolon_separated = true;

                // Adjust collected args for semicolon separation
                let mut args_it = args.into_iter();

                let mut values = vec![];
                let name = match args_it.next() {
                    Some(MixinDeclarationArgument::Variable { name, default }) => {
                        if let Some(value) = default {
                            values.push(value);
                        }
                        Some(name)
                    }
                    Some(MixinDeclarationArgument::Literal { value }) => {
                        values.push(value);
                        None
                    }
                    Some(_) => {
                        return fail(input);
                    }
                    None => None,
                };

                for arg in args_it {
                    match arg {
                        MixinDeclarationArgument::Literal { value } => {
                            values.push(value);
                        }
                        _ => {
                            return fail(input);
                        }
                    }
                }

                let value = Expression::CommaList(values);
                let arg = match name {
                    Some(name) => MixinDeclarationArgument::Variable {
                        name,
                        default: Some(value),
                    },
                    None => MixinDeclarationArgument::Literal { value },
                };
                args = vec![arg];
            } else {
                reached_end = true;
            }
        }

        if reached_end {
            break;
        }
    }

    Ok((input, args))
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
