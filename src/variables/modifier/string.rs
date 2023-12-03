use serde::de::IntoDeserializer;
use crate::variables::modifier::{Modifier, ModifierError};
use crate::variables::modifier::ModifierErrorType::{ParameterAmount, ParameterType, VariableType};
use crate::variables::Value;
use crate::variables::Value::{Boolean, String};

/// This modifier converts a string to upper case
pub struct UpperCaseModifier;
pub const UPPER_CASE_MODIFIER: &str = "case-upper";
impl Modifier for UpperCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        // Expects no parameters
        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))); }

        // Evaluate
        match variable {
            String(s) => {
                Ok(String(s.to_uppercase()))
            }
            _ => { Err(ModifierError::simple(VariableType(String("".into())))) }
        }
    }
}

/// This modifier converts a string to upper case
pub struct LowerCaseModifier;
pub const LOWER_CASE_MODIFIER: &str = "case-lower";
impl Modifier for LowerCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        // Expects no parameters
        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))); }

        // Evaluate
        match variable {
            String(s) => {
                Ok(String(s.to_lowercase()))
            }
            _ => { Err(ModifierError::simple(VariableType(String("".into())))) }
        }
    }
}

/// This modifier checks whether a string contains something else
pub struct ContainsModifier;
pub const CONTAINS_MODIFIER: &str = "contains";
impl Modifier for ContainsModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))); }

        match (variable, &parameters[0]) {
            (String(s), String(c)) => {
                Ok(Boolean(s.contains(c)))
            }
            (String(_), _) => { Err(ModifierError::simple(ParameterType(0, String("".into())))) }
            _ => { Err(ModifierError::simple(VariableType(String("".into())))) }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::variables::modifier::get_modifier;
    use crate::variables::Value::{Boolean, String};

    #[test]
    fn lower() {
        let modifier = get_modifier("case-lower").unwrap();

        // valid
        assert_eq!(String("abcdefghijklmnop".to_string()), modifier.evaluate(String("AbCDeFghIJklMNop".to_string()), vec![]).unwrap());

        // invalid
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(String("asdf".to_string()), vec![Boolean(true)]), Err(_)));
    }

    #[test]
    fn upper() {
        let modifier = get_modifier("case-upper").unwrap();

        // valid
        assert_eq!(String("ABCDEFGHIJKLMNOP".to_string()), modifier.evaluate(String("AbCDeFghIJklMNop".to_string()), vec![]).unwrap());

        // invalid
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(String("asdf".to_string()), vec![Boolean(true)]), Err(_)));
    }

    #[test]
    fn contains() {
        let modifier = get_modifier("contains").unwrap();

        // valid
        assert_eq!(Boolean(true), modifier.evaluate(String("this string contains something".to_string()), vec![String("contains".to_string())]).unwrap());
        assert_eq!(Boolean(false), modifier.evaluate(String("this string contains something".to_string()), vec![String("not this though".to_string())]).unwrap());

        // invalid
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(String("asdf".to_string()), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(String("asdf".to_string()), vec![Boolean(true)]), Err(_)));
    }

}
