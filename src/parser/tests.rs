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
    println!("{:?}", selector_group(input));
}

#[test]
fn test_value() {
    let input = "0 * 0 + 0 / 0";
    println!("{:?}", declaration_value(input));
}
