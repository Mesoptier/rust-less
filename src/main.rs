use chumsky::Parser;
use less::lexer;

fn main() {
    let file =
        std::fs::read_to_string("node_modules/@less/test-data/less/_main/variables.less").unwrap();
    println!("{:#?}", lexer().parse(file.as_str()).into_result());
}
