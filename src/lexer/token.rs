use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f\r]+|//.*|#.*")] // Skip whitespace (excluding newline) and line comments (// and #)
pub enum Token {
    #[token("\n")]
    Newline,  // Keep for internal use (flushes multi-word identifiers)

    // NEW: Explicit end-of-line token emitted to parser
    Eol,

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
    #[token("directory")]
    KeywordDirectory,
    #[token("delete")]
    KeywordDelete,
    #[token("exists")]
    KeywordExists,
    #[token("list")]
    KeywordList,
    #[token("map")]
    KeywordMap,
    #[token("remove")]
    KeywordRemove,
    #[token("clear")]
    KeywordClear,
    #[token("files")]
    KeywordFiles,
    #[token("found")]
    KeywordFound,
    #[token("permission")]
    KeywordPermission,
    #[token("denied")]
    KeywordDenied,
    #[token("recursively")]
    KeywordRecursively,
    #[token("extension")]
    KeywordExtension,
    #[token("extensions")]
    KeywordExtensions,
    #[token("url")]
    KeywordUrl,
    #[token("database")]
    KeywordDatabase,
    #[token("at")]
    KeywordAt,
    #[token("listen")]
    KeywordListen,
    #[token("port")]
    KeywordPort,
    #[token("server")]
    KeywordServer,
    #[token("request")]
    KeywordRequest,
    #[token("response")]
    KeywordResponse,
    #[token("respond")]
    KeywordRespond,
    #[token("comes")]
    KeywordComes,
    #[token("status")]
    KeywordStatus,
    #[token("least")]
    KeywordLeast,
    #[token("most")]
    KeywordMost,
    #[token("read")]
    KeywordRead,
    #[token("write")]
    KeywordWrite,
    #[token("append")]
    KeywordAppend,
    #[token("execute")]
    KeywordExecute,
    #[token("spawn")]
    KeywordSpawn,
    #[token("using")]
    KeywordUsing,
    #[token("shell")]
    KeywordShell,
    #[token("kill")]
    KeywordKill,
    #[token("process")]
    KeywordProcess,
    #[token("command")]
    KeywordCommand,
    #[token("output")]
    KeywordOutput,
    #[token("running")]
    KeywordRunning,
    #[token("arguments")]
    KeywordArguments,
    #[token("appending")]
    KeywordAppending,
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
    #[token("catch")]
    KeywordCatch,
    #[token("data")]
    KeywordData,
    #[token("date")]
    KeywordDate,
    #[token("time")]
    KeywordTime,
    // #[token("otherwise")]
    // KeywordOtherwise,
    #[token("error")]
    KeywordError,
    #[token("add")]
    KeywordAdd, // arithmetic operation keywords
    #[token("subtract")]
    KeywordSubtract,
    #[token("multiply")]
    KeywordMultiply,
    #[token("divide")]
    KeywordDivide,
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
    #[token("captured")]
    KeywordCaptured,
    #[token("digit")]
    KeywordDigit,
    #[token("letter")]
    KeywordLetter,
    #[token("whitespace")]
    KeywordWhitespace,
    #[token("character")]
    KeywordCharacter,
    #[token("unicode")]
    KeywordUnicode,
    #[token("category")]
    KeywordCategory,
    #[token("script")]
    KeywordScript,
    #[token("greedy")]
    KeywordGreedy,
    #[token("lazy")]
    KeywordLazy,
    #[token("zero")]
    KeywordZero,
    #[token("one")]
    KeywordOne,
    #[token("any")]
    KeywordAny,
    #[token("optional")]
    KeywordOptional,
    #[token("between")]
    KeywordBetween,
    #[token("start")]
    KeywordStart,
    #[token("text")]
    KeywordText,
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
    #[token("same")]
    KeywordSame,
    #[token("ahead")]
    KeywordAhead,
    #[token("behind")]
    KeywordBehind,

    // Container-related keywords
    #[token("container")]
    KeywordContainer,
    #[token("property")]
    KeywordProperty,
    #[token("extends")]
    KeywordExtends,
    #[token("implements")]
    KeywordImplements,
    #[token("interface")]
    KeywordInterface,
    #[token("requires")]
    KeywordRequires,
    #[token("event")]
    KeywordEvent,
    #[token("trigger")]
    KeywordTrigger,
    #[token("on")]
    KeywordOn,
    #[token("static")]
    KeywordStatic,
    #[token("public")]
    KeywordPublic,
    #[token("private")]
    KeywordPrivate,
    #[token("parent")]
    KeywordParent,
    #[token("new")]
    KeywordNew,
    #[token("constant")]
    KeywordConstant,
    #[token("must")]
    KeywordMust,
    #[token("defaults")]
    KeywordDefaults,

