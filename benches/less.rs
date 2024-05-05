use chumsky::input::Input;
use chumsky::prelude::SimpleSpan;
use chumsky::Parser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use less::{lexer, parser};

pub fn criterion_benchmark(c: &mut Criterion) {
    let file =
        std::fs::read_to_string("node_modules/@less/test-data/less/_main/variables.less").unwrap();
    c.bench_function("lexer", |b| {
        b.iter(|| lexer().parse(black_box(file.as_str())))
    });

    let tts = lexer().parse(file.as_str()).unwrap();
    let parser_input = tts.as_slice().spanned(SimpleSpan::splat(tts.len()));
    c.bench_function("parser", |b| {
        b.iter(|| parser().parse(black_box(parser_input.clone())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
