use chumsky::input::Input;
use chumsky::prelude::SimpleSpan;
use chumsky::Parser;

use less::{lexer, parser};

fn main() {
    let file =
        std::fs::read_to_string("node_modules/@less/test-data/less/_main/variables.less").unwrap();
    let tts = lexer().parse(file.as_str()).unwrap();
    let parser_input = tts.as_slice().spanned(SimpleSpan::splat(tts.len()));
    let result = parser().parse(parser_input).into_result();
    println!("{:#?}", result);
}
