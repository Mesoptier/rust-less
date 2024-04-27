extern crate less;

use std::process::Command;

use assert_json_diff::assert_json_matches;

include!(concat!(env!("OUT_DIR"), "/integration_tests_generated.rs"));

fn test_file(path: &str) {
    println!("Testing LESS file\n    at {}:1", path);

    let source = std::fs::read_to_string(path).unwrap();

    let expected = less_js_parse(path);

    let actual = less::parse(&source).unwrap().to_less_js_ast();

    let config = assert_json_diff::Config::new(assert_json_diff::CompareMode::Inclusive)
        .numeric_mode(assert_json_diff::NumericMode::AssumeFloat);

    assert_json_matches!(expected, actual, config);
}

fn less_js_parse(filename: &str) -> serde_json::Value {
    let child = Command::new("node")
        .args(&["parse-less.js", "--file", filename])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for the child process to finish
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let output = String::from_utf8(output.stdout).unwrap();

    serde_json::from_str(&output).unwrap()
}

trait ToLessJsAst {
    fn to_less_js_ast(&self) -> serde_json::Value;
}

impl ToLessJsAst for less::ast::Stylesheet<'_> {
    fn to_less_js_ast(&self) -> serde_json::Value {
        let mut rules = Vec::new();
        for rule in &self.items {
            rules.push(rule.to_less_js_ast());
        }

        serde_json::json!({
            "type": "Ruleset",
            "firstRoot": true,
            "root": true,
            "selectors": null,
            "rules": rules,
        })
    }
}

impl ToLessJsAst for less::ast::Item<'_> {
    fn to_less_js_ast(&self) -> serde_json::Value {
        use less::ast::Item;

        match self {
            Item::AtRule => todo!(),
            Item::QualifiedRule {
                selector_group,
                block,
            } => {
                let rules = block
                    .items
                    .iter()
                    .map(|item| item.to_less_js_ast())
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "type": "Ruleset",
                    "selectors": selector_group.to_less_js_ast(),
                    "rules": rules,
                })
            }
            Item::Declaration {
                name,
                value,
                important,
            } => {
                serde_json::json!({
                    "type": "Declaration",
                    "name": [
                        { "type": "Keyword", "value": name },
                    ],
                    "value": value.to_less_js_ast(),
                    "important": if *important { "!important" } else { "" },
                    "inline": false,
                    "merge": false,
                })
            }
            Item::VariableDeclaration { name, value } => {
                serde_json::json!({
                    "type": "Declaration",
                    "name": format!("@{}", name),
                    "value": value.to_less_js_ast(),
                    "important": "",
                    "inline": false,
                    "merge": false,
                    "variable": true,
                })
            }
            Item::VariableCall { .. } => todo!(),
            Item::MixinDeclaration { .. } => todo!(),
            Item::MixinCall { .. } => todo!(),
        }
    }
}

impl ToLessJsAst for less::ast::Expression<'_> {
    fn to_less_js_ast(&self) -> serde_json::Value {
        use less::ast::Expression;

        match self {
            Expression::SemicolonList(_) => todo!(),
            Expression::CommaList(exprs) => {
                let mut value = vec![];
                for expr in exprs {
                    value.push(expr.to_less_js_ast());
                }
                serde_json::json!({
                    "type": "Value",
                    "value": value,
                })
            }
            Expression::SpaceList(exprs) => {
                let mut value = vec![];
                for expr in exprs {
                    value.push(expr.to_less_js_ast());
                }
                serde_json::json!({
                    "type": "Expression",
                    "value": value,
                })
            }
            Expression::DetachedRuleset(_) => todo!(),
            Expression::UnaryOperation(_, _) => todo!(),
            Expression::BinaryOperation(_, _, _) => todo!(),
            Expression::Variable(name) => {
                serde_json::json!({
                    "type": "Variable",
                    "name": format!("@{}", name),
                })
            }
            Expression::VariableLookup(_, _) => todo!(),
            Expression::Property(_) => todo!(),
            Expression::Ident(_) => todo!(),
            Expression::Numeric(value, unit) => {
                let unit = match unit {
                    Some(unit) => serde_json::json!({
                        "type": "Unit",
                        "numerator": [unit],
                        "denominator": [],
                        "backupUnit": unit,
                    }),
                    None => serde_json::json!(null),
                };
                serde_json::json!({
                    "type": "Dimension",
                    "value": value,
                    "unit": unit,
                })
            }
            Expression::FunctionCall(_, _) => todo!(),
            Expression::QuotedString(_) => todo!(),
            Expression::InterpolatedString(_, _) => todo!(),
        }
    }
}

impl ToLessJsAst for less::ast::SelectorGroup<'_> {
    fn to_less_js_ast(&self) -> serde_json::Value {
        let mut selectors = Vec::new();
        for selector in &self.0 {
            selectors.push(selector.to_less_js_ast());
        }
        serde_json::json!(selectors)
    }
}

impl ToLessJsAst for less::ast::Selector<'_> {
    fn to_less_js_ast(&self) -> serde_json::Value {
        let mut elements = Vec::new();

        for (idx, seq) in self.0.iter().enumerate() {
            let mut value = String::new();
            for element in &seq.0 {
                value.push_str(&match element {
                    less::ast::SimpleSelector::Universal => "*".to_string(),
                    less::ast::SimpleSelector::Type(t) => t.to_string(),
                    less::ast::SimpleSelector::Id(id) => format!("#{}", id),
                    less::ast::SimpleSelector::Class(class) => format!(".{}", class),
                    less::ast::SimpleSelector::Attribute(name) => {
                        format!("[{}]", name)
                    }
                    less::ast::SimpleSelector::PseudoElement(pe) => format!("::{}", pe),
                    less::ast::SimpleSelector::PseudoClass(pc) => format!(":{}", pc),
                    less::ast::SimpleSelector::Negation(_) => todo!(),
                });
            }

            let combinator_value = if idx > 0 {
                match self.1[idx - 1] {
                    less::ast::Combinator::Descendant => " ",
                    less::ast::Combinator::Child => ">",
                    less::ast::Combinator::NextSibling => "+",
                    less::ast::Combinator::SubsequentSibling => "~",
                }
            } else {
                " "
            };

            elements.push(serde_json::json!({
                "type": "Element",
                "value": value,
                "combinator": {
                    "type": "Combinator",
                    "value": combinator_value,
                    "emptyOrWhitespace": combinator_value == " ",
                },
                "isVariable": false,
            }));
        }

        serde_json::json!({
            "type": "Selector",
            "elements": elements,
            "evaldCondition": true,
        })
    }
}

impl ToLessJsAst for less::ast::SimpleSelectorSequence<'_> {
    fn to_less_js_ast(&self) -> serde_json::Value {
        let mut string = String::new();
        for element in &self.0 {
            string.push_str(&match element {
                less::ast::SimpleSelector::Universal => "*".to_string(),
                less::ast::SimpleSelector::Type(t) => t.to_string(),
                less::ast::SimpleSelector::Id(id) => format!("#{}", id),
                less::ast::SimpleSelector::Class(class) => format!(".{}", class),
                less::ast::SimpleSelector::Attribute(name) => {
                    format!("[{}]", name)
                }
                less::ast::SimpleSelector::PseudoElement(pe) => format!("::{}", pe),
                less::ast::SimpleSelector::PseudoClass(pc) => format!(":{}", pc),
                less::ast::SimpleSelector::Negation(_) => todo!(),
            });
        }

        serde_json::json!(string)
    }
}
