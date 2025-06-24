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
    CloseFileStatement {
        file: Expression,
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
        error_name: String,
        when_block: Vec<Statement>,
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

impl std::error::Error for ParseError {}