    // Web server and signal handling tokens
    #[token("register")]
    KeywordRegister,
    #[token("signal")]
    KeywordSignal,
    #[token("handler")]
    KeywordHandler,
    #[token("stop")]
    KeywordStop,
    #[token("accepting")]
    KeywordAccepting,
    #[token("connections")]
    KeywordConnections,
    #[token("timeout")]
    KeywordTimeout,
    #[token("header")]
    KeywordHeader,
    #[token("current")]
    KeywordCurrent,
    #[token("milliseconds")]
    KeywordMilliseconds,
    #[token("formatted")]
    KeywordFormatted,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("%")]
    Percent,

    #[token(".")]
    Dot,

    #[token("=")]
    Equals,

    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    #[token("{")]
    LeftBrace,

    #[token("}")]
    RightBrace,

    #[regex("(?:yes|no|true|false)", |lex| {
        let text = lex.slice().to_ascii_lowercase();
        text == "yes" || text == "true"
    })]
    BooleanLiteral(bool),

    #[token("nothing")]
    #[token("missing")]
    #[token("undefined")]
    NothingLiteral,

    #[regex(r#""([^"\\]|\\.)*""#, |lex| parse_string(lex).ok())] // captures content inside quotes
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

fn parse_string(lex: &mut logos::Lexer<Token>) -> Result<String, ()> {
    let quoted = lex.slice(); // e.g. "\"Alice\""
    let inner = &quoted[1..quoted.len() - 1]; // strip the surrounding quotes

    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('0') => result.push('\0'),
                Some('"') => result.push('"'),
                Some(_) => {
                    // Invalid escape sequence - return error
                    return Err(());
                }
                None => {
                    // Trailing backslash - error
                    return Err(());
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
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
    /// Returns true if this token is a structural keyword that must be reserved
    /// These are keywords that define program structure and control flow
    pub fn is_structural_keyword(&self) -> bool {
        matches!(
            self,
            Token::KeywordStore
                | Token::KeywordDisplay
                | Token::KeywordCheck
                | Token::KeywordIf
                | Token::KeywordThen
                | Token::KeywordOtherwise
                | Token::KeywordEnd
                | Token::KeywordFor
                | Token::KeywordEach
                | Token::KeywordIn
                | Token::KeywordFrom
                | Token::KeywordTo
                | Token::KeywordBy
                | Token::KeywordRepeat
                | Token::KeywordWhile
                | Token::KeywordUntil
                | Token::KeywordForever
                | Token::KeywordAction
                | Token::KeywordWith
                | Token::KeywordBreak
                | Token::KeywordContinue
                | Token::KeywordReturn
                | Token::KeywordAs
                | Token::KeywordDefine
                | Token::KeywordAnd
                | Token::KeywordOr
                | Token::KeywordNot
                | Token::KeywordWait
                | Token::KeywordTry
                | Token::KeywordWhen
                | Token::KeywordCatch
                | Token::KeywordSkip
                | Token::KeywordThan
                | Token::KeywordPush
                | Token::KeywordZero
                | Token::KeywordAny
                | Token::KeywordContainer
                | Token::KeywordProperty
                | Token::KeywordExtends
                | Token::KeywordImplements
                | Token::KeywordInterface
                | Token::KeywordRequires
                | Token::KeywordEvent
                | Token::KeywordTrigger
                | Token::KeywordOn
                | Token::KeywordStatic
                | Token::KeywordPublic
                | Token::KeywordPrivate
                | Token::KeywordConstant
        )
    }

    /// Returns true if this token is a contextual keyword
    /// These can be used as variable names when not in their keyword context
    pub fn is_contextual_keyword(&self) -> bool {
        matches!(
            self,
            Token::KeywordCount        // Only reserved in 'count from X to Y' context
                | Token::KeywordPattern // Only reserved in pattern matching context
                | Token::KeywordFiles   // Only reserved in file operations context
                | Token::KeywordExtension
                | Token::KeywordExtensions
                | Token::KeywordContains // Can be a function name
                | Token::KeywordList    // Only reserved in type/create context
                | Token::KeywordMap     // Only reserved in type/create context
                | Token::KeywordText    // Only reserved in type context
                | Token::KeywordCreate  // Context-sensitive for expressions
                | Token::KeywordNew     // Context-sensitive
                | Token::KeywordParent  // Context-sensitive
                | Token::KeywordRead    // Context-sensitive
                | Token::KeywordPush    // Context-sensitive
                | Token::KeywordSkip    // Context-sensitive
                | Token::KeywordGive
                | Token::KeywordBack
                | Token::KeywordCalled
                | Token::KeywordNeeds
                | Token::KeywordChange
                | Token::KeywordReversed
                | Token::KeywordAt
                | Token::KeywordLeast
                | Token::KeywordMost
                | Token::KeywordThan
                | Token::KeywordZero
                | Token::KeywordAny
                | Token::KeywordMust
                | Token::KeywordDefaults
        )
    }

    /// Legacy method for backward compatibility
    /// Returns true for any keyword (structural or contextual)
    pub fn is_keyword(&self) -> bool {
        self.is_structural_keyword() || self.is_contextual_keyword()
    }
}
