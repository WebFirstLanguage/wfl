use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::sync::Arc;
use wfl::interpreter::value::Value;
use wfl::stdlib::text::native_parse_query_string;

fn benchmark_parse_query_string(c: &mut Criterion) {
    let input = Value::Text(Arc::from(
        "?page=1&limit=10&search=rust&sort=desc&filter=active&type=article&author=bolt&tag=performance&status=published&category=tech",
    ));

    c.bench_function("parse_query_string", |b| {
        b.iter(|| {
            black_box(native_parse_query_string(vec![input.clone()]).unwrap());
        })
    });
}

criterion_group!(benches, benchmark_parse_query_string);
criterion_main!(benches);
