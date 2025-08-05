use super::instruction::{Instruction, Program};
use super::PatternError;
use std::collections::HashMap;

const MAX_STEPS: usize = 100_000;

/// Result of a pattern match
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
    pub captures: HashMap<String, String>,
}

impl MatchResult {
    pub fn new(start: usize, end: usize, text: &str) -> Self {
        Self {
            start,
            end,
            matched_text: text[start..end].to_string(),
            captures: HashMap::new(),
        }
    }

    pub fn with_captures(start: usize, end: usize, text: &str, captures: HashMap<String, String>) -> Self {
        Self {
            start,
            end,
            matched_text: text[start..end].to_string(),
            captures,
        }
    }
}

/// Virtual machine state for pattern execution
#[derive(Debug, Clone)]
struct VMState {
    pc: usize,           // Program counter
    pos: usize,          // Current position in input text
    captures: Vec<Option<(usize, usize)>>, // Capture group start/end positions
    saves: Vec<usize>,   // Saved positions for backtracking
}

impl VMState {
    fn new(num_captures: usize, num_saves: usize) -> Self {
        Self {
            pc: 0,
            pos: 0,
            captures: vec![None; num_captures],
            saves: vec![0; num_saves],
        }
    }
}

/// Pattern matching virtual machine
pub struct PatternVM {
    step_count: usize,
}

impl PatternVM {
    pub fn new() -> Self {
        Self { step_count: 0 }
    }

