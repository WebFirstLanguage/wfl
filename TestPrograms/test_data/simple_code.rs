// Simple Rust file with only code lines
fn main() {
    let x = 42;
    let y = x + 1;
    println!("Hello, world!");
    let result = calculate(x, y);
    println!("Result: {}", result);
}

fn calculate(a: i32, b: i32) -> i32 {
    a * b + 10
}

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
    
    fn distance(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
