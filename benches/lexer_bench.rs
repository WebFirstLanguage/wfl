use std::time::Instant;

fn main() {
    let input = "hello\n".repeat(10000);
    let start = Instant::now();
    let _ = wfl::lexer::normalize_line_endings(&input);
    println!("Time: {:?}", start.elapsed());
}
