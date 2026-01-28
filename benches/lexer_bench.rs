use criterion::{Criterion, black_box, criterion_group, criterion_main};
use wfl::lexer::lex_wfl_with_positions;

fn benchmark_lexer_strings(c: &mut Criterion) {
    // Generate a large input with many string literals
    // Each line: store varN as "string literal number N"
    let mut input = String::with_capacity(1024 * 1024);
    for i in 0..5000 {
        input.push_str("store var");
        input.push_str(&i.to_string());
        input.push_str(" as \"string literal number ");
        input.push_str(&i.to_string());
        input.push_str(" which is long enough to matter\"\n");
    }

    c.bench_function("lex_large_strings", |b| {
        b.iter(|| {
            black_box(lex_wfl_with_positions(&input));
        })
    });
}

fn benchmark_lexer_no_strings(c: &mut Criterion) {
    // Generate input without string literals to serve as a baseline/control
    let mut input = String::with_capacity(1024 * 1024);
    for i in 0..5000 {
        input.push_str("store var");
        input.push_str(&i.to_string());
        input.push_str(" as ");
        input.push_str(&i.to_string());
        input.push('\n');
    }

    c.bench_function("lex_large_no_strings", |b| {
        b.iter(|| {
            black_box(lex_wfl_with_positions(&input));
        })
    });
}

fn benchmark_lexer_booleans(c: &mut Criterion) {
    // Generate input with many boolean literals
    let mut input = String::with_capacity(1024 * 1024);
    for _ in 0..10000 {
        input.push_str("true false yes no ");
    }

    c.bench_function("benchmark_lexer_booleans", |b| {
        b.iter(|| {
            black_box(lex_wfl_with_positions(&input));
        })
    });
}

criterion_group!(benches, benchmark_lexer_strings, benchmark_lexer_no_strings, benchmark_lexer_booleans);
criterion_main!(benches);
