use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wfl::stdlib::text::*;
use wfl::interpreter::value::Value;
use std::sync::Arc;

fn bench_percent_decode(c: &mut Criterion) {
    let inputs = vec![
        "no_encoding_here_just_plain_text",
        "some+spaces+and%20some%20percent%20encoding",
        "a=b&c=d",
        "name=John+Doe&age=30&city=New%20York",
    ];

    // We can't access `percent_decode` directly as it's private, but we can benchmark parse_query_string
    c.bench_function("native_parse_query_string", |b| {
        b.iter(|| {
            for input in &inputs {
                let _ = native_parse_query_string(vec![Value::Text(Arc::from(*input))]);
            }
        })
    });
}

criterion_group!(benches, bench_percent_decode);
criterion_main!(benches);
