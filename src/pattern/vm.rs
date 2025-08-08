//! Pattern Virtual Machine - Executes Pattern Bytecode
//!
//! The pattern VM is a stack-based virtual machine that executes compiled
//! pattern bytecode. It provides efficient pattern matching with support
//! for advanced features like backtracking, lookaround assertions, and
//! capture groups.

use super::PatternError;
use super::instruction::{Instruction, Program};
use std::collections::HashMap;

/// Maximum number of execution steps to prevent ReDoS (Regular Expression Denial of Service) attacks.
/// This limit ensures that malicious or poorly designed patterns cannot cause infinite loops.
const MAX_STEPS: usize = 100_000;

/// Result of a pattern match operation.
///
/// Contains the position of the match, the matched text, and any captured groups.
/// All positions are character indices, not byte indices, for proper Unicode support.
///
/// # Fields
/// * `start` - Character index where the match begins (inclusive)
/// * `end` - Character index where the match ends (exclusive)  
/// * `matched_text` - The actual text that was matched
/// * `captures` - Named capture groups and their matched content
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Start position of the match (character index)
    pub start: usize,
    /// End position of the match (character index)
    pub end: usize,
    /// The text that was matched
    pub matched_text: String,
    /// Named capture groups and their values
    pub captures: HashMap<String, String>,
}

impl MatchResult {
    /// Create a new match result without capture groups.
    ///
    /// # Arguments
    /// * `start` - Starting character index of the match
    /// * `end` - Ending character index of the match (exclusive)
    /// * `text` - The full input text being matched
    ///
    /// # Returns
    /// A new MatchResult with empty captures
    pub fn new(start: usize, end: usize, text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let matched_text = if start <= end && end <= chars.len() {
            chars[start..end].iter().collect()
        } else {
            String::new()
        };
        Self {
            start,
            end,
            matched_text,
            captures: HashMap::new(),
        }
    }

    /// Create a new match result with capture groups.
    ///
    /// # Arguments
    /// * `start` - Starting character index of the match
    /// * `end` - Ending character index of the match (exclusive)
    /// * `text` - The full input text being matched
    /// * `captures` - Named capture groups and their matched values
    ///
    /// # Returns
    /// A new MatchResult with the provided captures
    pub fn with_captures(
        start: usize,
        end: usize,
        text: &str,
        captures: HashMap<String, String>,
    ) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let matched_text = if start <= end && end <= chars.len() {
            chars[start..end].iter().collect()
        } else {
            String::new()
        };
        Self {
            start,
            end,
            matched_text,
            captures,
        }
    }
}

