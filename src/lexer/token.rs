use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f\r]+|//.*")] // Skip whitespace (excluding newline) and line comments
pub enum Token {
    #[token("\n")]
    Newline,
    #[token("store")]
    KeywordStore,
    #[token("create")]
    KeywordCreate,
    #[token("display")]
    KeywordDisplay,
    #[token("change")]
    KeywordChange,
    #[token("if")]
    KeywordIf,
    #[token("check")]
    KeywordCheck,
    #[token("otherwise")]
    KeywordOtherwise,
    #[token("then")]
    KeywordThen,
    #[token("end")]
    KeywordEnd,
    #[token("as")]
    KeywordAs,
    #[token("to")]
    KeywordTo,
    #[token("from")]
    KeywordFrom,
    #[token("with")]
    KeywordWith,
    #[token("and")]
    KeywordAnd,
    #[token("or")]
    KeywordOr,
    #[token("count")]
    KeywordCount,
    #[token("for")]
    KeywordFor,
    #[token("each")]
    KeywordEach,
    #[token("in")]
    KeywordIn,
    #[token("reversed")]
    KeywordReversed,
    #[token("repeat")]
    KeywordRepeat,
    #[token("while")]
    KeywordWhile,
    #[token("until")]
    KeywordUntil,
    #[token("forever")]
    KeywordForever,
    #[token("skip")]
    KeywordSkip, // equivalent to 'continue'
    #[token("continue")]
    KeywordContinue,
    #[token("break")]
    KeywordBreak,
    #[token("exit")]
    KeywordExit, // for "exit loop"
    #[token("loop")]
    KeywordLoop,
    #[token("define")]
    KeywordDefine,
    #[token("action")]
    KeywordAction,
    #[token("called")]
    KeywordCalled,
    #[token("needs")]
    KeywordNeeds,
    #[token("give")]
    KeywordGive,
    #[token("back")]
    KeywordBack, // used in "give back" (return)
    #[token("return")]
    KeywordReturn, // synonym for "give back"
    #[token("open")]
    KeywordOpen,
    #[token("close")]
    KeywordClose,
    #[token("file")]
    KeywordFile,
    #[token("url")]
    KeywordUrl,
    #[token("database")]
    KeywordDatabase,
    #[token("at")]
    KeywordAt,
    #[token("read")]
    KeywordRead,
    #[token("write")]
    KeywordWrite,
    #[token("append")]
    KeywordAppend,
    #[token("content")]
    KeywordContent,
    #[token("into")]
    KeywordInto, // (if needed for phrasing like "into variable")
    #[token("wait")]
    KeywordWait,
    #[token("try")]
    KeywordTry,
    #[token("when")]
    KeywordWhen,
    #[token("data")]
    KeywordData,
    // #[token("otherwise")]
    // KeywordOtherwise,
    #[token("error")]
    KeywordError,
    #[token("plus")]
    KeywordPlus, // arithmetic operators in word form
    #[token("minus")]
    KeywordMinus,
    #[token("times")]
    KeywordTimes,
    #[token("divided by")]
    KeywordDividedBy,
    #[token("divided")]
    KeywordDivided, // e.g., "divided by"
    #[token("by")]
    KeywordBy,
    #[token("contains")]
    KeywordContains,
    #[token("pattern")]
    KeywordPattern,
    #[token("matches")]
    KeywordMatches,
    #[token("find")]
    KeywordFind,
    #[token("replace")]
    KeywordReplace,
    #[token("split")]
    KeywordSplit,
    #[token("of")]
    KeywordOf,
    #[token("more")]
    KeywordMore,
    #[token("exactly")]
    KeywordExactly,
    #[token("capture")]
    KeywordCapture,
    #[token("digit")]
    KeywordDigit,
    #[token("letter")]
    KeywordLetter,
    #[token("whitespace")]
    KeywordWhitespace,
    #[token("greedy")]
    KeywordGreedy,
    #[token("lazy")]
    KeywordLazy,
    #[token("push")]
    KeywordPush,
    #[token("above")]
    KeywordAbove, // e.g., "is above 100"
    #[token("below")]
    KeywordBelow,
    #[token("equal")]
    KeywordEqual, // e.g., "is equal to"
    #[token("greater")]
    KeywordGreater,
    #[token("less")]
    KeywordLess,
    #[token("not")]
    KeywordNot,
    #[token("is")]
    KeywordIs,
    #[token("than")]
    KeywordThan,

    #[token(":")]
    Colon,

    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    #[regex("(?:yes|no|true|false)", |lex| {
        let text = lex.slice().to_ascii_lowercase();
        text == "yes" || text == "true"
    })]
    BooleanLiteral(bool),

    #[token("nothing")]
    #[token("missing")]
    #[token("undefined")]
    NothingLiteral,

    #[regex(r#""([^"\\]|\\.)*""#, |lex| parse_string(lex))] // captures content inside quotes
    StringLiteral(String),

    #[regex("[0-9]+\\.[0-9]+", |lex| lex.slice().parse::<f64>().unwrap())]
    FloatLiteral(f64),

    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLiteral(i64),

    #[regex("[A-Za-z][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    Error,
}

fn parse_string(lex: &mut logos::Lexer<Token>) -> String {
    let quoted = lex.slice(); // e.g. "\"Alice\""
    let inner = &quoted[1..quoted.len() - 1]; // strip the surrounding quotes
    inner.replace(r#"\""#, "\"")
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithPosition {
    pub token: Token,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl TokenWithPosition {
    pub fn new(token: Token, line: usize, column: usize, length: usize) -> Self {
        Self {
            token,
            line,
            column,
            length,
        }
    }
}

impl Token {
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::KeywordStore
                | Token::KeywordCreate
                | Token::KeywordDisplay
                | Token::KeywordCheck
                | Token::KeywordIf
                | Token::KeywordThen
                | Token::KeywordOtherwise
                | Token::KeywordEnd
                | Token::KeywordFor
                | Token::KeywordEach
                | Token::KeywordIn
                | Token::KeywordReversed
                | Token::KeywordFrom
                | Token::KeywordTo
                | Token::KeywordBy
                | Token::KeywordCount
                | Token::KeywordRepeat
                | Token::KeywordWhile
                | Token::KeywordUntil
                | Token::KeywordForever
                | Token::KeywordAction
                | Token::KeywordCalled
                | Token::KeywordWith
                | Token::KeywordNot
                | Token::KeywordBreak
                | Token::KeywordContinue
                | Token::KeywordReturn
                | Token::KeywordGive
                | Token::KeywordBack
                | Token::KeywordAs
                | Token::KeywordAt
                | Token::KeywordDefine
                | Token::KeywordNeeds
                | Token::KeywordChange
                | Token::KeywordAnd
                | Token::KeywordOr
                | Token::KeywordPattern
                | Token::KeywordRead
                | Token::KeywordWait
                | Token::KeywordSkip
                | Token::KeywordThan
                | Token::KeywordPush
        )
    }
}
