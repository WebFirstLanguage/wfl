use std::time::Instant;
use std::borrow::Cow;

pub fn normalize_line_endings_old(input: &str) -> String {
    input.replace("\r\n", "\n")
}

pub fn normalize_line_endings_new(input: &str) -> Cow<str> {
    if input.contains("\r\n") {
        Cow::Owned(input.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(input)
    }
}

fn main() {
    let input = "hello\n".repeat(10000); // 60KB string, no \r\n

    // Warmup
    for _ in 0..100 {
        let _ = normalize_line_endings_old(&input);
    }

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = normalize_line_endings_old(&input);
    }
    let duration_old = start.elapsed();
    println!("Old: {:?}", duration_old);

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = normalize_line_endings_new(&input);
    }
    let duration_new = start.elapsed();
    println!("New: {:?}", duration_new);

    println!("Speedup: {:.2}x", duration_old.as_secs_f64() / duration_new.as_secs_f64());
}
