use crate::parser::selector::selector_group;
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
fn test_selector() {
    let input = "body.class#id:pseudo:not(.not)::pseudo-elem > test + test test~test, a";

    assert_eq!(
        selector_group(input),
        Ok((
            "",
            SelectorGroup(vec![
                Selector(
                    vec![
                        SimpleSelectorSequence(vec![
                            SimpleSelector::Type("body".into()),
                            SimpleSelector::Class("class".into()),
                            SimpleSelector::Id("id".into()),
                            SimpleSelector::PseudoClass("pseudo".into()),
                            SimpleSelector::Negation(SimpleSelector::Class("not".into()).into()),
                            SimpleSelector::PseudoElement("pseudo-elem".into()),
                        ]),
                        SimpleSelectorSequence(vec![SimpleSelector::Type("test".into())]),
                        SimpleSelectorSequence(vec![SimpleSelector::Type("test".into())]),
                        SimpleSelectorSequence(vec![SimpleSelector::Type("test".into())]),
                        SimpleSelectorSequence(vec![SimpleSelector::Type("test".into())]),
                    ],
                    vec![
                        Combinator::Child,
                        Combinator::NextSibling,
                        Combinator::Descendant,
                        Combinator::SubsequentSibling
                    ]
                ),
                Selector(
                    vec![SimpleSelectorSequence(vec![SimpleSelector::Type(
                        "a".into()
                    )])],
                    vec![]
                )
            ])
        ))
    );
}

#[test]
fn test_value() {
    let input = "0 * 0 + 0 / 0";
    println!("{:?}", declaration_value(input));
}
