use crate::parser::value::declaration_value;

use super::*;

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

.box when (#lib.colors[@primary] = blue) {
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
                    guard: Some(Guard),
                    items: vec![]
                },
            },
        ))
    );
}

#[test]
fn test_qualified_rule() {
    assert_eq!(
        qualified_rule("a { color: blue; }"),
        Ok((
            "",
            Item::QualifiedRule {
                selector_group: SelectorGroup(vec![Selector(
                    vec![SimpleSelectorSequence(vec![SimpleSelector::Type(
                        "a".into()
                    )])],
                    vec![]
                )]),
                block: GuardedBlock {
                    guard: None,
                    items: vec![Item::Declaration {
                        name: "color".into(),
                        value: Value::CommaList(vec![Value::SpaceList(vec![Value::Ident(
                            "blue".into()
                        )])]),
                        important: false,
                    }]
                }
            }
        ))
    )
}

#[test]
fn test_declaration() {
    assert_eq!(
        declaration("color: blue;"),
        Ok((
            "",
            Item::Declaration {
                name: "color".into(),
                value: Value::CommaList(vec![Value::SpaceList(vec![Value::Ident("blue".into())])]),
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
            Value::CommaList(vec![Value::SpaceList(vec![Value::Operation(
                Operation::Add,
                Value::Operation(
                    Operation::Multiply,
                    Value::Numeric(0.0, None).into(),
                    Value::Numeric(0.0, None).into(),
                )
                .into(),
                Value::Operation(
                    Operation::Divide,
                    Value::Numeric(0.0, None).into(),
                    Value::Numeric(0.0, None).into(),
                )
                .into(),
            )])])
        ))
    );
    assert_eq!(
        declaration_value("blue"),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::Ident("blue".into())])])
        ))
    );
    assert_eq!(
        declaration_value("5px solid red"),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![
                Value::Numeric(5.0, Some("px".into())),
                Value::Ident("solid".into()),
                Value::Ident("red".into()),
            ])])
        ))
    );
    assert_eq!(
        declaration_value("@primary"),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::Variable(
                "primary".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("@colors[primary]"),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::VariableLookup(
                "colors".into(),
                vec![Lookup::Ident("primary".into())]
            )])])
        ))
    );
    assert_eq!(
        declaration_value("$color"),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::Property(
                "color".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("rgba(0, 0, 0, 0.5)"),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::FunctionCall(
                "rgba".into(),
                Value::SemicolonList(vec![Value::CommaList(vec![
                    Value::SpaceList(vec![Value::Numeric(0.0, None)]),
                    Value::SpaceList(vec![Value::Numeric(0.0, None)]),
                    Value::SpaceList(vec![Value::Numeric(0.0, None)]),
                    Value::SpaceList(vec![Value::Numeric(0.5, None)]),
                ])])
                .into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("\"test\""),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::QuotedString(
                "test".into()
            )])])
        ))
    );
    assert_eq!(
        declaration_value("\"color is @{color}\""),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::InterpolatedString(
                vec!["color is ".into(), "".into()],
                vec![InterpolatedValue::Variable("color".into())]
            )])])
        ))
    );
    assert_eq!(
        declaration_value("\"color is ${color}\""),
        Ok((
            "",
            Value::CommaList(vec![Value::SpaceList(vec![Value::InterpolatedString(
                vec!["color is ".into(), "".into()],
                vec![InterpolatedValue::Property("color".into())]
            )])])
        ))
    );
}
