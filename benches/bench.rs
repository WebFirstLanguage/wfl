use std::time::Instant;

fn main() {
    let text = "hello world ".repeat(10);
    let old = "xyz";
    let new = "abc";

    let iters = 100_000;

    let start = Instant::now();
    for _ in 0..iters {
        let _result = text.replace(old, new);
    }
    let duration1 = start.elapsed();

    let start = Instant::now();
    for _ in 0..iters {
        if text.contains(old) {
            let _result = text.replace(old, new);
        } else {
            let _result = text.clone(); // In actual code we avoid this using `Arc::clone(&text)`
        }
    }
    let duration2 = start.elapsed();

    println!("replace always (match not found): {:?}", duration1);
    println!("contains then replace (match not found): {:?}", duration2);
}
