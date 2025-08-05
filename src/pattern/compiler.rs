use super::PatternError;
use super::instruction::{CharClassType, Instruction, Program};
use crate::parser::ast::{Anchor, CharClass, PatternExpression, Quantifier};
use std::collections::HashMap;

/// Compiler that converts PatternExpression AST into bytecode
pub struct PatternCompiler {
    program: Program,
    capture_names: Vec<String>,
    capture_map: HashMap<String, usize>,
    save_counter: usize,
}

impl PatternCompiler {
    pub fn new() -> Self {
        Self {
            program: Program::new(),
            capture_names: Vec::new(),
            capture_map: HashMap::new(),
            save_counter: 0,
        }
    }

    /// Compile a PatternExpression into bytecode
    pub fn compile(&mut self, pattern: &PatternExpression) -> Result<Program, PatternError> {
        self.compile_expression(pattern)?;
        self.program.push(Instruction::Match);

        // Set metadata
        self.program.set_num_captures(self.capture_names.len());
        self.program.set_num_saves(self.save_counter);

        Ok(self.program.clone())
    }

    /// Get the list of capture group names
    pub fn capture_names(&self) -> Vec<String> {
        self.capture_names.clone()
    }

    /// Compile a single pattern expression
    fn compile_expression(&mut self, pattern: &PatternExpression) -> Result<(), PatternError> {
        match pattern {
            PatternExpression::Literal(text) => {
                self.compile_literal(text)?;
            }

            PatternExpression::CharacterClass(char_class) => {
                self.compile_char_class(char_class)?;
            }

            PatternExpression::Sequence(patterns) => {
                self.compile_sequence(patterns)?;
            }

            PatternExpression::Alternative(patterns) => {
                self.compile_alternative(patterns)?;
            }

            PatternExpression::Quantified {
                pattern,
                quantifier,
            } => {
                self.compile_quantified(pattern, quantifier)?;
            }

            PatternExpression::Capture { name, pattern } => {
                self.compile_capture(name, pattern)?;
            }

            PatternExpression::Backreference(name) => {
                self.compile_backreference(name)?;
            }

            PatternExpression::Anchor(anchor) => {
                self.compile_anchor(anchor)?;
            }

            PatternExpression::Lookahead(pattern) => {
                self.compile_lookahead(pattern)?;
            }

            PatternExpression::NegativeLookahead(pattern) => {
                self.compile_negative_lookahead(pattern)?;
            }

            PatternExpression::Lookbehind(pattern) => {
                self.compile_lookbehind(pattern)?;
            }

            PatternExpression::NegativeLookbehind(pattern) => {
                self.compile_negative_lookbehind(pattern)?;
            }
        }
        Ok(())
    }

    /// Compile a literal string
    fn compile_literal(&mut self, text: &str) -> Result<(), PatternError> {
        if text.is_empty() {
            return Ok(()); // Empty string matches trivially
        }

        if text.len() == 1 {
            // Single character - use Char instruction
            let ch = text.chars().next().unwrap();
            self.program.push(Instruction::Char(ch));
        } else {
            // Multi-character string - use Literal instruction
            self.program.push(Instruction::Literal(text.to_string()));
        }
        Ok(())
    }

    /// Compile a character class
    fn compile_char_class(&mut self, char_class: &CharClass) -> Result<(), PatternError> {
        let class_type = match char_class {
            CharClass::Digit => CharClassType::Digit,
            CharClass::Letter => CharClassType::Letter,
            CharClass::Whitespace => CharClassType::Whitespace,
        };
        self.program.push(Instruction::CharClass(class_type));
        Ok(())
    }

    /// Compile a sequence of patterns (concatenation)
    fn compile_sequence(&mut self, patterns: &[PatternExpression]) -> Result<(), PatternError> {
        for pattern in patterns {
            self.compile_expression(pattern)?;
        }
        Ok(())
    }

