mod boolean;
mod string;
mod number;

use number::{ParseNumberModifier, PARSE_NUMBER_MODIFIER};
use regex::Regex;
use string::{CamelCaseModifier, KebabCaseModifier, PascalCaseModifier, ShellexpandModifier, SnakeCaseModifier, CAMEL_CASE_MODIFIER, KEBAB_CASE_MODIFIER, PASCAL_CASE_MODIFIER, SHELLEXPAND_MODIFIER, SNAKE_CASE_MODIFIER};
use crate::variables::{Value, VariableError};
use crate::variables::context::{Expression, ExpressionModifier};
use crate::variables::modifier::boolean::{NOT_MODIFIER, AND_MODIFIER, OR_MODIFIER, IF_MODIFIER, NotModifier, AndModifier, OrModifier, IfModifier};
use crate::variables::modifier::string::{LOWER_CASE_MODIFIER, UPPER_CASE_MODIFIER, CONTAINS_MODIFIER, UpperCaseModifier, LowerCaseModifier, ContainsModifier};
use crate::variables::modifier::ModifierErrorType::{ParameterAmount, ParameterType, VariableType};
use crate::variables::modifier::number::{ADD_MODIFIER, SUBTRACT_MODIFIER, DIVISION_MODIFIER, MULTIPLY_MODIFIER, NEGATIVE_MODIFIER, AddModifier, SubtractModifier, MultiplyModifier, DivisionModifier, NegativeModifier};

/// This trait is a variable modifier
pub trait Modifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError>;
}

/// Represents a modifier error, which can be converted to a variable error
#[derive(Debug)]
pub struct ModifierError {
    error: ModifierErrorType,
    notes: Vec<ModifierErrorNote>
}

impl ModifierError {

    /// Creates a simple modifier error without notes
    fn simple(error: ModifierErrorType) -> Self {
        ModifierError {
            error, notes: vec![],
        }
    }

    /// Creates a modifier error with notes
    fn noted(error: ModifierErrorType, notes: Vec<ModifierErrorNote>) -> Self {
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
#[derive(Debug)]
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
#[derive(Debug)]
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
        LOWER_CASE_MODIFIER => { Box::new(LowerCaseModifier) }
        CAMEL_CASE_MODIFIER => { Box::new(CamelCaseModifier) }
        SNAKE_CASE_MODIFIER => { Box::new(SnakeCaseModifier) }
        PASCAL_CASE_MODIFIER => { Box::new(PascalCaseModifier) }
        KEBAB_CASE_MODIFIER => { Box::new(KebabCaseModifier) }
        CONTAINS_MODIFIER => { Box::new(ContainsModifier) }
        SHELLEXPAND_MODIFIER => { Box::new(ShellexpandModifier) }

        NOT_MODIFIER => { Box::new(NotModifier) }
        AND_MODIFIER => { Box::new(AndModifier) }
        OR_MODIFIER => { Box::new(OrModifier) }
        IF_MODIFIER => { Box::new(IfModifier) }

        ADD_MODIFIER => { Box::new(AddModifier) }
        SUBTRACT_MODIFIER => { Box::new(SubtractModifier) }
        MULTIPLY_MODIFIER => { Box::new(MultiplyModifier) }
        DIVISION_MODIFIER => { Box::new(DivisionModifier) }
        NEGATIVE_MODIFIER => { Box::new(NegativeModifier) }
        PARSE_NUMBER_MODIFIER => { Box::new(ParseNumberModifier) }

        EQ_MODIFIER => { Box::new(EqModifier) }

        FORMAT_COLOR_MODIFIER => { Box::new(FormatColorModifier) }
        _ => { return None }
    })
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

/// This modifier formats rgb colors akin to how date formatters work
struct FormatColorModifier;
const FORMAT_COLOR_MODIFIER: &str = "format-color";
impl Modifier for FormatColorModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        let input = Regex::new("^#?(?<r>[a-fA-F0-9]{2})(?<g>[a-fA-F0-9]{2})(?<b>[a-fA-F0-9]{2})(?<a>[a-fA-F0-9]{2})?$").expect("regex should be compilable");
        let pattern = Regex::new("%(?<f>[XFD])(?<c>[rgba])").expect("regex should be compilable");

        // capture color
        let (r,g,b,a) = if let Value::String(s) = variable {
            if let Some(m) = input.captures(&s) {
                (m.name("r").and_then(|m| u8::from_str_radix(m.as_str(), 16).ok()).expect("regex matched"),
                 m.name("g").and_then(|m| u8::from_str_radix(m.as_str(), 16).ok()).expect("regex matched"),
                 m.name("b").and_then(|m| u8::from_str_radix(m.as_str(), 16).ok()).expect("regex matched"),
                 m.name("a").and_then(|m| u8::from_str_radix(m.as_str(), 16).ok()).unwrap_or(255u8))
            } else {
                return Err( ModifierError::noted(VariableType(Value::String("".into())), vec![ModifierErrorNote::Variable("string must be a color of format (#)RRGGBB(AA)".into())]))
            }
        } else {
            return Err( ModifierError::simple(VariableType(Value::String("".into()))))
        };

        // process format
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))) }
        if let Value::String(format) = parameters.get(0).expect("already tested") {
            let mut references = vec![];

            for capture in pattern.captures_iter(format) {
                let component = capture.name("c").expect("regex matched").as_str();
                let format = capture.name("f").expect("regex matched").as_str();

                let component = match component {
                    "r" => { r },
                    "g" => { g },
                    "b" => { b },
                    "a" => { a },
                    _ => { unreachable!("regex only allows r g b or a") }
                };

                let result = match format {
                    "X" => { format!("{component:02x}") }
                    "D" => { format!("{component}") }
                    "F" => { format!("{:.3}", component as f32 / 255f32) }
                    _ => { unreachable!("regex only allows X D and F") }
                };

                references.push((capture.get(0).expect("regex matched").range(), result));
            }

            // assemble result string
            let mut formatted = String::new();
            let mut index = 0;

            for (range, string) in references {
                formatted += &format[index..range.start];
                index = range.end;

                formatted += &string;
            }

            formatted += &format[index..format.len()];

            Ok(Value::String(formatted))
        } else {
            Err(ModifierError::simple(ParameterType(0, Value::String("".into()))))
        }
    }
}
