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
    let input = "hello\r\n".repeat(10000); // Has \r\n

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = normalize_line_endings_old(&input);
    }
    let duration_old = start.elapsed();
    println!("Old (with replacement): {:?}", duration_old);

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = normalize_line_endings_new(&input);
    }
    let duration_new = start.elapsed();
    println!("New (with replacement): {:?}", duration_new);
}
