//! Bytecode Instructions for Pattern Virtual Machine
//!
//! This module defines the instruction set for the WFL pattern matching
//! virtual machine. The instructions provide a comprehensive set of operations
//! for efficient pattern matching with Unicode support.

/// Bytecode instructions for the pattern matching virtual machine.
///
/// Each instruction represents a single operation that the VM can execute.
/// Instructions are designed to be atomic and efficient, supporting advanced
/// pattern matching features while maintaining good performance.
///
/// ## Instruction Categories
/// * **Character Matching**: `Char`, `CharClass`, `Literal`
/// * **Control Flow**: `Jump`, `Split`, `Match`, `Fail`
/// * **Captures**: `StartCapture`, `EndCapture`, `Backreference`  
/// * **Anchors**: `StartAnchor`, `EndAnchor`
/// * **Backtracking**: `Save`, `Restore`
/// * **Lookaround**: `BeginLookahead`, `EndLookahead`, etc.
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
    CheckLookbehind(Box<Program>), // sub-program to match before current position

    /// Check negative lookbehind - verify pattern doesn't match before current position
    CheckNegativeLookbehind(Box<Program>), // sub-program to match before current position
}

/// Character class types supported by the pattern system.
///
/// Provides comprehensive Unicode support including character categories,
/// scripts, and properties. This allows patterns to match character sets
/// beyond ASCII using Unicode standards.
///
/// ## Built-in Classes
/// * `Digit` - ASCII digits 0-9
/// * `Letter` - ASCII letters a-z, A-Z (extended for Unicode)
/// * `Whitespace` - Space, tab, newline, and other whitespace characters
/// * `Any` - Matches any single character (except line terminators in some modes)
///
/// ## Unicode Support
/// * `UnicodeCategory` - Unicode general categories (Letter, Number, Symbol, etc.)
/// * `UnicodeScript` - Unicode scripts (Greek, Latin, Arabic, Cyrillic, etc.)
/// * `UnicodeProperty` - Unicode properties (Alphabetic, Uppercase, etc.)
///
/// For complete Unicode category and script support, see the Unicode documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum CharClassType {
    /// ASCII digits 0-9
    Digit,
    /// Letters (ASCII a-z, A-Z, extended for Unicode)
    Letter,
    /// Whitespace characters (space, tab, newline, etc.)
    Whitespace,
    /// Any single character
    Any,
    /// Unicode general category (e.g., "Letter", "Number", "Symbol")
    UnicodeCategory(String),
    /// Unicode script (e.g., "Greek", "Latin", "Arabic")
    UnicodeScript(String),
    /// Unicode property (e.g., "Alphabetic", "Uppercase", "Lowercase")
    UnicodeProperty(String),
}

impl CharClassType {
    /// Check if a character matches this character class
    pub fn matches(&self, ch: char) -> bool {
        match self {
            CharClassType::Digit => ch.is_ascii_digit(),
            CharClassType::Letter => ch.is_alphabetic(),
            CharClassType::Whitespace => ch.is_whitespace(),
            CharClassType::Any => true,
            CharClassType::UnicodeCategory(category) => match category.as_str() {
                "Letter" | "L" => ch.is_alphabetic(),
                "Number" | "N" => ch.is_numeric(),
                "Symbol" | "S" => matches!(ch,
                    '$' | '+' | '<' | '=' | '>' | '^' | '`' | '|' | '~' |
                    '\u{00A2}'..='\u{00A5}' | '\u{00A7}' | '\u{00A9}' | '\u{00AC}' |
                    '\u{00AE}'..='\u{00B1}' | '\u{00B4}' | '\u{00B6}' | '\u{00B8}' |
                    '\u{00D7}' | '\u{00F7}' | '\u{02C2}'..='\u{02C5}' |
                    '\u{02D2}'..='\u{02DF}' | '\u{02E5}'..='\u{02EB}' | '\u{02ED}' |
                    '\u{2100}'..='\u{214F}' | '\u{2190}'..='\u{2328}' |
                    '\u{2400}'..='\u{2426}' | '\u{2440}'..='\u{244A}'
                ),
                "Punctuation" | "P" => {
                    ch.is_ascii_punctuation()
                        || matches!(ch,
                            '\u{2010}'..='\u{2027}' | '\u{2030}'..='\u{203E}' |
                            '\u{2041}'..='\u{2053}' | '\u{2055}'..='\u{205E}'
                        )
                }
                "Mark" | "M" => matches!(ch,
                    '\u{0300}'..='\u{036F}' | '\u{0483}'..='\u{0489}' |
                    '\u{0591}'..='\u{05BD}' | '\u{05BF}' | '\u{05C1}'..='\u{05C2}' |
                    '\u{05C4}'..='\u{05C5}' | '\u{05C7}' | '\u{0610}'..='\u{061A}'
                ),
                _ => false,
            },
            CharClassType::UnicodeScript(script) => match script.as_str() {
                "Latin" => matches!(ch, 'A'..='Z' | 'a'..='z' | 
                    '\u{00C0}'..='\u{00FF}' | '\u{0100}'..='\u{017F}' |
                    '\u{0180}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}'),
                "Greek" => matches!(ch, '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}'),
                "Cyrillic" => matches!(ch, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}'),
                "Arabic" => matches!(ch, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}'),
                "Hebrew" => matches!(ch, '\u{0590}'..='\u{05FF}'),
                "Devanagari" => matches!(ch, '\u{0900}'..='\u{097F}'),
                "Chinese" | "Han" => matches!(ch, '\u{4E00}'..='\u{9FFF}' | 
                    '\u{3400}'..='\u{4DBF}' | '\u{20000}'..='\u{2A6DF}'),
                "Japanese" | "Hiragana" => matches!(ch, '\u{3040}'..='\u{309F}'),
                "Katakana" => matches!(ch, '\u{30A0}'..='\u{30FF}'),
                "Korean" | "Hangul" => matches!(ch, '\u{AC00}'..='\u{D7AF}' | 
                    '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}'),
                _ => false,
            },
            CharClassType::UnicodeProperty(property) => match property.as_str() {
                "Alphabetic" => ch.is_alphabetic(),
                "Uppercase" => ch.is_uppercase(),
                "Lowercase" => ch.is_lowercase(),
                "Numeric" => ch.is_numeric(),
                "Alphanumeric" => ch.is_alphanumeric(),
                "Control" => ch.is_control(),
                _ => false,
            },
        }
    }
}

/// A compiled pattern program consisting of a sequence of instructions
#[derive(Debug, Clone, PartialEq)]
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
