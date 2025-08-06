use std::fmt;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Represents the visibility of a container member (property or method)
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Visibility {
    #[default]
    Public,
    Private,
}

/// Types of validation rules that can be applied to properties
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationRuleType {
    NotEmpty,
    MinLength,
    MaxLength,
    ExactLength,
    MinValue,
    MaxValue,
    Pattern,
    Custom,
}

/// Represents a validation rule for a property
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub parameters: Vec<Expression>,
    pub line: usize,
    pub column: usize,
}

/// Represents a property definition in a container
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDefinition {
    pub name: String,
    pub property_type: Option<Type>,
    pub default_value: Option<Expression>,
    pub validation_rules: Vec<ValidationRule>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub line: usize,
    pub column: usize,
}

/// Represents a property initializer in a container instantiation
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyInitializer {
    pub name: String,
    pub value: Expression,
    pub line: usize,
    pub column: usize,
}

/// Represents an action signature in an interface
#[derive(Debug, Clone, PartialEq)]
pub struct ActionSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub line: usize,
    pub column: usize,
}

/// Represents an event definition in a container
#[derive(Debug, Clone, PartialEq)]
pub struct EventDefinition {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VariableDeclaration {
        name: String,
        value: Expression,
        line: usize,
        column: usize,
    },
    Assignment {
        name: String,
        value: Expression,
        line: usize,
        column: usize,
    },
    IfStatement {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
        line: usize,
        column: usize,
    },
    SingleLineIf {
        condition: Expression,
        then_stmt: Box<Statement>,
        else_stmt: Option<Box<Statement>>,
        line: usize,
        column: usize,
    },
    ForEachLoop {
        item_name: String,
        collection: Expression,
        reversed: bool,
        body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    CountLoop {
        start: Expression,
        end: Expression,
        step: Option<Expression>,
        downward: bool,
        body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    WhileLoop {
        condition: Expression,
        body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    RepeatWhileLoop {
        condition: Expression,
        body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    RepeatUntilLoop {
        condition: Expression,
        body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    ForeverLoop {
        body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    DisplayStatement {
        value: Expression,
        line: usize,
        column: usize,
    },
    ActionDefinition {
        name: String,
        parameters: Vec<Parameter>,
        body: Vec<Statement>,
        return_type: Option<Type>,
        line: usize,
        column: usize,
    },
    ReturnStatement {
        value: Option<Expression>,
        line: usize,
        column: usize,
    },
    ExpressionStatement {
        expression: Expression,
        line: usize,
        column: usize,
    },
    BreakStatement {
        line: usize,
        column: usize,
    },
    ContinueStatement {
        line: usize,
        column: usize,
    },
    ExitStatement {
        line: usize,
        column: usize,
    },
    OpenFileStatement {
        path: Expression,
        variable_name: String,
        mode: FileOpenMode,
        line: usize,
        column: usize,
    },
    ReadFileStatement {
        path: Expression,
        variable_name: String,
        line: usize,
        column: usize,
    },
    WriteFileStatement {
        file: Expression,
        content: Expression,
        mode: WriteMode,
        line: usize,
        column: usize,
    },
    WriteToStatement {
        content: Expression,
        file: Expression,
        line: usize,
        column: usize,
    },
    CloseFileStatement {
        file: Expression,
        line: usize,
        column: usize,
    },
    CreateDirectoryStatement {
        path: Expression,
        line: usize,
        column: usize,
    },
    CreateFileStatement {
        path: Expression,
        content: Expression,
        line: usize,
        column: usize,
    },
    DeleteFileStatement {
        path: Expression,
        line: usize,
        column: usize,
    },
    DeleteDirectoryStatement {
        path: Expression,
        line: usize,
        column: usize,
    },
    WaitForStatement {
        inner: Box<Statement>,
        line: usize,
        column: usize,
    },
    TryStatement {
        body: Vec<Statement>,
        when_clauses: Vec<WhenClause>,
        otherwise_block: Option<Vec<Statement>>,
        line: usize,
        column: usize,
    },
    HttpGetStatement {
        url: Expression,
        variable_name: String,
        line: usize,
        column: usize,
    },
    HttpPostStatement {
        url: Expression,
        data: Expression,
        variable_name: String,
        line: usize,
        column: usize,
    },
    PushStatement {
        list: Expression,
        value: Expression,
        line: usize,
        column: usize,
    },
    // Container-related statements
    ContainerDefinition {
        name: String,
        extends: Option<String>,
        implements: Vec<String>,
        properties: Vec<PropertyDefinition>,
        methods: Vec<Statement>,
        events: Vec<EventDefinition>,
        static_properties: Vec<PropertyDefinition>,
        static_methods: Vec<Statement>,
        line: usize,
        column: usize,
    },
    ContainerInstantiation {
        container_type: String,
        instance_name: String,
        arguments: Vec<Argument>,
        property_initializers: Vec<PropertyInitializer>,
        line: usize,
        column: usize,
    },
    InterfaceDefinition {
        name: String,
        extends: Vec<String>,
        required_actions: Vec<ActionSignature>,
        line: usize,
        column: usize,
    },
    EventDefinition {
        name: String,
        parameters: Vec<Parameter>,
        line: usize,
        column: usize,
    },
    EventTrigger {
        name: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
    EventHandler {
        event_source: Expression,
        event_name: String,
        handler_body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    ParentMethodCall {
        method_name: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
    PatternDefinition {
        name: String,
        pattern: PatternExpression,
        line: usize,
        column: usize,
    },
    // Network-related statements
    AcceptConnection {
        listener_handle: Box<Expression>,
        connection_variable: String,
        line: usize,
        column: usize,
    },
    ReadFromConnection {
        connection_handle: Box<Expression>,
        variable_name: String,
        line: usize,
        column: usize,
    },
    WriteToConnection {
        connection_handle: Box<Expression>,
        data: Box<Expression>,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal, usize, usize), // line, column
    Variable(String, usize, usize), // line, column
    BinaryOperation {
        left: Box<Expression>,
        operator: Operator,
        right: Box<Expression>,
        line: usize,
        column: usize,
    },
    UnaryOperation {
        operator: UnaryOperator,
        expression: Box<Expression>,
        line: usize,
        column: usize,
    },
    FunctionCall {
        function: Box<Expression>,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
    MemberAccess {
        object: Box<Expression>,
        property: String,
        line: usize,
        column: usize,
    },
    ActionCall {
        name: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
    IndexAccess {
        collection: Box<Expression>,
        index: Box<Expression>,
        line: usize,
        column: usize,
    },
    Concatenation {
        left: Box<Expression>,
        right: Box<Expression>,
        line: usize,
        column: usize,
    },
    PatternMatch {
        text: Box<Expression>,
        pattern: Box<Expression>,
        line: usize,
        column: usize,
    },
    PatternFind {
        text: Box<Expression>,
        pattern: Box<Expression>,
        line: usize,
        column: usize,
    },
    PatternReplace {
        text: Box<Expression>,
        pattern: Box<Expression>,
        replacement: Box<Expression>,
        line: usize,
        column: usize,
    },
    PatternSplit {
        text: Box<Expression>,
        pattern: Box<Expression>,
        line: usize,
        column: usize,
    },
    AwaitExpression {
        expression: Box<Expression>,
        line: usize,
        column: usize,
    },
    // Container-related expressions
    StaticMemberAccess {
        container: String,
        member: String,
        line: usize,
        column: usize,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
    PropertyAccess {
        object: Box<Expression>,
        property: String,
        line: usize,
        column: usize,
    },
    FileExists {
        path: Box<Expression>,
        line: usize,
        column: usize,
    },
    DirectoryExists {
        path: Box<Expression>,
        line: usize,
        column: usize,
    },
    ListFiles {
        path: Box<Expression>,
        line: usize,
        column: usize,
    },
    ReadContent {
        file_handle: Box<Expression>,
        line: usize,
        column: usize,
    },
    ListFilesRecursive {
        path: Box<Expression>,
        extensions: Option<Vec<Expression>>,
        line: usize,
        column: usize,
    },
    ListFilesFiltered {
        path: Box<Expression>,
        extensions: Vec<Expression>,
        line: usize,
        column: usize,
    },
    // Network-related expressions
    ListenOnPort {
        port: Box<Expression>,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Nothing,
    Pattern(String),
    List(Vec<Expression>),
}

/// Represents different types of character classes in patterns
#[derive(Debug, Clone, PartialEq)]
pub enum CharClass {
    Digit,      // digit
    Letter,     // letter
    Whitespace, // whitespace
    // Unicode categories
    UnicodeCategory(String), // e.g., "Letter", "Number", "Symbol"
    UnicodeScript(String),   // e.g., "Greek", "Latin", "Arabic"
    UnicodeProperty(String), // e.g., "Alphabetic", "Uppercase", "Lowercase"
}

/// Represents different types of quantifiers
#[derive(Debug, Clone, PartialEq)]
pub enum Quantifier {
    Optional,          // optional
    ZeroOrMore,        // zero or more
    OneOrMore,         // one or more
    Exactly(u32),      // exactly N
    Between(u32, u32), // between N and M
}

/// Represents different types of anchors
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    StartOfText, // start of text
    EndOfText,   // end of text
}

/// Represents a pattern expression in the new pattern matching system
#[derive(Debug, Clone, PartialEq)]
pub enum PatternExpression {
    /// Literal text to match exactly
    Literal(String),
    /// Character class (digit, letter, whitespace)
    CharacterClass(CharClass),
    /// A quantified pattern (e.g., "one or more digit")
    Quantified {
        pattern: Box<PatternExpression>,
        quantifier: Quantifier,
    },
    /// A sequence of patterns (e.g., "digit '-' digit")
    Sequence(Vec<PatternExpression>),
    /// Alternative patterns (e.g., "letter or digit")
    Alternative(Vec<PatternExpression>),
    /// Named capture group
    Capture {
        name: String,
        pattern: Box<PatternExpression>,
    },
    /// Backreference to a previously captured group
    Backreference(String),
    /// Anchor pattern (start/end of text)
    Anchor(Anchor),
    /// Positive lookahead - matches if pattern would match ahead
    Lookahead(Box<PatternExpression>),
    /// Negative lookahead - matches if pattern would NOT match ahead  
    NegativeLookahead(Box<PatternExpression>),
    /// Positive lookbehind - matches if pattern would match behind
    Lookbehind(Box<PatternExpression>),
    /// Negative lookbehind - matches if pattern would NOT match behind
    NegativeLookbehind(Box<PatternExpression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    And,
    Or,
    Contains,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,
    Minus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<Type>,
    pub default_value: Option<Expression>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub name: Option<String>,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Text,
    Number,
    Boolean,
    Nothing,
    Pattern,
    Custom(String),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    Unknown,          // Used during type inference before a type is determined
    Error,            // Used to mark expressions that have already failed type checking
    Async(Box<Type>), // For asynchronous operations returning a value of Type
    Any,              // Used for generic types like lists of any type
    // Container-related types
    Container(String),
    ContainerInstance(String),
    Interface(String),
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    pub fn new(message: String, line: usize, column: usize) -> Self {
        ParseError {
            message,
            line,
            column,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WriteMode {
    Overwrite,
    Append,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileOpenMode {
    Read,
    Write,
    Append,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    General,
    FileNotFound,
    PermissionDenied,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenClause {
    pub error_type: ErrorType,
    pub error_name: String,
    pub body: Vec<Statement>,
}

impl std::error::Error for ParseError {}
