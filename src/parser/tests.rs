use crate::parser::expression::declaration_value;

use super::*;

macro_rules! selector_group {
    ( $input:expr ) => {
        selector_group($input).unwrap().1
    };
}

#[test]
fn test_stylesheet() {
    let input = r#"
#lib() {
  .colors() {
    @primary: blue;
    @secondary: green;
  }
  .rules(@size) {
    border: @size solid white;
  }
}

.box when (@test[@primary] = blue) {
  width: 100px;
  height: ($width / 2);
}

.bar:extend(.box) {
  @media (min-width: 600px) {
    width: 200px;
    #lib.rules(1px);
  }
}
"#;
    println!("{:#?}", parse_stylesheet(input));
}

#[test]
fn test_qualified_rule() {
    assert_eq!(
        qualified_rule("a { color: blue; }"),
        Ok((
            "",
            Item::QualifiedRule {
                selector_group: selector_group!("a"),
                block: GuardedBlock {
                    guard: None,
                    items: vec![Item::Declaration {
                        name: "color".into(),
                        value: Expression::CommaList(vec![Expression::SpaceList(vec![
                            Expression::Ident("blue".into())
                        ])]),
                        important: false,
                    }]
                }
            }
        ))
    );

    assert_eq!(
        qualified_rule("a when (true) { }"),
        Ok((
            "",
            Item::QualifiedRule {
                selector_group: selector_group!("a"),
                block: GuardedBlock {
                    guard: Some(Expression::Ident("true".into())),
                    items: vec![]
                }
            }
        ))
    );
}

#[test]
fn test_declaration() {
    assert_eq!(
        declaration("color: blue;"),
        Ok((
            "",
            Item::Declaration {
                name: "color".into(),
                value: Expression::CommaList(vec![Expression::SpaceList(vec![Expression::Ident(
                    "blue".into()
                )])]),
                important: false,
            }
        ))
    );
}

#[test]
fn test_declaration_value() {
    assert_eq!(
        declaration_value("0 * 0 + 0 / 0"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![
                Expression::BinaryOperation(
                    BinaryOperator::Add,
                    Expression::BinaryOperation(
                        BinaryOperator::Multiply,
                        Expression::Numeric(0.0, None).into(),
                        Expression::Numeric(0.0, None).into(),
                    )
                    .into(),
                    Expression::BinaryOperation(
                        BinaryOperator::Divide,
                        Expression::Numeric(0.0, None).into(),
                        Expression::Numeric(0.0, None).into(),
                    )
                    .into(),
                )
            ])])
        ))
    );
    assert_eq!(
        declaration_value("blue"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![Expression::Ident(
                "blue".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("5px solid red"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![
                Expression::Numeric(5.0, Some("px".into())),
                Expression::Ident("solid".into()),
                Expression::Ident("red".into()),
            ])])
        ))
    );
    assert_eq!(
        declaration_value("@primary"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![Expression::Variable(
                "primary".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("@colors[primary]"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![
                Expression::VariableLookup("colors".into(), vec![Lookup::Ident("primary".into())])
            ])])
        ))
    );
    assert_eq!(
        declaration_value("$color"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![Expression::Property(
                "color".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("rgba(0, 0, 0, 0.5)"),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![Expression::FunctionCall(
                "rgba".into(),
                Expression::SemicolonList(vec![Expression::CommaList(vec![
                    Expression::SpaceList(vec![Expression::Numeric(0.0, None)]),
                    Expression::SpaceList(vec![Expression::Numeric(0.0, None)]),
                    Expression::SpaceList(vec![Expression::Numeric(0.0, None)]),
                    Expression::SpaceList(vec![Expression::Numeric(0.5, None)]),
                ])])
                .into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("\"test\""),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![Expression::QuotedString(
                "test".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("\"color is @{color}\""),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![
                Expression::InterpolatedString(
                    vec!["color is ".into(), "".into()],
                    vec![InterpolatedValue::Variable("color".into())]
                )
            ])])
        ))
    );
    assert_eq!(
        declaration_value("\"color is ${color}\""),
        Ok((
            "",
            Expression::CommaList(vec![Expression::SpaceList(vec![
                Expression::InterpolatedString(
                    vec!["color is ".into(), "".into()],
                    vec![InterpolatedValue::Property("color".into())]
                )
            ])])
        ))
    );
}
