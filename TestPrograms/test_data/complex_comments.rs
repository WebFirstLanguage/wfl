// Complex comment scenarios for testing edge cases

fn main() {
    /* Block comment */ let x = 5; // Code after block comment
    
    /*
     * Multi-line block comment
     * with asterisks
     */ 
    let y = 10;

    /* Block comment with code after */ println!("Hello");
    
    // Comment with /* fake block comment markers */
    let z = x + y;
    
    /*
    Unclosed block comment spanning multiple lines
    This should be handled gracefully
    More lines in the block comment
    */
    
    println!("Result: {}", z);
}

/* Another block comment */
fn another_function() {
    // Regular comment
    let a = 1;
    /* Inline block */ let b = 2; /* Another inline */ 
    println!("{} + {} = {}", a, b, a + b);
}
