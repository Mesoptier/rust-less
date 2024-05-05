use chumsky::input::SpannedInput;
use chumsky::prelude::*;
use std::marker::PhantomData;

use crate::ast::*;
use crate::lexer::{Delim, Span, Spanned, Token, TokenTree};

type ParserInput<'tokens, 'src> =
    SpannedInput<TokenTree<'src>, Span, &'tokens [Spanned<TokenTree<'src>>]>;
type ParserExtra<'tokens, 'src> = extra::Err<Rich<'tokens, TokenTree<'src>, Span>>;

fn strip_trailing_junk<'tokens, 'src>(
    mut value: &'tokens [Spanned<TokenTree<'src>>],
) -> &'tokens [Spanned<TokenTree<'src>>] {
    while let Some(((TokenTree::Token(Token::Whitespace | Token::Comment(_)), _), rest_value)) =
        value.split_last()
    {
        value = rest_value;
    }
    value
}

fn parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Stylesheet<'tokens, 'src>>,
    ParserExtra<'tokens, 'src>,
> + Clone {
    let whitespace_or_comment = select_ref!(TokenTree::Token(Token::Whitespace) | TokenTree::Token(Token::Comment(_)) => ());
    let junk = whitespace_or_comment.repeated().ignored();
    let symbol =
        |symbol: char| select_ref!(TokenTree::Token(Token::Symbol(s)) if s == &symbol => ());
    let ident = select_ref!(TokenTree::Token(Token::Ident(ident)) => *ident);
    let at_ident = symbol('@').ignore_then(ident);

    // Item parsers
    let list_of_items = recursive(|list_of_items| {
        // Parses a TokenTree::Tree of a specific delimiter and returns its contents as a slice
        let tree = |delim: Delim| {
            select_ref!(
                TokenTree::Tree(d, tts) if d == &delim
                    => tts.as_slice().spanned(Span::splat(tts.len()))
            )
        };

        // Parse a rule's block
        let rule_block = list_of_items.nested_in(tree(Delim::Brace));

        // Parse a Declaration
        let declaration = {
            let declaration_name = choice((
                ident.map(DeclarationName::Ident),
                at_ident.map(DeclarationName::Variable),
                // TODO: Support LESS interpolation in declaration names
            ));

            // Parse component values up to a semicolon or eof
            let declaration_value = any().and_is(symbol(';').not()).repeated().to_slice();

            group((
                declaration_name
                    .then_ignore(junk)
                    .then_ignore(symbol(':'))
                    .then_ignore(junk),
                declaration_value.then_ignore(choice((symbol(';'), end()))),
            ))
            .map(|(name, mut value)| {
                value = strip_trailing_junk(value);

                // Split off the !important flag
                let important = {
                    value
                        .split_last_chunk::<2>()
                        .filter(|(_, chunk)| {
                            matches!(
                                chunk,
                                [
                                    (TokenTree::Token(Token::Symbol('!')), _),
                                    (TokenTree::Token(Token::Ident("important")), _),
                                ]
                            )
                        })
                        .inspect(|(rest_value, _)| value = rest_value)
                        .is_some()
                };

                value = strip_trailing_junk(value);

                Declaration {
                    name,
                    value,
                    important,
                }
            })
        };

        // Parse an AtRule
        let at_rule = {
            // Parse the prelude up to eof, semicolon, or block.
            let at_rule_prelude = any()
                .and_is(
                    select_ref!(
                        TokenTree::Token(Token::Symbol(';')) => (),
                        TokenTree::Tree(delim, _) if delim == &Delim::Brace => (),
                    )
                    .not(),
                )
                .repeated()
                .to_slice();

            // Parse the end of the at-rule.
            let at_rule_end = choice((
                end().to(None),
                symbol(';').to(None),
                rule_block.clone().map(Some),
            ));

            group((at_ident, at_rule_prelude, at_rule_end)).map(|(name, prelude, block)| {
                AtRule::Generic(GenericAtRule {
                    name,
                    prelude,
                    block,
                })
            })
        };

        // Parse a QualifiedRule
        let qualified_rule = {
            // Parse the prelude up to eof, semicolon, or block. Eof and semicolon are parse errors,
            // which we'll deal with when parsing the block.
            let qualified_rule_prelude = any()
                .and_is(
                    select_ref!(
                        TokenTree::Token(Token::Symbol(';')) => (),
                        TokenTree::Tree(delim, _) if delim == &Delim::Brace => (),
                    )
                    .not(),
                )
                .repeated()
                .to_slice();

            group((
                qualified_rule_prelude,
                // TODO: Deal with eof or semicolon as parse errors
                rule_block.clone(),
            ))
            .map(|(prelude, block)| QualifiedRule::Generic(GenericRule { prelude, block }))
        };

        // Parse a Call
        let call = {
            let call_end = choice((end(), symbol(';')));

            // Parse a MixinCall
            let mixin_call = {
                // TODO: Support namespaced selectors (e.g. `.foo.bar` or `#foo > .bar`).
                let mixin_call_selector = symbol('.').then(ident).to_slice();
                // TODO: Parse mixin arguments
                let mixin_call_arguments =
                    select_ref!(TokenTree::Tree(Delim::Paren, tts) => tts.as_slice());
                group((
                    mixin_call_selector,
                    mixin_call_arguments.then_ignore(call_end),
                ))
                .map(|(selector, arguments)| MixinCall {
                    selector,
                    arguments,
                })
            };

            // Parse a VariableCall
            let variable_call = at_ident
                .then_ignore(
                    select_ref!(TokenTree::Tree(Delim::Paren, tts) if tts.is_empty() => ()),
                )
                .then_ignore(call_end)
                .map(|name| VariableCall {
                    name,
                    _lookups: PhantomData,
                });

            // Parse a FunctionCall
            let function_call = group((
                ident,
                select_ref!(TokenTree::Tree(Delim::Paren, tts) => tts.as_slice())
                    .then_ignore(call_end),
            ))
            .map(|(name, arguments)| FunctionCall { name, arguments });

            choice((
                mixin_call.map(Call::Mixin),
                variable_call.map(Call::Variable),
                function_call.map(Call::Function),
            ))
        };

        // Parse an Item
        let item = choice((
            declaration.map(Item::Declaration),
            call.map(Item::Call),
            at_rule.map(Item::AtRule),
            qualified_rule.map(Item::QualifiedRule),
        ))
        .map_with(|item, e| (item, e.span()));

        // Parse a list of items separated by junk (whitespace or comments)
        item.separated_by(junk)
            .allow_leading()
            .allow_trailing()
            .collect()
    });

    // A stylesheet is just a list of items
    list_of_items.map_with(|items, e| (Stylesheet { items }, e.span()))
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;
    use std::marker::PhantomData;

    use crate::ast::*;
    use crate::lexer::{lexer, Span, Token, TokenTree};
    use crate::parser::parser;

    #[test]
    fn test_item_at_rule() {
        // Parse an at-rule with no prelude or block
        let input = "@foo;";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::AtRule(AtRule::Generic(GenericAtRule {
                            name: "foo",
                            prelude: &[],
                            block: None,
                        })),
                        Span::new(0, 5)
                    )]
                },
                Span::new(0, input.len())
            ))
        );

        // Parse an at-rule with a simple prelude and no block
        let input = "@foo bar;";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::AtRule(AtRule::Generic(GenericAtRule {
                            name: "foo",
                            prelude: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(4, 5)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(5, 8))
                            ],
                            block: None,
                        })),
                        Span::new(0, 9)
                    )]
                },
                Span::new(0, input.len())
            ))
        );

        // Parse an at-rule with a simple prelude and block
        let input = "@foo bar { @baz; }";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::AtRule(AtRule::Generic(GenericAtRule {
                            name: "foo",
                            prelude: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(4, 5)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(5, 8)),
                                (TokenTree::Token(Token::Whitespace), Span::new(8, 9)),
                            ],
                            block: Some(vec![(
                                Item::AtRule(AtRule::Generic(GenericAtRule {
                                    name: "baz",
                                    prelude: &[],
                                    block: None,
                                })),
                                Span::new(11, 16)
                            )]),
                        })),
                        Span::new(0, 18)
                    )]
                },
                Span::new(0, input.len())
            ))
        );
    }

    #[test]
    fn test_item_variable_declaration() {
        // Parse a variable declaration
        let input = "@foo: bar;";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::Declaration(Declaration {
                            name: DeclarationName::Variable("foo"),
                            value: &[(TokenTree::Token(Token::Ident("bar")), Span::new(6, 9))],
                            important: false,
                        }),
                        Span::new(0, 10)
                    )]
                },
                Span::new(0, input.len())
            ))
        );
    }

    #[test]
    fn test_item_declaration() {
        // Parse a declaration
        let input = "foo: bar;";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::Declaration(Declaration {
                            name: DeclarationName::Ident("foo"),
                            value: &[(TokenTree::Token(Token::Ident("bar")), Span::new(5, 8))],
                            important: false,
                        }),
                        Span::new(0, 9)
                    )]
                },
                Span::new(0, input.len())
            ))
        );

        // Parse a declaration with important
        let input = "foo: bar !important;";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::Declaration(Declaration {
                            name: DeclarationName::Ident("foo"),
                            value: &[(TokenTree::Token(Token::Ident("bar")), Span::new(5, 8))],
                            important: true,
                        }),
                        Span::new(0, 20)
                    )]
                },
                Span::new(0, input.len())
            ))
        );
    }

    #[test]
    fn test_item_qualified_rule() {
        // Parse a qualified rule
        let input = "foo { bar: baz; }";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::QualifiedRule(QualifiedRule::Generic(GenericRule {
                            prelude: &[
                                (TokenTree::Token(Token::Ident("foo")), Span::new(0, 3)),
                                (TokenTree::Token(Token::Whitespace), Span::new(3, 4)),
                            ],
                            block: vec![(
                                Item::Declaration(Declaration {
                                    name: DeclarationName::Ident("bar"),
                                    value: &[(
                                        TokenTree::Token(Token::Ident("baz")),
                                        Span::new(11, 14)
                                    )],
                                    important: false,
                                }),
                                Span::new(6, 15)
                            )],
                        })),
                        Span::new(0, 17)
                    )]
                },
                Span::new(0, input.len())
            ))
        );
    }

    #[test]
    fn test_item_call() {
        // Parse a mixin call
        let input = ".foo(@arg: blue);";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::Call(Call::Mixin(MixinCall {
                            selector: &[
                                (TokenTree::Token(Token::Symbol('.')), Span::new(0, 1)),
                                (TokenTree::Token(Token::Ident("foo")), Span::new(1, 4))
                            ],
                            arguments: &[
                                (TokenTree::Token(Token::Symbol('@')), Span::new(5, 6)),
                                (TokenTree::Token(Token::Ident("arg")), Span::new(6, 9)),
                                (TokenTree::Token(Token::Symbol(':')), Span::new(9, 10)),
                                (TokenTree::Token(Token::Whitespace), Span::new(10, 11)),
                                (TokenTree::Token(Token::Ident("blue")), Span::new(11, 15)),
                            ],
                        })),
                        Span::new(0, 17)
                    )]
                },
                Span::new(0, input.len())
            ))
        );

        // Parse a variable call
        let input = "@foo();";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::Call(Call::Variable(VariableCall {
                            name: "foo",
                            _lookups: PhantomData,
                        })),
                        Span::new(0, 7)
                    )]
                },
                Span::new(0, input.len())
            ))
        );

        // Parse a function call
        let input = "foo();";
        let tts = lexer().parse(input).unwrap();
        let result = parser()
            .parse((&tts).spanned(Span::splat(tts.len())))
            .into_result();
        assert_eq!(
            result,
            Ok((
                Stylesheet {
                    items: vec![(
                        Item::Call(Call::Function(FunctionCall {
                            name: "foo",
                            arguments: &[],
                        })),
                        Span::new(0, 6)
                    )]
                },
                Span::new(0, input.len())
            ))
        );
    }
}
