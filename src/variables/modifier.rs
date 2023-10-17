use crate::variables::{Value, VariableError};
use crate::variables::context::{Expression, ExpressionModifier};
use crate::variables::modifier::ModifierErrorType::{ParameterAmount, ParameterType, VariableType};

/// This trait is a variable modifier
trait Modifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError>;
}

/// Represents a modifier error, which can be converted to a variable error
pub struct ModifierError {
    error: ModifierErrorType,
    notes: Vec<ModifierErrorNote>
}

impl ModifierError {

    /// Creates a simple modifier error without notes
    pub fn simple(error: ModifierErrorType) -> Self {
        ModifierError {
            error, notes: vec![],
        }
    }

    /// Creates a modifier error with notes
    pub fn noted(error: ModifierErrorType, notes: Vec<ModifierErrorNote>) -> Self {
        ModifierError {
            error, notes
        }
    }

    /// Converts the modifier to a more common variable error
    pub fn to_general(&self, expr: &Expression, modifier: &ExpressionModifier) -> VariableError {
        let title = match &self.error {
            ParameterAmount(_) => { "invalid amount of modifier parameters" }
            ParameterType(_, _) => { "invalid parameter type for modifier"}
            VariableType(_) => { "variable has invalid type for modifier" }
            ModifierErrorType::Runtime(_) => { "modifier runtime error" }
        }.to_owned();

        let primary = match &self.error {
            ParameterAmount(u) => {
                (modifier.name_range.clone(), format!("modifier expects {u} parameters, found {}", modifier.parameters.len()))
            }
            ParameterType(index, t) => {
                let parameter = modifier.parameters.get(*index).expect("parameter implementation broken");
                (parameter.content_range.clone(), format!("modifier expected parameter as type {}", t.type_name()))
            }
            VariableType(t) => {
                (expr.content_range.clone(), format!("modifier expected base as type {}", t.type_name()))
            }
            ModifierErrorType::Runtime(s) => {
                (modifier.name_range.clone(), s.clone())
            }
        };

        let secondary = self.notes.iter().map(|note| {
            match note {
                ModifierErrorNote::Variable(s) => {
                    (expr.content_range.start..(modifier.name_range.start - 1), s.clone())
                }
                ModifierErrorNote::Parameter(i, s) => {
                    let parameter = modifier.parameters.get(*i).expect("parameter implementation broken");
                    (parameter.content_range.clone(), s.clone())
                }
            }
        }).collect();

        let summary = match &self.error {
            ParameterAmount(_) => { "invalid amount of parameters provided to the modifier"}
            ParameterType(_, _) => { "the shown parameter provided is of the wrong type" }
            VariableType(_) => { "the base for the modifier is of the wrong type" }
            ModifierErrorType::Runtime(s) => { s.as_str() }
        }.to_owned();

        VariableError { title, primary, secondary, summary }
    }
}

/// Is the general error type for a modifier error
enum ModifierErrorType {
    /// Invalid parameter amount supplied, expected amount is
    ParameterAmount(usize),
    /// Invalid parameter type supplied, at index, expected type is
    ParameterType(usize, Value),
    /// Invalid variable type supplied, expected type is
    VariableType(Value),
    /// Runtime error occurred, detailed message is
    Runtime(String)
}

/// Adds additional detail to a modifier error
enum ModifierErrorNote {
    /// Note about the variable itself
    Variable(String),
    /// Note about the parameter at index
    Parameter(usize, String)
}

pub fn evaluate_modifier(modifier: &str, variable: Value, parameters: Vec<Value>) -> Option<Result<Value, ModifierError>> {
    get_modifier(modifier).map(|m| m.evaluate(variable, parameters))
}

pub fn get_modifier(name: &str) -> Option<Box<dyn Modifier>> {
    Some(match name {
        UPPER_CASE_MODIFIER => { Box::new(UpperCaseModifier) }
        EQ_MODIFIER => { Box::new(EqModifier) }
        _ => { return None }
    })
}

/// This modifier converts a string to upper case
struct UpperCaseModifier;
const UPPER_CASE_MODIFIER: &str = "case-upper";
impl Modifier for UpperCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        // Expects no parameters
        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))); }

        // Evaluate
        match variable {
            Value::String(s) => {
                Ok(Value::String(s.to_uppercase()))
            }
            _ => { Err(ModifierError::simple(VariableType(Value::String("".into())))) }
        }
    }
}

/// This modifier compares the variable and a parameter
struct EqModifier;
const EQ_MODIFIER: &str = "eq";
impl Modifier for EqModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {

        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))) }
        let parameter = parameters.get(0).expect("checked length already");

        Ok(Value::Boolean(match (variable, parameter) {

            (Value::String(s1), Value::String(s2)) => { s1.eq(s2) }
            (Value::Number(n1), Value::Number(n2)) => { n1.eq(n2) }
            (Value::Boolean(b1), Value::Boolean(b2)) => { b1.eq(b2) }

            (var, _) => return Err(
               ModifierError::noted(ParameterType(0, var), vec![ModifierErrorNote::Variable("should be of the same type as the variable".into())])
            )
        }))
    }
}