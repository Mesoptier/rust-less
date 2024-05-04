use chumsky::input::SpannedInput;
use chumsky::prelude::*;

use crate::ast::*;
use crate::lexer::{Delim, Span, Spanned, Token, TokenTree};

type ParserInput<'tokens, 'src> =
    SpannedInput<TokenTree<'src>, Span, &'tokens [Spanned<TokenTree<'src>>]>;
type ParserExtra<'tokens, 'src> = extra::Err<Rich<'tokens, TokenTree<'src>, Span>>;

fn parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<Stylesheet<'tokens, 'src>>,
    ParserExtra<'tokens, 'src>,
> + Clone {
    let whitespace_or_comment = select_ref!(TokenTree::Token(Token::Whitespace) | TokenTree::Token(Token::Comment(_)) => ());
    let junk = whitespace_or_comment.repeated().ignored();

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
                select_ref!(TokenTree::Token(Token::Symbol('@')) => ()),
                select_ref!(TokenTree::Token(Token::Ident(ident)) => *ident),
                junk.or_not(),
                select_ref!(TokenTree::Token(Token::Symbol(':')) => ()),
                any()
                    .and_is(select_ref!(TokenTree::Token(Token::Symbol(';')) => ()).not())
                    .repeated()
                    .to_slice(),
                choice((
                    select_ref!(TokenTree::Token(Token::Symbol(';')) => ()),
                    end(),
                )),
            ))
            .map(|(_, name, _, _, value, _)| Item::VariableDeclaration { name, value })
        };

        // let item_variable_call = todo();

        // Parse an at-rule
        let item_at_rule = {
            let item_at_rule_end = select_ref!(
                TokenTree::Token(Token::Symbol(';')) => (),
                TokenTree::Tree(delim, _) if delim == &Delim::Brace => ()
            );
            let item_at_rule_opt_block = choice((
                end().to(None),
                just(TokenTree::Token(Token::Symbol(';'))).to(None),
                rule_block.map(Some),
            ));
            group((
                select_ref!(TokenTree::Token(Token::Symbol('@')) => ()),
                select_ref!(TokenTree::Token(Token::Ident(ident)) => ident),
                // Parse the prelude up to eof, semicolon, or block
                any().and_is(item_at_rule_end.not()).repeated().to_slice(),
                // Parse the optional block
                item_at_rule_opt_block,
            ))
            .map(|(_, name, prelude, block)| Item::AtRule {
                name,
                prelude,
                block,
            })
        };

        // let item_mixin_rule = todo();
        // let item_qualified_rule = todo();
        // let item_declaration = todo();
        // let item_mixin_call = todo();

        let item = choice((
            item_variable_declaration,
            // item_variable_call,
            item_at_rule,
            // item_mixin_rule,
            // item_qualified_rule,
            // item_declaration,
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
    use crate::ast::*;
    use chumsky::prelude::*;

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
}
