/// Bytecode instructions for the pattern matching virtual machine
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Match a specific character
    Char(char),

    /// Match any character in a character class
    CharClass(CharClassType),

    /// Match a literal string
    Literal(String),

    /// Jump to another instruction (used for alternatives and quantifiers)
    Jump(usize),

    /// Split execution into two paths (for alternation and optional matching)
    Split(usize, usize), // try first address, then second

    /// Start a capture group
    StartCapture(usize), // capture group index

    /// End a capture group
    EndCapture(usize), // capture group index

    /// Match a backreference to a previously captured group
    Backreference(usize), // capture group index

    /// Match start of text
    StartAnchor,

    /// Match end of text
    EndAnchor,

    /// Successfully match
    Match,

    /// Fail to match (used for error cases)
    Fail,

    /// Save current position for backtracking
    Save(usize), // slot index

    /// Restore position from saved slot
    Restore(usize), // slot index

    /// Begin positive lookahead - save position and execute nested program
    BeginLookahead,
    
    /// End positive lookahead - restore position and continue if nested program matched
    EndLookahead,
    
    /// Begin negative lookahead - save position and execute nested program  
    BeginNegativeLookahead,
    
    /// End negative lookahead - restore position and continue if nested program failed
    EndNegativeLookahead,
    
    /// Check positive lookbehind - verify pattern matches before current position
    CheckLookbehind(usize), // length of the lookbehind pattern
    
    /// Check negative lookbehind - verify pattern doesn't match before current position
    CheckNegativeLookbehind(usize), // length of the lookbehind pattern
}

/// Character class types supported by the pattern system
#[derive(Debug, Clone, PartialEq)]
pub enum CharClassType {
    Digit,      // matches 0-9
    Letter,     // matches a-z, A-Z
    Whitespace, // matches space, tab, newline, etc.
    Any,        // matches any single character
}

impl CharClassType {
    /// Check if a character matches this character class
    pub fn matches(&self, ch: char) -> bool {
        match self {
            CharClassType::Digit => ch.is_ascii_digit(),
            CharClassType::Letter => ch.is_alphabetic(),
            CharClassType::Whitespace => ch.is_whitespace(),
            CharClassType::Any => true,
        }
    }
}

/// A compiled pattern program consisting of a sequence of instructions
#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub num_captures: usize,
    pub num_saves: usize,
}

impl Program {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            num_captures: 0,
            num_saves: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            instructions: Vec::with_capacity(capacity),
            num_captures: 0,
            num_saves: 0,
        }
    }

    pub fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Get an instruction at a specific program counter
    pub fn get(&self, pc: usize) -> Option<&Instruction> {
        self.instructions.get(pc)
    }

    /// Set the number of capture groups in this program
    pub fn set_num_captures(&mut self, count: usize) {
        self.num_captures = count;
    }

    /// Set the number of save slots needed for backtracking
    pub fn set_num_saves(&mut self, count: usize) {
        self.num_saves = count;
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_class_digit() {
        let digit_class = CharClassType::Digit;
        assert!(digit_class.matches('0'));
        assert!(digit_class.matches('5'));
        assert!(digit_class.matches('9'));
        assert!(!digit_class.matches('a'));
        assert!(!digit_class.matches(' '));
    }

    #[test]
    fn test_char_class_letter() {
        let letter_class = CharClassType::Letter;
        assert!(letter_class.matches('a'));
        assert!(letter_class.matches('Z'));
        assert!(letter_class.matches('M'));
        assert!(!letter_class.matches('5'));
        assert!(!letter_class.matches(' '));
    }

    #[test]
    fn test_char_class_whitespace() {
        let ws_class = CharClassType::Whitespace;
        assert!(ws_class.matches(' '));
        assert!(ws_class.matches('\t'));
        assert!(ws_class.matches('\n'));
        assert!(!ws_class.matches('a'));
        assert!(!ws_class.matches('5'));
    }

    #[test]
    fn test_char_class_any() {
        let any_class = CharClassType::Any;
        assert!(any_class.matches('a'));
        assert!(any_class.matches('5'));
        assert!(any_class.matches(' '));
        assert!(any_class.matches('\n'));
        assert!(any_class.matches('🦀'));
    }

    #[test]
    fn test_program_creation() {
        let mut program = Program::new();
        assert!(program.is_empty());
        assert_eq!(program.len(), 0);

        program.push(Instruction::Char('a'));
        program.push(Instruction::Match);

        assert!(!program.is_empty());
        assert_eq!(program.len(), 2);
        assert_eq!(program.get(0), Some(&Instruction::Char('a')));
        assert_eq!(program.get(1), Some(&Instruction::Match));
        assert_eq!(program.get(2), None);
    }
}
