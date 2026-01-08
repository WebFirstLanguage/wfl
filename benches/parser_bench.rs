use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn benchmark_parser(c: &mut Criterion) {
    let input = fs::read_to_string("examples/leak_demo.wfl").unwrap();

    c.bench_function("parse_leak_demo", |b| {
        b.iter(|| {
            // Bolt: removed normalization step
            let tokens = lex_wfl_with_positions(&input);
            let mut parser = Parser::new(&tokens);
            black_box(parser.parse().unwrap());
        })
    });
}

criterion_group!(benches, benchmark_parser);
criterion_main!(benches);
