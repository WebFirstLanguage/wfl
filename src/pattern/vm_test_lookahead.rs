#[cfg(test)]
mod lookahead_tests {
    use crate::pattern::{PatternExpression, Compiler, PatternVM};
    use crate::parser::ast::CharClass;

    #[test]
    fn test_positive_lookahead() {
        // Test pattern: digit check ahead for {letter}
        let pattern = PatternExpression::Concatenation(vec![
            PatternExpression::CharClass(CharClass::Digit),
            PatternExpression::Lookahead(Box::new(
                PatternExpression::CharClass(CharClass::Letter)
            ))
        ]);

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&pattern).unwrap();
        
        println!("Bytecode instructions:");
        for (i, instr) in compiled.program.instructions.iter().enumerate() {
            println!("{}: {:?}", i, instr);
        }

        let mut vm = PatternVM::new();
        
        // Should match "5a" (digit followed by letter)
        assert!(vm.execute(&compiled.program, "5a").unwrap());
        
        // Should NOT match "59" (digit not followed by letter)
        assert!(!vm.execute(&compiled.program, "59").unwrap());
    }
}