use std::time::Instant;
use std::borrow::Cow;

pub fn normalize_line_endings_old(input: &str) -> String {
    input.replace("\r\n", "\n")
}

pub fn normalize_line_endings_new(input: &str) -> Cow<str> {
    // Optimization: most files do not have \r. Check for \r first.
    if !input.contains('r') {
         return Cow::Borrowed(input);
    }
    // If it has \r, it might not be \r\n, but likelihood is higher.
    // Check specific sequence
    if input.contains("\r\n") {
        Cow::Owned(input.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(input)
    }
}

fn main() {
    let input = "hello\n".repeat(10000); // Clean string

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
}
