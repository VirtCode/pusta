use crate::variables::modifier::{Modifier, ModifierError, ModifierErrorNote, ModifierErrorType};
use crate::variables::modifier::ModifierErrorType::{ParameterAmount, ParameterType, VariableType};
use crate::variables::Value;
use crate::variables::Value::{Number, String};

/// This modifier adds two numbers together
pub struct AddModifier;
pub const ADD_MODIFIER: &str = "add";
impl Modifier for AddModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))); }

        match (variable, &parameters[0]) {
            (Number(a), Number(b)) => {
                Ok(Number(a + b))
            }
            (Number(_), _) => { Err(ModifierError::simple(ParameterType(0, Number(0.0)))) }
            _ => { Err(ModifierError::simple(VariableType(Number(0.0)))) }
        }
    }
}

/// This modifier subtracts two numbers together
pub struct SubtractModifier;
pub const SUBTRACT_MODIFIER: &str = "sub";
impl Modifier for SubtractModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))); }

        match (variable, &parameters[0]) {
            (Number(a), Number(b)) => {
                Ok(Number(a - b))
            }
            (Number(_), _) => { Err(ModifierError::simple(ParameterType(0, Number(0.0)))) }
            _ => { Err(ModifierError::simple(VariableType(Number(0.0)))) }
        }
    }
}

/// This modifier multiplies two numbers together
pub struct MultiplyModifier;
pub const MULTIPLY_MODIFIER: &str = "mul";
impl Modifier for MultiplyModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))); }

        match (variable, &parameters[0]) {
            (Number(a), Number(b)) => {
                Ok(Number(a * b))
            }
            (Number(_), _) => { Err(ModifierError::simple(ParameterType(0, Number(0.0)))) }
            _ => { Err(ModifierError::simple(VariableType(Number(0.0)))) }
        }
    }
}

/// This modifier divides two numbers together
pub struct DivisionModifier;
pub const DIVISION_MODIFIER: &str = "div";
impl Modifier for DivisionModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { return Err(ModifierError::simple(ParameterAmount(1))); }


        match (variable, &parameters[0]) {
            (Number(a), Number(b)) => {
                if *b == 0.0 {
                    Err(ModifierError::noted(ModifierErrorType::Runtime("cannot divide by 0".into()), vec![ModifierErrorNote::Parameter(0, "evaluates to zero".into())]))
                } else {
                    Ok(Number(a / b))
                }
            }
            (Number(_), _) => { Err(ModifierError::simple(ParameterType(0, Number(0.0)))) }
            _ => { Err(ModifierError::simple(VariableType(Number(0.0)))) }
        }
    }
}

/// This modifier takes the negative of a modifier
pub struct NegativeModifier;
pub const NEGATIVE_MODIFIER: &str = "neg";
impl Modifier for NegativeModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {

        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))) }

        if let Number(value) = variable {
            Ok(Number(-value))
        } else {
            Err(ModifierError::simple(VariableType(Number(0.0))))
        }
    }
}

/// This modifier parses a number from a string
pub struct ParseNumberModifier;
pub const PARSE_NUMBER_MODIFIER: &str = "parsenum";
impl Modifier for ParseNumberModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))) }

        if let String(value) = variable {
            if let Ok(val) = value.parse() {
                Ok(Number(val))
            } else {
                Err( ModifierError::noted(VariableType(Value::String("".into())), vec![ModifierErrorNote::Variable("string can not be parsed as a number".into())]))
            }
        } else {
            Err(ModifierError::simple(VariableType(String("".into()))))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::variables::modifier::{get_modifier, Modifier};
    use crate::variables::Value::{Boolean, Number, String};

    fn check_two_args(modifier: &Box<dyn Modifier>) {
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Number(0.0), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Number(0.0), vec![Number(0.0), Number(0.0)]), Err(_)));
    }

    #[test]
    fn add() {
        let modifier = get_modifier("add").unwrap();

        // valid inputs
        assert_eq!(Number(42.0), modifier.evaluate(Number(29.0), vec![Number(13.0)]).unwrap());

        // invalid inputs
        check_two_args(&modifier);
    }

    #[test]
    fn subtract() {
        let modifier = get_modifier("sub").unwrap();

        // valid inputs
        assert_eq!(Number(42.0), modifier.evaluate(Number(63.0), vec![Number(21.0)]).unwrap());

        // invalid inputs
        check_two_args(&modifier);
    }

    #[test]
    fn multiply() {
        let modifier = get_modifier("mul").unwrap();

        // valid inputs
        assert_eq!(Number(42.0), modifier.evaluate(Number(10.5), vec![Number(4.0)]).unwrap());

        // invalid inputs
        check_two_args(&modifier);
    }

    #[test]
    fn divide() {
        let modifier = get_modifier("div").unwrap();

        // valid inputs
        assert_eq!(Number(42.0), modifier.evaluate(Number(567.0), vec![Number(13.5)]).unwrap());

        // invalid inputs
        check_two_args(&modifier);
        assert!(matches!(modifier.evaluate(Number(42.0), vec![Number(0.0)]), Err(_)));
    }

    #[test]
    fn negative() {
        let modifier = get_modifier("neg").unwrap();

        // valid inputs
        assert_eq!(Number(42.0), modifier.evaluate(Number(-42.0), vec![]).unwrap());

        // invalid inputs
        assert!(matches!(modifier.evaluate(Number(0.0), vec![Number(0.0)]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
    }

    #[test]
    fn parse() {
        let modifier = get_modifier("parsenum").unwrap();

        // valid inputs
        assert_eq!(Number(-100.7828), modifier.evaluate(String("-100.7828".into()), vec![]).unwrap());

        // invalid inputs
        assert!(matches!(modifier.evaluate(String("amogus not a number".into()), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(String("".into()), vec![Number(0.0)]), Err(_)));
    }

}