    /// Compile alternative patterns (alternation)
    fn compile_alternative(&mut self, patterns: &[PatternExpression]) -> Result<(), PatternError> {
        if patterns.is_empty() {
            return Err(PatternError::CompileError("Empty alternative".to_string()));
        }

        if patterns.len() == 1 {
            return self.compile_expression(&patterns[0]);
        }

        // Generate code structure:
        // split L1, L2
        // <pattern1>
        // jump END
        // L1: split L3, L4  (if more alternatives)
        // <pattern2>
        // jump END
        // L2: <pattern3>
        // END:

        let mut jump_to_end = Vec::new();
        let _split_locations: Vec<usize> = Vec::new();

        // For each alternative except the last, emit a split
        for (i, pattern) in patterns.iter().enumerate() {
            if i == patterns.len() - 1 {
                // Last alternative - just compile it
                self.compile_expression(pattern)?;
            } else {
                // Not the last - emit split and compile pattern
                let split_addr = self.program.len();
                self.program.push(Instruction::Split(0, 0)); // Will be patched

                self.compile_expression(pattern)?;

                // Jump to end after this alternative succeeds
                let jump_addr = self.program.len();
                self.program.push(Instruction::Jump(0)); // Will be patched
                jump_to_end.push(jump_addr);

                // Patch the split to point to the next alternative
                let next_alternative_addr = self.program.len();
                if let Some(Instruction::Split(first, second)) =
                    self.program.instructions.get_mut(split_addr)
                {
                    *first = split_addr + 1; // Next instruction (the pattern)
                    *second = next_alternative_addr; // Next alternative (will be filled next iteration)
                }
            }
        }

        // Patch all jump-to-end instructions
        let end_addr = self.program.len();
        for jump_addr in jump_to_end {
            if let Some(Instruction::Jump(target)) = self.program.instructions.get_mut(jump_addr) {
                *target = end_addr;
            }
        }

        Ok(())
    }

    /// Compile a quantified pattern
    fn compile_quantified(
        &mut self,
        pattern: &PatternExpression,
        quantifier: &Quantifier,
    ) -> Result<(), PatternError> {
        match quantifier {
            Quantifier::Optional => {
                // Optional: split to pattern or skip
                // split L1, L2
                // L1: <pattern>
                // L2: (continue)

                let split_addr = self.program.len();
                self.program.push(Instruction::Split(0, 0)); // Will be patched

                self.compile_expression(pattern)?;

                let end_addr = self.program.len();

                // Patch split
                if let Some(Instruction::Split(first, second)) =
                    self.program.instructions.get_mut(split_addr)
                {
                    *first = split_addr + 1; // Try the pattern
                    *second = end_addr; // Or skip it
                }
            }

            Quantifier::ZeroOrMore => {
                // Zero or more: split to pattern or skip, with loop back
                // L1: split L2, L3
                // L2: <pattern>
                //     jump L1
                // L3: (continue)

                let loop_start = self.program.len();
                self.program.push(Instruction::Split(0, 0)); // Will be patched

                self.compile_expression(pattern)?;

                // Jump back to loop start
                self.program.push(Instruction::Jump(loop_start));

                let end_addr = self.program.len();

                // Patch split
                if let Some(Instruction::Split(first, second)) =
                    self.program.instructions.get_mut(loop_start)
                {
                    *first = loop_start + 1; // Try the pattern
                    *second = end_addr; // Or exit loop
                }
            }

            Quantifier::OneOrMore => {
                // One or more: pattern, then optional loop
                // <pattern>
                // L1: split L2, L3
                // L2: <pattern>
                //     jump L1
                // L3: (continue)

                self.compile_expression(pattern)?;

                let loop_start = self.program.len();
                self.program.push(Instruction::Split(0, 0)); // Will be patched

                self.compile_expression(pattern)?;

                // Jump back to loop start
                self.program.push(Instruction::Jump(loop_start));

                let end_addr = self.program.len();

                // Patch split
                if let Some(Instruction::Split(first, second)) =
                    self.program.instructions.get_mut(loop_start)
                {
                    *first = loop_start + 1; // Try another iteration
                    *second = end_addr; // Or exit loop
                }
            }

            Quantifier::Exactly(n) => {
                // Exactly N: just repeat the pattern N times
                for _ in 0..*n {
                    self.compile_expression(pattern)?;
                }
            }

            Quantifier::Between(min, max) => {
                // Between min and max: first min required, then up to (max-min) optional

                // Required repetitions
                for _ in 0..*min {
                    self.compile_expression(pattern)?;
                }

                // Optional repetitions
                let optional_count = max - min;
                for _ in 0..optional_count {
                    let split_addr = self.program.len();
                    self.program.push(Instruction::Split(0, 0)); // Will be patched

                    self.compile_expression(pattern)?;

                    let end_addr = self.program.len();

                    // Patch split
                    if let Some(Instruction::Split(first, second)) =
                        self.program.instructions.get_mut(split_addr)
                    {
                        *first = split_addr + 1; // Try the pattern
                        *second = end_addr; // Or skip it
                    }
                }
            }
        }

        Ok(())
    }

