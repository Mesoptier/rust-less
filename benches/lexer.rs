use chumsky::Parser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use less::lexer;

pub fn criterion_benchmark(c: &mut Criterion) {
    let file =
        std::fs::read_to_string("node_modules/@less/test-data/less/_main/variables.less").unwrap();
    c.bench_function("lexer", |b| {
        b.iter(|| lexer().parse(black_box(file.as_str())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
