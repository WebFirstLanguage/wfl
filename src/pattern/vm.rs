//! Pattern Virtual Machine - Executes Pattern Bytecode
//!
//! The pattern VM is a stack-based virtual machine that executes compiled
//! pattern bytecode. It provides efficient pattern matching with support
//! for advanced features like backtracking, lookaround assertions, and
//! capture groups.

use super::PatternError;
use super::instruction::{Instruction, Program};
use crate::exec::budget::{BudgetExceeded, ExecutionBudget, PatternMeter};
use std::collections::HashMap;
use std::sync::Arc;

/// Translate a per-match meter breach into the pattern VM's error type.
///
/// The meter enforces the pattern step ceiling, the active-state ceiling,
/// cooperative cancellation, and — unless the match is exempt (inside a
/// `main loop`) — the wall-clock deadline. A deadline breach is mapped to the
/// timeout variant (so it surfaces as the historic `[Timeout]` error), not the
/// step-limit error.
fn budget_to_pattern_error(exceeded: BudgetExceeded) -> PatternError {
    match exceeded {
        BudgetExceeded::PatternStates { .. } => PatternError::StateLimitExceeded,
        BudgetExceeded::Deadline { limit_secs } => PatternError::Timeout { limit_secs },
        BudgetExceeded::Cancelled => PatternError::Cancelled,
        _ => PatternError::StepLimitExceeded,
    }
}

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

    /// Create a match result from the already-materialized character slice,
    /// avoiding a fresh `text.chars().collect()`. Used on the hot path where the
    /// VM has collected the input once up front (see [`PatternVM`] runners).
    fn from_chars(
        start: usize,
        end: usize,
        chars: &[char],
        captures: HashMap<String, String>,
    ) -> Self {
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
    /// The per-match meter owning this match's transition and active-state
    /// counters. It borrows the run's ceilings, wall-clock deadline, and
    /// cancellation flag from the shared [`ExecutionBudget`], but its counters
    /// are private to this top-level match — nested lookaround/lookbehind VMs
    /// clone the *same* meter (via [`PatternVM::with_meter`]) so their work
    /// counts against the enclosing match, while a second, unrelated match under
    /// the same run budget gets an independent meter.
    meter: Arc<PatternMeter>,
    /// Debug flag for test mode (only available in test builds)
    #[cfg(test)]
    debug: bool,
}

impl PatternVM {
    /// A VM bound to the current-thread run budget (see
    /// [`ExecutionBudget::current_or_default`]), so stdlib pattern builtins and
    /// [`super::CompiledPattern`]'s convenience methods honour the run's
    /// configured `max_pattern_steps` / `max_pattern_states` ceilings — and fall
    /// back to a bounded default when no run is active.
    pub fn new() -> Self {
        Self::with_budget(ExecutionBudget::current_or_default())
    }

    /// A VM that shares an existing [`ExecutionBudget`], on a fresh per-match
    /// meter, so a pattern match respects the same step/state ceilings, deadline,
    /// and cancellation as the run that launched it.
    pub fn with_budget(budget: Arc<ExecutionBudget>) -> Self {
        Self::with_meter(PatternMeter::new(budget))
    }

    /// A VM that shares an existing per-match [`PatternMeter`]. Used to spawn
    /// nested lookaround/lookbehind VMs so their transitions and active states
    /// count against the enclosing match's ceilings.
    pub(crate) fn with_meter(meter: Arc<PatternMeter>) -> Self {
        Self {
            meter,
            #[cfg(test)]
            debug: false,
        }
    }

    /// Execute a pattern program against input text (just test if it matches),
    /// resetting this VM's per-match meter first so a reused VM does not carry
    /// transitions from an unrelated prior match.
    pub fn execute(&mut self, program: &Program, text: &str) -> Result<bool, PatternError> {
        self.meter.reset();
        self.run_execute(program, text)
    }