    /// Compile a capture group
    fn compile_capture(
        &mut self,
        name: &str,
        pattern: &PatternExpression,
    ) -> Result<(), PatternError> {
        // Assign capture index
        let capture_index = if let Some(&index) = self.capture_map.get(name) {
            index
        } else {
            let index = self.capture_names.len();
            self.capture_names.push(name.to_string());
            self.capture_map.insert(name.to_string(), index);
            index
        };

        // Start capture
        self.program.push(Instruction::StartCapture(capture_index));

        // Compile the pattern
        self.compile_expression(pattern)?;

        // End capture
        self.program.push(Instruction::EndCapture(capture_index));

        Ok(())
    }

    /// Compile a backreference
    fn compile_backreference(&mut self, name: &str) -> Result<(), PatternError> {
        // Look up the capture index by name
        if let Some(&capture_index) = self.capture_map.get(name) {
            self.program.push(Instruction::Backreference(capture_index));
            Ok(())
        } else {
            Err(PatternError::CompileError(format!(
                "Backreference to undefined capture group: '{}'",
                name
            )))
        }
    }

    /// Compile an anchor
    fn compile_anchor(&mut self, anchor: &Anchor) -> Result<(), PatternError> {
        match anchor {
            Anchor::StartOfText => {
                self.program.push(Instruction::StartAnchor);
            }
            Anchor::EndOfText => {
                self.program.push(Instruction::EndAnchor);
            }
        }
        Ok(())
    }

    /// Compile a positive lookahead
    fn compile_lookahead(&mut self, pattern: &PatternExpression) -> Result<(), PatternError> {
        // For lookaheads, we need to:
        // 1. Begin lookahead (saves position)
        // 2. Compile the pattern to check
        // 3. End lookahead (restores position if pattern matched)
        self.program.push(Instruction::BeginLookahead);
        self.compile_expression(pattern)?;
        self.program.push(Instruction::EndLookahead);
        Ok(())
    }

    /// Compile a negative lookahead
    fn compile_negative_lookahead(&mut self, pattern: &PatternExpression) -> Result<(), PatternError> {
        // For negative lookaheads:
        // 1. Begin negative lookahead (saves position)
        // 2. Compile the pattern to check
        // 3. End negative lookahead (restores position if pattern didn't match)
        self.program.push(Instruction::BeginNegativeLookahead);
        self.compile_expression(pattern)?;
        self.program.push(Instruction::EndNegativeLookahead);
        Ok(())
    }

    /// Compile a positive lookbehind
    fn compile_lookbehind(&mut self, pattern: &PatternExpression) -> Result<(), PatternError> {
        // For lookbehinds, we need to calculate the fixed length of the pattern
        // This is a simplified implementation that only supports fixed-length lookbehinds
        match self.calculate_pattern_length(pattern) {
            Some(length) => {
                // Create a separate program for the lookbehind pattern
                let mut lookbehind_compiler = PatternCompiler::new();
                lookbehind_compiler.compile_expression(pattern)?;
                lookbehind_compiler.program.push(Instruction::Match);
                
                // For now, we'll use a simplified approach:
                // Store the lookbehind length and let the VM handle it
                self.program.push(Instruction::CheckLookbehind(length));
                
                // TODO: In a full implementation, we'd embed the lookbehind program
                // as data within the instruction
            }
            None => {
                return Err(PatternError::CompileError(
                    "Lookbehind patterns must have a fixed length".to_string()
                ));
            }
        }
        Ok(())
    }

    /// Compile a negative lookbehind
    fn compile_negative_lookbehind(&mut self, pattern: &PatternExpression) -> Result<(), PatternError> {
        // Similar to positive lookbehind but checks for non-match
        match self.calculate_pattern_length(pattern) {
            Some(length) => {
                self.program.push(Instruction::CheckNegativeLookbehind(length));
            }
            None => {
                return Err(PatternError::CompileError(
                    "Lookbehind patterns must have a fixed length".to_string()
                ));
            }
        }
        Ok(())
    }

    /// Calculate the fixed length of a pattern (if possible)
    fn calculate_pattern_length(&self, pattern: &PatternExpression) -> Option<usize> {
        match pattern {
            PatternExpression::Literal(s) => Some(s.chars().count()),
            PatternExpression::CharacterClass(_) => Some(1),
            PatternExpression::Sequence(patterns) => {
                let mut total = 0;
                for p in patterns {
                    total += self.calculate_pattern_length(p)?;
                }
                Some(total)
            }
            PatternExpression::Capture { pattern, .. } => self.calculate_pattern_length(pattern),
            _ => None, // Quantifiers, alternatives, etc. don't have fixed length
        }
    }

