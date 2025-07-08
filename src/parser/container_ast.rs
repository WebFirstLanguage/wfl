use super::ast::{Argument, Expression, Parameter, Statement, Type};
use std::fmt;

/// Represents the visibility of a container member (property or method)
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Public // Default to public visibility
    }
}

/// Represents a validation rule for a property
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub parameters: Vec<Expression>,
    pub line: usize,
    pub column: usize,
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

/// Container-related statements to be added to the Statement enum
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatement {
    /// Container definition statement
    ContainerDefinition {
        name: String,
        extends: Option<String>,
        implements: Vec<String>,
        properties: Vec<PropertyDefinition>,
        methods: Vec<Statement>, // ActionDefinition statements
        events: Vec<EventDefinition>,
        static_properties: Vec<PropertyDefinition>,
        static_methods: Vec<Statement>, // ActionDefinition statements
        line: usize,
        column: usize,
    },
    
    /// Container instantiation statement
    ContainerInstantiation {
        container_type: String,
        instance_name: String,
        arguments: Vec<Argument>,
        property_initializers: Vec<PropertyInitializer>,
        line: usize,
        column: usize,
    },
    
    /// Interface definition statement
    InterfaceDefinition {
        name: String,
        extends: Vec<String>,
        required_actions: Vec<ActionSignature>,
        line: usize,
        column: usize,
    },
    
    /// Event definition statement
    EventDefinition {
        name: String,
        parameters: Vec<Parameter>,
        line: usize,
        column: usize,
    },
    
    /// Event trigger statement
    EventTrigger {
        name: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
    
    /// Event handler statement
    EventHandler {
        event_source: Expression,
        event_name: String,
        handler_body: Vec<Statement>,
        line: usize,
        column: usize,
    },
    
    /// Parent method call statement
    ParentMethodCall {
        method_name: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
}

/// Represents an event definition in a container
#[derive(Debug, Clone, PartialEq)]
pub struct EventDefinition {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub line: usize,
    pub column: usize,
}

/// Container-related expressions to be added to the Expression enum
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerExpression {
    /// Static member access expression
    StaticMemberAccess {
        container: String,
        member: String,
        line: usize,
        column: usize,
    },
    
    /// Method call expression
    MethodCall {
        object: Box<Expression>,
        method: String,
        arguments: Vec<Argument>,
        line: usize,
        column: usize,
    },
}

/// Container-related types to be added to the Type enum
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerType {
    /// Container type
    Container(String),
    
    /// Container instance type
    ContainerInstance(String),
    
    /// Interface type
    Interface(String),
}