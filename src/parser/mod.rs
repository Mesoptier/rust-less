use chumsky::input::SpannedInput;
use chumsky::prelude::*;

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

        // Parse a variable declaration
        let item_variable_declaration = {
            group((
                at_ident.then_ignore(junk.or_not()).then_ignore(symbol(':')),
                // Parse component values up to a semicolon or eof
                any().and_is(symbol(';').not()).repeated().to_slice(),
                choice((symbol(';'), end())),
            ))
            .map(|(name, value, _)| Item::VariableDeclaration { name, value })
        };

        // let item_variable_call = todo();

        // Parse an at-rule
        let item_at_rule = {
            let item_at_rule_end = select_ref!(
                TokenTree::Token(Token::Symbol(';')) => (),
                TokenTree::Tree(delim, _) if delim == &Delim::Brace => ()
            );
            group((
                at_ident,
                // Parse the prelude up to eof, semicolon, or block
                any().and_is(item_at_rule_end.not()).repeated().to_slice(),
                // Parse the optional block
                choice((
                    end().to(None),
                    symbol(';').to(None),
                    rule_block.clone().map(Some),
                )),
            ))
            .map(|(name, prelude, block)| Item::AtRule {
                name,
                prelude,
                block,
            })
        };

        // let item_mixin_rule = todo();
        // let item_qualified_rule = todo();

        // Parse a declaration
        let item_declaration = {
            group((
                // Parse the declaration name
                ident
                    .map(DeclarationName::Literal)
                    .then_ignore(junk.or_not())
                    .then_ignore(symbol(':')),
                // Parse component values up to a semicolon or eof
                any()
                    .and_is(symbol(';').not())
                    .repeated()
                    .to_slice()
                    .then_ignore(choice((symbol(';'), end()))),
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

                Item::Declaration {
                    name,
                    value,
                    important,
                }
            })
        };

        // let item_mixin_call = todo();

        let item = choice((
            item_variable_declaration,
            // item_variable_call,
            item_at_rule,
            // item_mixin_rule,
            // item_qualified_rule,
            item_declaration,
            // item_mixin_call,
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
                        Item::AtRule {
                            name: "foo",
                            prelude: &[],
                            block: None,
                        },
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
                        Item::AtRule {
                            name: "foo",
                            prelude: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(4, 5)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(5, 8))
                            ],
                            block: None,
                        },
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
                        Item::AtRule {
                            name: "foo",
                            prelude: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(4, 5)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(5, 8)),
                                (TokenTree::Token(Token::Whitespace), Span::new(8, 9)),
                            ],
                            block: Some(vec![(
                                Item::AtRule {
                                    name: "baz",
                                    prelude: &[],
                                    block: None,
                                },
                                Span::new(11, 16)
                            )]),
                        },
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
                        Item::VariableDeclaration {
                            name: "foo",
                            value: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(5, 6)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(6, 9))
                            ],
                        },
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
                        Item::Declaration {
                            name: DeclarationName::Literal("foo"),
                            value: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(4, 5)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(5, 8))
                            ],
                            important: false,
                        },
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
                        Item::Declaration {
                            name: DeclarationName::Literal("foo"),
                            value: &[
                                (TokenTree::Token(Token::Whitespace), Span::new(4, 5)),
                                (TokenTree::Token(Token::Ident("bar")), Span::new(5, 8))
                            ],
                            important: true,
                        },
                        Span::new(0, 20)
                    )]
                },
                Span::new(0, input.len())
            ))
        );
    }
}