    /// Allocate a new save slot for backtracking
    fn _alloc_save_slot(&mut self) -> usize {
        let slot = self.save_counter;
        self.save_counter += 1;
        slot
    }
}

impl Default for PatternCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{CharClass, PatternExpression, Quantifier};

    #[test]
    fn test_compile_literal() {
        let mut compiler = PatternCompiler::new();
        let pattern = PatternExpression::Literal("hello".to_string());

        let program = compiler.compile(&pattern).unwrap();

        assert_eq!(program.instructions.len(), 2); // Literal + Match
        match &program.instructions[0] {
            Instruction::Literal(text) => assert_eq!(text, "hello"),
            _ => panic!("Expected Literal instruction"),
        }
        assert_eq!(program.instructions[1], Instruction::Match);
    }

    #[test]
    fn test_compile_single_char() {
        let mut compiler = PatternCompiler::new();
        let pattern = PatternExpression::Literal("a".to_string());

        let program = compiler.compile(&pattern).unwrap();

        assert_eq!(program.instructions.len(), 2); // Char + Match
        match &program.instructions[0] {
            Instruction::Char(ch) => assert_eq!(*ch, 'a'),
            _ => panic!("Expected Char instruction"),
        }
    }

    #[test]
    fn test_compile_char_class() {
        let mut compiler = PatternCompiler::new();
        let pattern = PatternExpression::CharacterClass(CharClass::Digit);

        let program = compiler.compile(&pattern).unwrap();

        assert_eq!(program.instructions.len(), 2); // CharClass + Match
        match &program.instructions[0] {
            Instruction::CharClass(CharClassType::Digit) => {}
            _ => panic!("Expected CharClass(Digit) instruction"),
        }
    }

    #[test]
    fn test_compile_sequence() {
        let mut compiler = PatternCompiler::new();
        let pattern = PatternExpression::Sequence(vec![
            PatternExpression::Literal("a".to_string()),
            PatternExpression::CharacterClass(CharClass::Digit),
            PatternExpression::Literal("b".to_string()),
        ]);

        let program = compiler.compile(&pattern).unwrap();

        assert_eq!(program.instructions.len(), 4); // Char + CharClass + Char + Match
        assert_eq!(program.instructions[0], Instruction::Char('a'));
        assert_eq!(
            program.instructions[1],
            Instruction::CharClass(CharClassType::Digit)
        );
        assert_eq!(program.instructions[2], Instruction::Char('b'));
        assert_eq!(program.instructions[3], Instruction::Match);
    }

    #[test]
    fn test_compile_optional() {
        let mut compiler = PatternCompiler::new();
        let pattern = PatternExpression::Quantified {
            pattern: Box::new(PatternExpression::Literal("a".to_string())),
            quantifier: Quantifier::Optional,
        };

        let program = compiler.compile(&pattern).unwrap();

        // Should have: Split, Char, Match
        assert_eq!(program.instructions.len(), 3);
        match &program.instructions[0] {
            Instruction::Split(first, second) => {
                assert_eq!(*first, 1); // Try the character
                assert_eq!(*second, 2); // Or skip to Match
            }
            _ => panic!("Expected Split instruction"),
        }
        assert_eq!(program.instructions[1], Instruction::Char('a'));
        assert_eq!(program.instructions[2], Instruction::Match);
    }

    #[test]
    fn test_compile_capture() {
        let mut compiler = PatternCompiler::new();
        let pattern = PatternExpression::Capture {
            name: "test".to_string(),
            pattern: Box::new(PatternExpression::Literal("hello".to_string())),
        };

        let program = compiler.compile(&pattern).unwrap();
        let capture_names = compiler.capture_names();

        assert_eq!(capture_names, vec!["test"]);
        assert_eq!(program.num_captures, 1);

        // Should have: StartCapture, Literal, EndCapture, Match
        assert_eq!(program.instructions.len(), 4);
        assert_eq!(program.instructions[0], Instruction::StartCapture(0));
        match &program.instructions[1] {
            Instruction::Literal(text) => assert_eq!(text, "hello"),
            _ => panic!("Expected Literal instruction"),
        }
        assert_eq!(program.instructions[2], Instruction::EndCapture(0));
        assert_eq!(program.instructions[3], Instruction::Match);
    }
}