/// Virtual machine state for pattern execution
#[derive(Debug, Clone)]
struct VMState {
    pc: usize,                             // Program counter
    pos: usize,                            // Current position in input text
    captures: Vec<Option<(usize, usize)>>, // Capture group start/end positions
    saves: Vec<usize>,                     // Saved positions for backtracking
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

/// Pattern matching virtual machine.
///
/// The VM executes compiled pattern bytecode using a stack-based approach
/// with support for backtracking, captures, and advanced pattern features.
///
/// ## Security Features
/// * **Step Limiting**: Prevents ReDoS attacks by limiting execution steps
/// * **Memory Safety**: All operations are bounds-checked
/// * **Safe Backtracking**: Controlled state management prevents infinite loops
///
/// ## Performance Features
/// * **Bytecode Execution**: Efficient interpretation of compiled patterns
/// * **Character-based**: Works with Unicode character indices
/// * **Optimized Backtracking**: Minimal state saves for performance
///
/// ## Thread Safety
/// Each VM instance maintains its own execution state, making it safe to use
/// different VM instances concurrently. However, a single VM instance should
/// not be used from multiple threads simultaneously.
pub struct PatternVM {
    /// Count of execution steps to prevent infinite loops
    step_count: usize,
    /// Debug flag for test mode (only available in test builds)
    #[cfg(test)]
    debug: bool,
}

impl PatternVM {
    pub fn new() -> Self {
        Self {
            step_count: 0,
            #[cfg(test)]
            debug: false,
        }
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
    pub fn find(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Option<MatchResult> {
        self.step_count = 0;

        // Try matching at each position in the text
        for start_pos in 0..=text.len() {
            if let Ok(Some(result)) = self.find_at_position(program, text, start_pos, capture_names)
            {
                return Some(result);
            }
        }

        None
    }

    /// Find all matches in the text
    pub fn find_all(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Vec<MatchResult> {
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
    fn execute_at_position(
        &mut self,
        program: &Program,
        text: &str,
        start_pos: usize,
    ) -> Result<bool, PatternError> {
        let initial_state = VMState::new(program.num_captures, program.num_saves);
        let mut states = vec![VMState {
            pos: start_pos,
            ..initial_state
        }];

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
                    StepResult::Match(_) => {
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
    fn find_at_position(
        &mut self,
        program: &Program,
        text: &str,
        start_pos: usize,
        capture_names: &[String],
    ) -> Result<Option<MatchResult>, PatternError> {
        let initial_state = VMState::new(program.num_captures, program.num_saves);
        let mut states = vec![VMState {
            pos: start_pos,
            ..initial_state
        }];

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
                    StepResult::Match(final_state) => {
                        // Found a match, construct result with captures
                        let mut captures: HashMap<String, String> = HashMap::new();

                        // Extract captures from the final state
                        let text_chars: Vec<char> = text.chars().collect();
                        for (i, name) in capture_names.iter().enumerate() {
                            if let Some((start, end)) = final_state.captures[i] {
                                let captured_text: String =
                                    if start <= end && end <= text_chars.len() {
                                        text_chars[start..end].iter().collect()
                                    } else {
                                        String::new()
                                    };
                                captures.insert(name.clone(), captured_text);
                            }
                        }

                        return Ok(Some(MatchResult::with_captures(
                            start_pos,
                            final_state.pos,
                            text,
                            captures,
                        )));
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
    #[allow(clippy::only_used_in_recursion)]
    fn step(
        &mut self,
        program: &Program,
        text: &str,
        mut state: VMState,
    ) -> Result<StepResult, PatternError> {
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
                        #[cfg(test)]
                        if self.debug {
                            if state.pos >= chars.len() {
                                println!("  CharClass {char_class:?} failed - end of string");
                            } else {
                                let ch = chars[state.pos];
                                println!(
                                    "  CharClass {char_class:?} failed - char '{ch}' doesn't match"
                                );
                            }
                        }
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
                            *state.captures.get_mut(*capture_index).unwrap() =
                                Some((*start, state.pos));
                        }
                    }
                    state.pc += 1;
                }

                Instruction::Backreference(capture_index) => {
                    // Match against a previously captured group
                    if *capture_index < state.captures.len() {
                        if let Some((start, end)) = state.captures[*capture_index] {
                            // Get the captured text
                            let captured_len = end - start;

                            // Check if we have enough characters left
                            if state.pos + captured_len <= chars.len() {
                                // Check if the text at current position matches the captured text
                                let captured_text = &chars[start..end];
                                let current_text = &chars[state.pos..state.pos + captured_len];

                                if captured_text == current_text {
                                    state.pc += 1;
                                    state.pos += captured_len;
                                } else {
                                    return Ok(StepResult::Fail);
                                }
                            } else {
                                return Ok(StepResult::Fail);
                            }
                        } else {
                            // Capture group hasn't been matched yet
                            return Ok(StepResult::Fail);
                        }
                    } else {
                        // Invalid capture index
                        return Ok(StepResult::Fail);
                    }
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
                    return Ok(StepResult::Match(state));
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

                Instruction::BeginLookahead => {
                    // Save the current position
                    let _saved_pos = state.pos;

                    #[cfg(test)]
                    if self.debug {
                        println!("  BeginLookahead at pos {_saved_pos}");
                    }

                    // Find the matching EndLookahead
                    let mut end_pc = state.pc + 1;
                    let mut depth = 1;
                    while depth > 0 && end_pc < program.instructions.len() {
                        match &program.instructions[end_pc] {
                            Instruction::BeginLookahead => depth += 1,
                            Instruction::EndLookahead => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            end_pc += 1;
                        }
                    }

                    // Create a sub-program for the lookahead pattern
                    let mut lookahead_program = Program::new();
                    for i in (state.pc + 1)..end_pc {
                        lookahead_program.push(program.instructions[i].clone());
                    }
                    lookahead_program.push(Instruction::Match);

                    #[cfg(test)]
                    if self.debug {
                        println!(
                            "  Lookahead sub-program: {:?}",
                            lookahead_program.instructions
                        );
                    }

                    // Try to match the lookahead pattern at the current position
                    let mut lookahead_vm = PatternVM::new();
                    #[cfg(test)]
                    {
                        lookahead_vm.debug = self.debug;
                    }

                    let lookahead_matched =
                        lookahead_vm.execute_at_position(&lookahead_program, text, state.pos)?;

                    if lookahead_matched {
                        #[cfg(test)]
                        if self.debug {
                            println!("  Lookahead pattern matched!");
                        }
                        // Pattern matched, but don't consume any input
                        state.pc = end_pc + 1; // Skip past EndLookahead
                    } else {
                        #[cfg(test)]
                        if self.debug {
                            println!("  Lookahead pattern failed");
                        }
                        return Ok(StepResult::Fail);
                    }
                }

                Instruction::EndLookahead => {
                    // This should only be reached by the lookahead logic above
                    state.pc += 1;
                }

                Instruction::BeginNegativeLookahead => {
                    // Save the current position
                    let saved_pos = state.pos;
                    state.pc += 1;

                    // Try to match the lookahead pattern
                    let lookahead_state = state.clone();

                    // Execute until we hit EndNegativeLookahead or fail
                    let mut depth = 1;
                    let mut current_states = vec![lookahead_state];
                    let mut any_matched = false;

                    'outer: while depth > 0 && !current_states.is_empty() {
                        let mut next_states = Vec::new();

                        for lookahead_state in current_states.drain(..) {
                            if lookahead_state.pc >= program.instructions.len() {
                                continue;
                            }

                            match &program.instructions[lookahead_state.pc] {
                                Instruction::BeginNegativeLookahead => depth += 1,
                                Instruction::EndNegativeLookahead => {
                                    depth -= 1;
                                    if depth == 0 {
                                        // We reached the end without matching - success!
                                        any_matched = true;
                                        state.pos = saved_pos;
                                        state.pc = lookahead_state.pc + 1;
                                        break 'outer;
                                    }
                                }
                                _ => {}
                            }

                            match self.step(program, text, lookahead_state)? {
                                StepResult::Fail => {
                                    // Good - this path failed
                                }
                                StepResult::Continue(states) => {
                                    next_states.extend(states);
                                }
                                StepResult::Match(_) => {
                                    // Pattern matched inside negative lookahead - fail
                                    return Ok(StepResult::Fail);
                                }
                            }
                        }

                        current_states = next_states;
                    }

                    if !any_matched && current_states.is_empty() {
                        // All paths failed - which is what we want for negative lookahead
                        // Skip to after EndNegativeLookahead
                        let mut skip_depth = 1;
                        while skip_depth > 0 && state.pc < program.instructions.len() {
                            #[cfg(test)]
                            if std::env::var("VM_DEBUG").is_ok() {
                                let inst = &program.instructions[state.pc];
                                println!(
                                    "PC: {pc}, Pos: {pos}, Inst: {inst:?}",
                                    pc = state.pc,
                                    pos = state.pos
                                );
                            }

                            match &program.instructions[state.pc] {
                                Instruction::BeginNegativeLookahead => skip_depth += 1,
                                Instruction::EndNegativeLookahead => {
                                    skip_depth -= 1;
                                    if skip_depth == 0 {
                                        state.pc += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            state.pc += 1;
                        }
                        state.pos = saved_pos;
                    } else if !any_matched {
                        // Pattern could still match
                        return Ok(StepResult::Fail);
                    }
                }

                Instruction::EndNegativeLookahead => {
                    // This should only be reached by the negative lookahead logic above
                    state.pc += 1;
                }

                Instruction::CheckLookbehind(lookbehind_program) => {
                    // Execute the lookbehind pattern against text before current position
                    // We need to find where the pattern should start matching

                    // Try matching at different positions before current position
                    let mut matched = false;
                    let text_chars: Vec<char> = text.chars().collect();

                    // Get the text before current position
                    if state.pos > 0 {
                        // Try to match the pattern ending at current position
                        // We'll try different starting positions
                        let max_lookback = state.pos.min(1000); // Limit lookback distance

                        for start_offset in 1..=max_lookback {
                            let start_pos = state.pos - start_offset;

                            // Create a new VM to execute the lookbehind pattern
                            let mut lookbehind_vm = PatternVM::new();

                            // Create a slice of text to match against
                            let text_slice: String =
                                text_chars[start_pos..state.pos].iter().collect();

                            // Try to match the entire slice
                            if let Ok(result) =
                                lookbehind_vm.execute(lookbehind_program, &text_slice)
                                && result
                            {
                                // Check if the match uses the entire slice
                                let matches =
                                    lookbehind_vm.find_all(lookbehind_program, &text_slice, &[]);
                                if let Some(first_match) = matches.first()
                                    && first_match.start == 0
                                    && first_match.end == text_slice.len()
                                {
                                    matched = true;
                                    break;
                                }
                            }
                        }
                    }

                    if matched {
                        state.pc += 1;
                    } else {
                        return Ok(StepResult::Fail);
                    }
                }

                Instruction::CheckNegativeLookbehind(lookbehind_program) => {
                    // Similar to CheckLookbehind but expects the pattern to NOT match
                    let mut matched = false;
                    let text_chars: Vec<char> = text.chars().collect();

                    if state.pos > 0 {
                        // Try to match the pattern ending at current position
                        let max_lookback = state.pos.min(1000); // Limit lookback distance

                        for start_offset in 1..=max_lookback {
                            let start_pos = state.pos - start_offset;

                            // Create a new VM to execute the lookbehind pattern
                            let mut lookbehind_vm = PatternVM::new();

                            // Create a slice of text to match against
                            let text_slice: String =
                                text_chars[start_pos..state.pos].iter().collect();

                            // Try to match the entire slice
                            if let Ok(result) =
                                lookbehind_vm.execute(lookbehind_program, &text_slice)
                                && result
                            {
                                // Check if the match uses the entire slice
                                let matches =
                                    lookbehind_vm.find_all(lookbehind_program, &text_slice, &[]);
                                if let Some(first_match) = matches.first()
                                    && first_match.start == 0
                                    && first_match.end == text_slice.len()
                                {
                                    matched = true;
                                    break;
                                }
                            }
                        }
                    }

                    // For negative lookbehind, we succeed if the pattern did NOT match
                    if !matched {
                        state.pc += 1;
                    } else {
                        return Ok(StepResult::Fail);
                    }
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
    Match(VMState),         // Pattern matched successfully with final state
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
        program.push(Instruction::Char('a')); // 1
        program.push(Instruction::Jump(4)); // 2: Jump to Match
        program.push(Instruction::Char('b')); // 3
        program.push(Instruction::Match); // 4

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

    #[test]
    fn test_positive_lookahead() {
        // Test pattern: digit followed by lookahead for letter
        let mut program = Program::new();
        program.push(Instruction::CharClass(CharClassType::Digit));
        program.push(Instruction::BeginLookahead);
        program.push(Instruction::CharClass(CharClassType::Letter));
        program.push(Instruction::EndLookahead);
        program.push(Instruction::Match);

        println!("Program instructions:");
        for (i, inst) in program.instructions.iter().enumerate() {
            println!("{i}: {inst:?}");
        }

        let mut vm = PatternVM::new();
        vm.debug = true;

        // Should match "5a" (digit followed by letter)
        println!("\nTesting '5a':");
        let result1 = vm.execute(&program, "5a").unwrap();
        println!("Result: {result1}");
        assert!(result1);

        // Should NOT match "59" (digit not followed by letter)
        println!("\nTesting '59':");
        let result2 = vm.execute(&program, "59").unwrap();
        println!("Result: {result2}");
        assert!(!result2);
    }
}
