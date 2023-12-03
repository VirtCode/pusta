use crate::variables::modifier::{Modifier, ModifierError};
use crate::variables::modifier::ModifierErrorType::{ParameterAmount, ParameterType, VariableType};
use crate::variables::Value;
use crate::variables::Value::Boolean;

/// This modifier inverts a boolean
pub struct NotModifier;
pub const NOT_MODIFIER: &str = "not";
impl Modifier for NotModifier {
    fn evaluate(&self, variable: Value, mut parameters: Vec<Value>) -> Result<Value, ModifierError> {

        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))) }

        if let Boolean(value) = variable {
            Ok(Boolean(!value))
        } else {
            Err(ModifierError::simple(VariableType(Boolean(false))))
        }
    }
}

/// This modifier is a boolean or
pub struct OrModifier;
pub const OR_MODIFIER: &str = "or";
impl Modifier for OrModifier {
    fn evaluate(&self, variable: Value, mut parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))) }

        match (variable, &parameters[0]) {
            (Boolean(a), Boolean(b)) => { Ok(Boolean(a || *b)) }

            (Boolean(_), _) => { Err(ModifierError::simple(ParameterType(0, Boolean(false)))) }
            _ => { Err(ModifierError::simple(VariableType(Boolean(false)))) }
        }
    }
}

/// this modifier is a boolean and
pub struct AndModifier;
pub const AND_MODIFIER: &str = "and";
impl Modifier for AndModifier {
    fn evaluate(&self, variable: Value, mut parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))) }

        match (variable, &parameters[0]) {
            (Boolean(a), Boolean(b)) => { Ok(Boolean(a && *b)) }

            (Boolean(_), _) => { Err(ModifierError::simple(ParameterType(0, Boolean(false)))) }
            _ => { Err(ModifierError::simple(VariableType(Boolean(false)))) }
        }
    }
}

/// This modifier acts as a ternary operator
pub struct IfModifier;
pub const IF_MODIFIER: &str = "if";
impl Modifier for IfModifier {
    fn evaluate(&self, variable: Value, mut parameters: Vec<Value>) -> Result<Value, ModifierError> {

        if parameters.len() != 2 { return Err(ModifierError::simple(ParameterAmount(2))) }

        if let Boolean(condition) = variable {
            if condition { Ok(parameters.remove(0)) }
            else { Ok(parameters.remove(1)) }
        } else {
            Err(ModifierError::simple(VariableType(Boolean(false))))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::variables::modifier::get_modifier;
    use crate::variables::Value::{Boolean, Number, String};

    #[test]
    fn not() {
        let modifier = get_modifier("not").unwrap();

        // valid inputs
        assert_eq!(Boolean(true), modifier.evaluate(Boolean(false), vec![]).unwrap());
        assert_eq!(Boolean(false), modifier.evaluate(Boolean(true), vec![]).unwrap());

        // invalid inputs
        assert!(matches!(modifier.evaluate(Number(-1.0), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(true), vec![Boolean(false)]), Err(_)));
    }

    #[test]
    fn or() {
        let modifier = get_modifier("or").unwrap();

        // valid inputs
        assert_eq!(Boolean(true), modifier.evaluate(Boolean(false), vec![Boolean(true)]).unwrap());
        assert_eq!(Boolean(true), modifier.evaluate(Boolean(true), vec![Boolean(false)]).unwrap());
        assert_eq!(Boolean(true), modifier.evaluate(Boolean(true), vec![Boolean(true)]).unwrap());
        assert_eq!(Boolean(false), modifier.evaluate(Boolean(false), vec![Boolean(false)]).unwrap());

        // invalid inputs
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Number(-1.0), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(false), vec![Number(1.0)]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(false), vec![Boolean(false), Boolean(true)]), Err(_)));
    }

    #[test]
    fn and() {
        let modifier = get_modifier("and").unwrap();

        // valid inputs
        assert_eq!(Boolean(false), modifier.evaluate(Boolean(false), vec![Boolean(true)]).unwrap());
        assert_eq!(Boolean(false), modifier.evaluate(Boolean(true), vec![Boolean(false)]).unwrap());
        assert_eq!(Boolean(true), modifier.evaluate(Boolean(true), vec![Boolean(true)]).unwrap());
        assert_eq!(Boolean(false), modifier.evaluate(Boolean(false), vec![Boolean(false)]).unwrap());

        // invalid inputs
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Number(-1.0), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(false), vec![Number(1.0)]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(false), vec![Boolean(false), Boolean(true)]), Err(_)));
    }

    #[test]
    fn if_mod() {
        let modifier = get_modifier("if").unwrap();

        // valid inputs
        assert_eq!(String("a".to_string()), modifier.evaluate(Boolean(false), vec![Number(1.0), String("a".to_string())]).unwrap());
        assert_eq!(Number(1.0), modifier.evaluate(Boolean(true), vec![Number(1.0), String("a".to_string())]).unwrap());

        // invalid inputs
        assert!(matches!(modifier.evaluate(Number(-1.0), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(true), vec![Boolean(false)]), Err(_)));
        assert!(matches!(modifier.evaluate(Number(-1.0), vec![Boolean(false)]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(true), vec![Boolean(false), Boolean(false), Boolean(false)]), Err(_)));
    }
}