    /// Find the first match in the text — **backward-compatible** wrapper that
    /// returns `Option` and silently converts a budget/ReDoS breach to `None`.
    /// Use [`PatternVM::try_find`] to observe breaches.
    pub fn find(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Option<MatchResult> {
        self.try_find(program, text, capture_names).unwrap_or(None)
    }

    /// Find all matches in the text — **backward-compatible** wrapper that
    /// returns `Vec` and silently converts a budget/ReDoS breach to an empty
    /// result. Use [`PatternVM::try_find_all`] to observe breaches.
    pub fn find_all(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Vec<MatchResult> {
        self.try_find_all(program, text, capture_names)
            .unwrap_or_default()
    }

    /// Find the first match, resetting this VM's per-match meter first and
    /// **propagating** a budget/ReDoS breach.
    pub fn try_find(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Result<Option<MatchResult>, PatternError> {
        self.meter.reset();
        self.run_find(program, text, capture_names)
    }

    /// Find all matches, resetting this VM's per-match meter first and
    /// **propagating** a budget/ReDoS breach.
    pub fn try_find_all(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Result<Vec<MatchResult>, PatternError> {
        self.meter.reset();
        self.run_find_all(program, text, capture_names)
    }

    /// Position-loop for [`PatternVM::execute`] that does **not** reset the
    /// meter, so nested lookaround/lookbehind VMs share the enclosing match's
    /// budget rather than getting a fresh quota.
    fn run_execute(&mut self, program: &Program, text: &str) -> Result<bool, PatternError> {
        // Checkpoint *before* the O(text) preprocessing: pattern input is a
        // runtime value that is not bounded by `max_source_size` (it can be an
        // unbounded file read, client data, or a constructed string), so an
        // already-expired deadline or cancelled run must not pay to materialize
        // a large input first. Then collect the input into `Vec<char>` exactly
        // once and reuse the slice for every position and every step, instead of
        // re-collecting O(text) on each transition.
        self.meter.charge_step().map_err(budget_to_pattern_error)?;
        let chars: Vec<char> = text.chars().collect();
        // Try matching at each character position in the text.
        for start_pos in 0..=chars.len() {
            if self.execute_at_position(program, &chars, start_pos)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Position-loop for [`PatternVM::try_find`]; does not reset the meter (see
    /// [`PatternVM::run_execute`]).
    fn run_find(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Result<Option<MatchResult>, PatternError> {
        // Checkpoint before preprocessing, then collect once (see `run_execute`).
        self.meter.charge_step().map_err(budget_to_pattern_error)?;
        let chars: Vec<char> = text.chars().collect();
        // Try matching at each character position in the text.
        for start_pos in 0..=chars.len() {
            if let Some(result) =
                self.find_at_position(program, &chars, start_pos, capture_names)?
            {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    /// Position-loop for [`PatternVM::try_find_all`]; does not reset the meter
    /// (see [`PatternVM::run_execute`]).
    fn run_find_all(
        &mut self,
        program: &Program,
        text: &str,
        capture_names: &[String],
    ) -> Result<Vec<MatchResult>, PatternError> {
        // Checkpoint before preprocessing, then collect once (see `run_execute`).
        self.meter.charge_step().map_err(budget_to_pattern_error)?;
        let chars: Vec<char> = text.chars().collect();
        let mut matches = Vec::new();
        let mut pos = 0;

        while pos <= chars.len() {
            if let Some(result) = self.find_at_position(program, &chars, pos, capture_names)? {
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

        Ok(matches)
    }

    /// Execute pattern starting at a specific position
    fn execute_at_position(
        &mut self,
        program: &Program,
        chars: &[char],
        start_pos: usize,
    ) -> Result<bool, PatternError> {
        let initial_state = VMState::new(program.num_captures, program.num_saves);
        let mut states = vec![VMState {
            pos: start_pos,
            ..initial_state
        }];
        // Reserve state slots against the per-match meter. The current and the
        // next generation coexist in memory during the inner loop, and nested
        // lookaround/lookbehind frontiers (which share this meter) stack on top,
        // so `res` counts *all* simultaneously-live states. One reservation is
        // grown as the next generation is built and shrunk when the consumed
        // generation is released, so it drops correctly on every exit path.
        let mut res = self
            .meter
            .reserve_states(states.len())
            .map_err(budget_to_pattern_error)?;

        while !states.is_empty() {
            let consumed = states.len();
            let mut next_states = Vec::new();

            for state in states {
                // Each transition is charged inside `step()` (per instruction),
                // so an epsilon-jump cycle is bounded too.
                match self.step(program, chars, state)? {
                    StepResult::Continue(new_states) => {
                        // Fail fast on exponential state fan-out: reserve slots
                        // for the new states as the generation is built, not
                        // once it is complete.
                        res.grow(new_states.len())
                            .map_err(budget_to_pattern_error)?;
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

            res.release(consumed); // the previous generation is now consumed
            states = next_states;
        }

        Ok(false)
    }

    /// Find a match starting at a specific position
    fn find_at_position(
        &mut self,
        program: &Program,
        chars: &[char],
        start_pos: usize,
        capture_names: &[String],
    ) -> Result<Option<MatchResult>, PatternError> {
        let initial_state = VMState::new(program.num_captures, program.num_saves);
        let mut states = vec![VMState {
            pos: start_pos,
            ..initial_state
        }];
        // See `execute_at_position`: one reservation counts all live states
        // across current + next + nested frontiers, grown/shrunk per generation.
        let mut res = self
            .meter
            .reserve_states(states.len())
            .map_err(budget_to_pattern_error)?;

        while !states.is_empty() {
            let consumed = states.len();
            let mut next_states = Vec::new();

            for state in states {
                // Transitions are charged inside `step()` (per instruction).
                match self.step(program, chars, state)? {
                    StepResult::Continue(new_states) => {
                        // Fail fast on exponential state fan-out.
                        res.grow(new_states.len())
                            .map_err(budget_to_pattern_error)?;
                        next_states.extend(new_states);
                    }
                    StepResult::Match(final_state) => {
                        // Found a match, construct result with captures
                        let mut captures: HashMap<String, String> = HashMap::new();

                        // Extract captures from the final state, reusing the
                        // already-collected character slice (no re-collect).
                        for (i, name) in capture_names.iter().enumerate() {
                            if let Some((start, end)) = final_state.captures[i] {
                                let captured_text: String = if start <= end && end <= chars.len() {
                                    chars[start..end].iter().collect()
                                } else {
                                    String::new()
                                };
                                captures.insert(name.clone(), captured_text);
                            }
                        }

                        return Ok(Some(MatchResult::from_chars(
                            start_pos,
                            final_state.pos,
                            chars,
                            captures,
                        )));
                    }
                    StepResult::Fail => {
                        // This execution path failed, try others
                    }
                }
            }

            res.release(consumed); // the previous generation is now consumed
            states = next_states;
        }

        Ok(None)
    }

    /// Execute one step of the virtual machine.
    ///
    /// `chars` is the input already materialized once by the calling runner, so
    /// this hot path performs no per-step `text.chars().collect()` (which would
    /// be O(text) on every transition for unbounded runtime input).
    #[allow(clippy::only_used_in_recursion)]
    fn step(
        &mut self,
        program: &Program,
        chars: &[char],
        mut state: VMState,
    ) -> Result<StepResult, PatternError> {
        loop {
            // Charge one transition per dispatched instruction against the
            // per-match meter. This is the real ReDoS guard: it bounds every
            // instruction chain (including epsilon-`Jump` cycles that never
            // consume input) and, because nested lookaround/lookbehind VMs share
            // this meter, counts their work against the same match. It also
            // samples the wall-clock deadline (unless exempt), so a single
            // synchronous match cannot run past `timeout_seconds`.
            self.meter.charge_step().map_err(budget_to_pattern_error)?;

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

                    // Try to match the lookahead pattern at the current position.
                    // The nested VM shares this match's meter, so its transitions
                    // and active states count against the same ceilings.
                    let mut lookahead_vm = PatternVM::with_meter(Arc::clone(&self.meter));
                    #[cfg(test)]
                    {
                        lookahead_vm.debug = self.debug;
                    }

                    let lookahead_matched =
                        lookahead_vm.execute_at_position(&lookahead_program, chars, state.pos)?;

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
                    // Reserve the negative-lookahead frontier against the same
                    // per-match meter, so its states add to (rather than escape)
                    // the enclosing match's active-state ceiling.
                    let mut res = self
                        .meter
                        .reserve_states(current_states.len())
                        .map_err(budget_to_pattern_error)?;

                    'outer: while depth > 0 && !current_states.is_empty() {
                        let consumed = current_states.len();
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

                            match self.step(program, chars, lookahead_state)? {
                                StepResult::Fail => {
                                    // Good - this path failed
                                }
                                StepResult::Continue(states) => {
                                    // Fail fast on fan-out; the inner `self.step()`
                                    // calls already charge transitions.
                                    res.grow(states.len()).map_err(budget_to_pattern_error)?;
                                    next_states.extend(states);
                                }
                                StepResult::Match(_) => {
                                    // Pattern matched inside negative lookahead - fail
                                    return Ok(StepResult::Fail);
                                }
                            }
                        }

                        res.release(consumed); // the previous generation is consumed
                        current_states = next_states;
                    }
                    drop(res); // release the negative-lookahead frontier

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

                    // Get the text before current position
                    if state.pos > 0 {
                        // Try to match the pattern ending at current position
                        // We'll try different starting positions
                        let max_lookback = state.pos.min(1000); // Limit lookback distance

                        for start_offset in 1..=max_lookback {
                            let start_pos = state.pos - start_offset;

                            // Create a nested VM sharing this match's meter, so
                            // its transitions/states count against the same
                            // per-match ceilings.
                            let mut lookbehind_vm = PatternVM::with_meter(Arc::clone(&self.meter));

                            // Create a slice of text to match against, from the
                            // already-materialized character slice.
                            let text_slice: String = chars[start_pos..state.pos].iter().collect();

                            // Try to match the entire slice with the non-resetting
                            // runners, so the shared per-match meter is not reset.
                            // `?` propagates a budget breach from the nested VM.
                            if lookbehind_vm.run_execute(lookbehind_program, &text_slice)? {
                                // Check if the match uses the entire slice
                                let matches = lookbehind_vm.run_find_all(
                                    lookbehind_program,
                                    &text_slice,
                                    &[],
                                )?;
                                // `first_match.end` is a CHARACTER index (like
                                // every `MatchResult` offset), while
                                // `text_slice.len()` is a BYTE count — they
                                // diverge on multibyte UTF-8, inverting the
                                // full-slice test. The slice spans exactly
                                // `start_offset` characters, so compare with that.
                                if let Some(first_match) = matches.first()
                                    && first_match.start == 0
                                    && first_match.end == start_offset
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

                    if state.pos > 0 {
                        // Try to match the pattern ending at current position
                        let max_lookback = state.pos.min(1000); // Limit lookback distance

                        for start_offset in 1..=max_lookback {
                            let start_pos = state.pos - start_offset;

                            // Create a nested VM sharing this match's meter, so
                            // its transitions/states count against the same
                            // per-match ceilings.
                            let mut lookbehind_vm = PatternVM::with_meter(Arc::clone(&self.meter));

                            // Create a slice of text to match against, from the
                            // already-materialized character slice.
                            let text_slice: String = chars[start_pos..state.pos].iter().collect();

                            // Try to match the entire slice with the non-resetting
                            // runners, so the shared per-match meter is not reset.
                            // `?` propagates a budget breach from the nested VM.
                            if lookbehind_vm.run_execute(lookbehind_program, &text_slice)? {
                                // Check if the match uses the entire slice
                                let matches = lookbehind_vm.run_find_all(
                                    lookbehind_program,
                                    &text_slice,
                                    &[],
                                )?;
                                // `first_match.end` is a CHARACTER index (like
                                // every `MatchResult` offset), while
                                // `text_slice.len()` is a BYTE count — they
                                // diverge on multibyte UTF-8, inverting the
                                // full-slice test. The slice spans exactly
                                // `start_offset` characters, so compare with that.
                                if let Some(first_match) = matches.first()
                                    && first_match.start == 0
                                    && first_match.end == start_offset
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
    fn find_uses_char_indices_across_multibyte_input() {
        // The input is materialized into `Vec<char>` once up front and matched by
        // CHARACTER index. A digit after multibyte characters must be located at
        // its char index (not a byte offset), and the matched text must be that
        // one character — guarding the single up-front collection and the
        // `chars.len()` position bound.
        let mut program = Program::new();
        program.push(Instruction::CharClass(CharClassType::Digit));
        program.push(Instruction::Match);

        let mut vm = PatternVM::new();
        // "café☕7": c a f é ☕ 7 → the digit is at character index 5.
        let result = vm
            .find(&program, "café☕7", &[])
            .expect("the digit should be found");
        assert_eq!(result.matched_text, "7");
        assert_eq!(result.start, 5);
        assert_eq!(result.end, 6);
    }

    #[test]
    fn cancellation_is_observed_before_materializing_large_input() {
        use crate::exec::budget::ExecutionBudget;
        use std::sync::Arc;

        // Pattern input is a runtime value not bounded by `max_source_size`. The
        // runner checkpoints the budget *before* collecting the input, so an
        // already-cancelled run aborts without paying the O(text) materialization
        // (and without the old per-step re-collection).
        let mut program = Program::new();
        program.push(Instruction::Char('z'));
        program.push(Instruction::Match);

        let budget = Arc::new(ExecutionBudget::default());
        budget.cancel();
        let mut vm = PatternVM::with_budget(budget);

        let big = "a".repeat(1_000_000);
        let result = vm.try_find(&program, &big, &[]);
        assert!(
            result.is_err(),
            "a cancelled run must abort before matching a large input"
        );
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

#[cfg(test)]
mod unicode_lookbehind_tests {
    //! Lookbehind must keep its full-slice test entirely in CHARACTER indices.
    //! The lookbehind slice is built from a `Vec<char>`, and `MatchResult::end`
    //! is a character index; comparing it against the slice's BYTE length
    //! (`String::len()`) diverges the moment the window holds a multibyte char,
    //! inverting the assertion. These pin both directions over a 2-byte `é`.
    use crate::parser::ast::PatternExpression;
    use crate::pattern::CompiledPattern;

    fn matches(pattern: &PatternExpression, text: &str) -> bool {
        CompiledPattern::compile(pattern)
            .expect("pattern compiles")
            .matches(text)
    }

    fn lit(s: &str) -> PatternExpression {
        PatternExpression::Literal(s.to_string())
    }

    #[test]
    fn positive_lookbehind_spans_a_multibyte_window() {
        // (?<=café)!  — the "café" window ends at character index 4 but byte
        // index 5, so a byte-length comparison would never see a full-slice
        // match and the assertion would wrongly fail.
        let pattern = PatternExpression::Sequence(vec![
            PatternExpression::Lookbehind(Box::new(lit("café"))),
            lit("!"),
        ]);
        assert!(
            matches(&pattern, "café!"),
            "positive lookbehind must span the multibyte 'é' window"
        );
        assert!(
            !matches(&pattern, "cafe!"),
            "a window that does not match must not satisfy the lookbehind"
        );
    }

    #[test]
    fn negative_lookbehind_spans_a_multibyte_window() {
        // (?<!café)!  — must FAIL on "café!" because the forbidden multibyte
        // window IS present. A byte/char mix-up under-detects it and would let
        // the negative assertion wrongly succeed.
        let pattern = PatternExpression::Sequence(vec![
            PatternExpression::NegativeLookbehind(Box::new(lit("café"))),
            lit("!"),
        ]);
        assert!(
            !matches(&pattern, "café!"),
            "negative lookbehind must detect the multibyte forbidden window"
        );
        assert!(
            matches(&pattern, "cafe!"),
            "absent forbidden window → the negative assertion matches"
        );
    }
}