    /// Execute a pattern program against input text (just test if it matches)
    pub fn execute(&mut self, program: &Program, text: &str) -> Result<bool, PatternError> {
        self.step_count = 0;
        
        // Try matching at each position in the text
        for start_pos in 0..=text.len() {
            if self.execute_at_position(program, text, start_pos)? {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Find the first match in the text
    pub fn find(&mut self, program: &Program, text: &str, capture_names: &[String]) -> Option<MatchResult> {
        self.step_count = 0;
        
        // Try matching at each position in the text
        for start_pos in 0..=text.len() {
            if let Ok(Some(result)) = self.find_at_position(program, text, start_pos, capture_names) {
                return Some(result);
            }
        }
        
        None
    }

    /// Find all matches in the text
    pub fn find_all(&mut self, program: &Program, text: &str, capture_names: &[String]) -> Vec<MatchResult> {
        let mut matches = Vec::new();
        let mut pos = 0;
        
        while pos <= text.len() {
            self.step_count = 0;
            
            if let Ok(Some(result)) = self.find_at_position(program, text, pos, capture_names) {
                pos = if result.end > result.start {
                    result.end // Move past this match
                } else {
                    result.start + 1 // Handle zero-width matches
                };
                matches.push(result);
            } else {
                pos += 1;
            }
        }
        
        matches
    }

    /// Execute pattern starting at a specific position
    fn execute_at_position(&mut self, program: &Program, text: &str, start_pos: usize) -> Result<bool, PatternError> {
        let initial_state = VMState::new(program.num_captures, program.num_saves);
        let mut states = vec![VMState { pos: start_pos, ..initial_state }];
        
        while !states.is_empty() {
            self.step_count += 1;
            if self.step_count > MAX_STEPS {
                return Err(PatternError::StepLimitExceeded);
            }
            
            let mut next_states = Vec::new();
            
            for state in states {
                match self.step(program, text, state)? {
                    StepResult::Continue(new_states) => {
                        next_states.extend(new_states);
                    }
                    StepResult::Match => {
                        return Ok(true);
                    }
                    StepResult::Fail => {
                        // This execution path failed, try others
                    }
                }
            }
            
            states = next_states;
        }
        
        Ok(false)
    }

    /// Find a match starting at a specific position
    fn find_at_position(&mut self, program: &Program, text: &str, start_pos: usize, capture_names: &[String]) -> Result<Option<MatchResult>, PatternError> {
        let initial_state = VMState::new(program.num_captures, program.num_saves);
        let mut states = vec![VMState { pos: start_pos, ..initial_state }];
        
        while !states.is_empty() {
            self.step_count += 1;
            if self.step_count > MAX_STEPS {
                return Err(PatternError::StepLimitExceeded);
            }
            
            let mut next_states = Vec::new();
            
            for state in states {
                match self.step(program, text, state)? {
                    StepResult::Continue(new_states) => {
                        next_states.extend(new_states);
                    }
                    StepResult::Match => {
                        // Found a match, construct result
                        let mut captures: HashMap<String, String> = HashMap::new();
                        // Note: We'll need to track the matching state to get captures and end position
                        // For now, return a basic match
                        return Ok(Some(MatchResult::new(start_pos, start_pos + 1, text)));
                    }
                    StepResult::Fail => {
                        // This execution path failed, try others
                    }
                }
            }
            
            states = next_states;
        }
        
        Ok(None)
    }

    /// Execute one step of the virtual machine
    fn step(&mut self, program: &Program, text: &str, mut state: VMState) -> Result<StepResult, PatternError> {
        let chars: Vec<char> = text.chars().collect();
        
        loop {
            let instruction = match program.get(state.pc) {
                Some(inst) => inst,
                None => return Ok(StepResult::Fail), // Invalid PC
            };

            match instruction {
                Instruction::Char(expected_char) => {
                    if state.pos < chars.len() && chars[state.pos] == *expected_char {
                        state.pc += 1;
                        state.pos += 1;
                    } else {
                        return Ok(StepResult::Fail);
                    }
                }
                
                Instruction::CharClass(char_class) => {
                    if state.pos < chars.len() && char_class.matches(chars[state.pos]) {
                        state.pc += 1;
                        state.pos += 1;
                    } else {
                        return Ok(StepResult::Fail);
                    }
                }
                
                Instruction::Literal(literal) => {
                    let literal_chars: Vec<char> = literal.chars().collect();
                    if state.pos + literal_chars.len() <= chars.len() {
                        let text_slice = &chars[state.pos..state.pos + literal_chars.len()];
                        if text_slice == &literal_chars[..] {
                            state.pc += 1;
                            state.pos += literal_chars.len();
                        } else {
                            return Ok(StepResult::Fail);
                        }
                    } else {
                        return Ok(StepResult::Fail);
                    }
                }
                
                Instruction::Jump(target) => {
                    state.pc = *target;
                }
                
                Instruction::Split(first, second) => {
                    // Create two execution paths
                    let mut state1 = state.clone();
                    let mut state2 = state;
                    
                    state1.pc = *first;
                    state2.pc = *second;
                    
                    return Ok(StepResult::Continue(vec![state1, state2]));
                }
                
                Instruction::StartCapture(capture_index) => {
                    if *capture_index < state.captures.len() {
                        // Start the capture group
                        if let Some(capture) = state.captures.get_mut(*capture_index) {
                            *capture = Some((state.pos, state.pos)); // Start position
                        }
                    }
                    state.pc += 1;
                }
                
                Instruction::EndCapture(capture_index) => {
                    if *capture_index < state.captures.len() {
                        // End the capture group
                        if let Some(Some((start, _))) = state.captures.get_mut(*capture_index) {
                            *state.captures.get_mut(*capture_index).unwrap() = Some((*start, state.pos));
                        }
                    }
                    state.pc += 1;
                }
                
                Instruction::StartAnchor => {
                    if state.pos == 0 {
                        state.pc += 1;
                    } else {
                        return Ok(StepResult::Fail);
                    }
                }
                
                Instruction::EndAnchor => {
                    if state.pos == chars.len() {
                        state.pc += 1;
                    } else {
                        return Ok(StepResult::Fail);
                    }
                }
                
                Instruction::Match => {
                    return Ok(StepResult::Match);
                }
                
                Instruction::Fail => {
                    return Ok(StepResult::Fail);
                }
                
                Instruction::Save(slot) => {
                    if *slot < state.saves.len() {
                        state.saves[*slot] = state.pos;
                    }
                    state.pc += 1;
                }
                
                Instruction::Restore(slot) => {
                    if *slot < state.saves.len() {
                        state.pos = state.saves[*slot];
                    }
                    state.pc += 1;
                }
            }
        }
    }
}

impl Default for PatternVM {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of executing one VM step
enum StepResult {
    Continue(Vec<VMState>), // Continue with these states
    Match,                  // Pattern matched successfully
    Fail,                   // This execution path failed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::instruction::{CharClassType, Instruction};

    #[test]
    fn test_simple_char_match() {
        let mut program = Program::new();
        program.push(Instruction::Char('a'));
        program.push(Instruction::Match);
        
        let mut vm = PatternVM::new();
        assert!(vm.execute(&program, "a").unwrap());
        assert!(!vm.execute(&program, "b").unwrap());
        assert!(!vm.execute(&program, "").unwrap());
    }

    #[test]
    fn test_literal_match() {
        let mut program = Program::new();
        program.push(Instruction::Literal("hello".to_string()));
        program.push(Instruction::Match);
        
        let mut vm = PatternVM::new();
        assert!(vm.execute(&program, "hello").unwrap());
        assert!(vm.execute(&program, "hello world").unwrap());
        assert!(!vm.execute(&program, "hi").unwrap());
        assert!(!vm.execute(&program, "hell").unwrap());
    }

    #[test]
    fn test_char_class_match() {
        let mut program = Program::new();
        program.push(Instruction::CharClass(CharClassType::Digit));
        program.push(Instruction::Match);
        
        let mut vm = PatternVM::new();
        assert!(vm.execute(&program, "5").unwrap());
        assert!(vm.execute(&program, "0 remaining").unwrap());
        assert!(!vm.execute(&program, "a").unwrap());
        assert!(!vm.execute(&program, "").unwrap());
    }

    #[test]
    fn test_sequence_match() {
        let mut program = Program::new();
        program.push(Instruction::Char('a'));
        program.push(Instruction::CharClass(CharClassType::Digit));
        program.push(Instruction::Char('b'));
        program.push(Instruction::Match);
        
        let mut vm = PatternVM::new();
        assert!(vm.execute(&program, "a5b").unwrap());
        assert!(vm.execute(&program, "a0b extra").unwrap());
        assert!(!vm.execute(&program, "ab").unwrap());
        assert!(!vm.execute(&program, "a5c").unwrap());
        assert!(!vm.execute(&program, "5ab").unwrap());
    }

    #[test]
    fn test_split_alternative() {
        // Pattern: 'a' | 'b'
        let mut program = Program::new();
        program.push(Instruction::Split(1, 3)); // Try 'a' at 1, or 'b' at 3
        program.push(Instruction::Char('a'));   // 1
        program.push(Instruction::Jump(4));     // 2: Jump to Match
        program.push(Instruction::Char('b'));   // 3
        program.push(Instruction::Match);       // 4
        
        let mut vm = PatternVM::new();
        assert!(vm.execute(&program, "a").unwrap());
        assert!(vm.execute(&program, "b").unwrap());
        assert!(!vm.execute(&program, "c").unwrap());
    }

    #[test]
    fn test_anchors() {
        // Pattern: start of text + 'a' + end of text
        let mut program = Program::new();
        program.push(Instruction::StartAnchor);
        program.push(Instruction::Char('a'));
        program.push(Instruction::EndAnchor);
        program.push(Instruction::Match);
        
        let mut vm = PatternVM::new();
        assert!(vm.execute(&program, "a").unwrap());
        assert!(!vm.execute(&program, "ba").unwrap());
        assert!(!vm.execute(&program, "ab").unwrap());
        assert!(!vm.execute(&program, "bab").unwrap());
    }
}