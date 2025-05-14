use crate::variables::modifier::ModifierErrorType::{ParameterAmount, ParameterType, VariableType};
use crate::variables::modifier::{Modifier, ModifierError};
use crate::variables::Value;
use crate::variables::Value::{Boolean, String};

use super::ModifierErrorNote;

/// This modifier converts a string to upper case
pub struct UpperCaseModifier;
pub const UPPER_CASE_MODIFIER: &str = "case-upper";
impl Modifier for UpperCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        // Expects no parameters
        if !parameters.is_empty() {
            return Err(ModifierError::simple(ParameterAmount(0)));
        }

        // Evaluate
        match variable {
            String(s) => Ok(String(s.to_uppercase())),
            _ => Err(ModifierError::simple(VariableType(String("".into())))),
        }
    }
}

/// This modifier converts a string to upper case
pub struct LowerCaseModifier;
pub const LOWER_CASE_MODIFIER: &str = "case-lower";
impl Modifier for LowerCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        // Expects no parameters
        if !parameters.is_empty() {
            return Err(ModifierError::simple(ParameterAmount(0)));
        }

        // Evaluate
        match variable {
            String(s) => Ok(String(s.to_lowercase())),
            _ => Err(ModifierError::simple(VariableType(String("".into())))),
        }
    }
}

/// Turn a string into it's case segments based on an origin case
///
/// The segments are returned as a vector of lower-cased strings
fn into_case_segments(
    s: std::string::String,
    case: &str,
) -> Result<Vec<std::string::String>, std::string::String> {
    let segments = match case {
        KEBAB_CASE_MODIFIER => s.to_lowercase().split("-").map(std::string::String::from).collect(),
        SNAKE_CASE_MODIFIER => s.to_lowercase().split("_").map(std::string::String::from).collect(),
        CAMEL_CASE_MODIFIER | PASCAL_CASE_MODIFIER => {
            let mut c = s.chars();
            // uppercase first letter to transform camelCase to PascalCase
            let s = match c.next() {
                None => std::string::String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            };

            let mut previous_upper = true;
            let mut start = 0;
            let mut segments = Vec::new();

            if s.len() == 1 {
                return Ok(vec![s.to_lowercase()]);
            }

            s[1..].chars().enumerate().for_each(|(i, c)| {
                if c.is_uppercase() {
                    segments.push(s[start..i+1].to_lowercase());
                    start = i + 1;
                } else if c.is_lowercase() && previous_upper {
                    previous_upper = false;
                }
                if i == s.len() - 2 {
                    segments.push(s[start..i+2].to_lowercase())
                }
            });
            segments
        }
        _ => Err(format!("case '{case}' is not a valid origin case"))?,
    };
    Ok(segments)
}

pub struct CamelCaseModifier;
pub const CAMEL_CASE_MODIFIER: &str = "case-camel";
impl Modifier for CamelCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 {
            return Err(ModifierError::simple(ParameterAmount(1)));
        }

        match (variable, &parameters[0]) {
            (String(s), String(c)) => {
                let mut segments = into_case_segments(s, c.as_str()).map_err(|err| {
                    ModifierError::noted(
                        ParameterType(0, String("".into())),
                        vec![ModifierErrorNote::Parameter(0, err)],
                    )
                })?.into_iter();
                let first = segments.next().unwrap_or_default();
                let rest = segments.map(|seg| {
                    let mut c = seg.chars();
                    match c.next() {
                        None => std::string::String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                }).collect::<Vec<_>>().join("");
                Ok(String(first + rest.as_str()))
            }
            (String(_), _) => Err(ModifierError::simple(ParameterType(0, String("".into())))),
            _ => Err(ModifierError::simple(VariableType(String("".into())))),
        }
    }
}

pub struct SnakeCaseModifier;
pub const SNAKE_CASE_MODIFIER: &str = "case-snake";
impl Modifier for SnakeCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { Err(ModifierError::simple(ParameterAmount(1)))? }

        match (variable, &parameters[0]) {
            (String(s), String(c)) => {
                let segments = into_case_segments(s, c.as_str()).map_err(|err| {
                    ModifierError::noted(
                        ParameterType(0, String("".into())),
                        vec![ModifierErrorNote::Parameter(0, err)],
                    )
                })?;
                Ok(String(segments.join("_")))
            }
            (String(_), _) => Err(ModifierError::simple(ParameterType(0, String("".into())))),
            _ => Err(ModifierError::simple(VariableType(String("".into())))),
        }
    }
}

pub struct PascalCaseModifier;
pub const PASCAL_CASE_MODIFIER: &str = "case-pascal";
impl Modifier for PascalCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { Err(ModifierError::simple(ParameterAmount(1)))? }

        match (variable, &parameters[0]) {
            (String(s), String(c)) => {
                let segments = into_case_segments(s, c.as_str()).map_err(|err| {
                    ModifierError::noted(
                        ParameterType(0, String("".into())),
                        vec![ModifierErrorNote::Parameter(0, err)],
                    )
                })?;
                let str = segments.iter().map(|seg| {
                    let mut c = seg.chars();
                    match c.next() {
                        None => std::string::String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                }).collect::<Vec<_>>().join("");
                Ok(String(str))
            }
            (String(_), _) => Err(ModifierError::simple(ParameterType(0, String("".into())))),
            _ => Err(ModifierError::simple(VariableType(String("".into())))),
        }
    }
}

pub struct KebabCaseModifier;
pub const KEBAB_CASE_MODIFIER: &str = "case-kebab";
impl Modifier for KebabCaseModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if parameters.len() != 1 { Err(ModifierError::simple(ParameterAmount(1)))? }

        match (variable, &parameters[0]) {
            (String(s), String(c)) => {
                let segments = into_case_segments(s, c.as_str()).map_err(|err| {
                    ModifierError::noted(
                        ParameterType(0, String("".into())),
                        vec![ModifierErrorNote::Parameter(0, err)],
                    )
                })?;
                Ok(String(segments.join("-")))
            }
            (String(_), _) => Err(ModifierError::simple(ParameterType(0, String("".into())))),
            _ => Err(ModifierError::simple(VariableType(String("".into())))),
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

/// This modifier checks whether a string contains something else
pub struct ShellexpandModifier;
pub const SHELLEXPAND_MODIFIER: &str = "tilde";
impl Modifier for ShellexpandModifier {
    fn evaluate(&self, variable: Value, parameters: Vec<Value>) -> Result<Value, ModifierError> {
        if !parameters.is_empty() { return Err(ModifierError::simple(ParameterAmount(0))); }

        if let String(s) = variable {
            Ok(String(shellexpand::tilde(&s).to_string()))
        } else {
            Err(ModifierError::simple(VariableType(String("".into()))))
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
    fn camel() {
        let modifier = get_modifier("case-camel").unwrap();

        assert_eq!(String("thisIsCamelCase".to_string()), modifier.evaluate(String("thisIsCamelCase".to_string()), vec![String("case-camel".to_string())]).unwrap());
        assert_eq!(String("thisIsCamelCase".to_string()), modifier.evaluate(String("this_is_camel_case".to_string()), vec![String("case-snake".to_string())]).unwrap());
        assert_eq!(String("thisIsCamelCase".to_string()), modifier.evaluate(String("ThisIsCamelCase".to_string()), vec![String("case-pascal".to_string())]).unwrap());
        assert_eq!(String("thisIsCamelCase".to_string()), modifier.evaluate(String("this-is-camel-case".to_string()), vec![String("case-kebab".to_string())]).unwrap());

        assert_eq!(String("p".to_string()), modifier.evaluate(String("P".to_string()), vec![String("case-pascal".to_string())]).unwrap());
        assert_eq!(String("pP".to_string()), modifier.evaluate(String("PP".to_string()), vec![String("case-pascal".to_string())]).unwrap());

        // require exactly one parameter
        assert!(modifier.evaluate(String("thisIsCamelCase".to_string()), vec![]).is_err());
        assert!(modifier.evaluate(String("thisIsCamelCase".to_string()), vec![String("".into()), String("".into())]).is_err());
        // parameter needs to be a well-known case type
        assert!(modifier.evaluate(String("thisIsCamelCase".to_string()), vec![String("".into())]).is_err());
    }

    #[test]
    fn snake() {
        let modifier = get_modifier("case-snake").unwrap();

        assert_eq!(String("this_is_snake_case".to_string()), modifier.evaluate(String("thisIsSnakeCase".to_string()), vec![String("case-camel".to_string())]).unwrap());
        assert_eq!(String("this_is_snake_case".to_string()), modifier.evaluate(String("this_is_snake_case".to_string()), vec![String("case-snake".to_string())]).unwrap());
        assert_eq!(String("this_is_snake_case".to_string()), modifier.evaluate(String("ThisIsSnakeCase".to_string()), vec![String("case-pascal".to_string())]).unwrap());
        assert_eq!(String("this_is_snake_case".to_string()), modifier.evaluate(String("this-is-snake-case".to_string()), vec![String("case-kebab".to_string())]).unwrap());

        assert_eq!(String("p".to_string()), modifier.evaluate(String("P".to_string()), vec![String("case-pascal".to_string())]).unwrap());
        assert_eq!(String("p_p".to_string()), modifier.evaluate(String("PP".to_string()), vec![String("case-pascal".to_string())]).unwrap());

        // require exactly one parameter
        assert!(modifier.evaluate(String("this_is_snake_case".to_string()), vec![]).is_err());
        assert!(modifier.evaluate(String("this_is_snake_case".to_string()), vec![String("".into()), String("".into())]).is_err());
        // parameter needs to be a well-known case type
        assert!(modifier.evaluate(String("this_is_snake_case".to_string()), vec![String("".into())]).is_err());
    }

    #[test]
    fn pascal() {
        let modifier = get_modifier("case-pascal").unwrap();

        assert_eq!(String("ThisIsPascalCase".to_string()), modifier.evaluate(String("thisIsPascalCase".to_string()), vec![String("case-camel".to_string())]).unwrap());
        assert_eq!(String("ThisIsPascalCase".to_string()), modifier.evaluate(String("this_is_pascal_case".to_string()), vec![String("case-snake".to_string())]).unwrap());
        assert_eq!(String("ThisIsPascalCase".to_string()), modifier.evaluate(String("ThisIsPascalCase".to_string()), vec![String("case-pascal".to_string())]).unwrap());
        assert_eq!(String("ThisIsPascalCase".to_string()), modifier.evaluate(String("this-is-pascal-case".to_string()), vec![String("case-kebab".to_string())]).unwrap());

        assert_eq!(String("P".to_string()), modifier.evaluate(String("p".to_string()), vec![String("case-camel".to_string())]).unwrap());
        assert_eq!(String("PP".to_string()), modifier.evaluate(String("pP".to_string()), vec![String("case-camel".to_string())]).unwrap());

        // require exactly one parameter
        assert!(modifier.evaluate(String("ThisIsPascalCase".to_string()), vec![]).is_err());
        assert!(modifier.evaluate(String("ThisIsPascalCase".to_string()), vec![String("".into()), String("".into())]).is_err());
        // parameter needs to be a well-known case type
        assert!(modifier.evaluate(String("ThisIsPascalCase".to_string()), vec![String("".into())]).is_err());
    }

    #[test]
    fn kebab() {
        let modifier = get_modifier("case-kebab").unwrap();

        assert_eq!(String("this-is-kebab-case".to_string()), modifier.evaluate(String("thisIsKebabCase".to_string()), vec![String("case-camel".to_string())]).unwrap());
        assert_eq!(String("this-is-kebab-case".to_string()), modifier.evaluate(String("this_is_kebab_case".to_string()), vec![String("case-snake".to_string())]).unwrap());
        assert_eq!(String("this-is-kebab-case".to_string()), modifier.evaluate(String("ThisIsKebabCase".to_string()), vec![String("case-pascal".to_string())]).unwrap());
        assert_eq!(String("this-is-kebab-case".to_string()), modifier.evaluate(String("this-is-kebab-case".to_string()), vec![String("case-kebab".to_string())]).unwrap());

        assert_eq!(String("p".to_string()), modifier.evaluate(String("p".to_string()), vec![String("case-camel".to_string())]).unwrap());
        assert_eq!(String("p-p".to_string()), modifier.evaluate(String("pP".to_string()), vec![String("case-camel".to_string())]).unwrap());

        // require exactly one parameter
        assert!(modifier.evaluate(String("this-is-kebab-case".to_string()), vec![]).is_err());
        assert!(modifier.evaluate(String("this-is-kebab-case".to_string()), vec![String("".into()), String("".into())]).is_err());
        // parameter needs to be a well-known case type
        assert!(modifier.evaluate(String("this-is-kebab-case".to_string()), vec![String("".into())]).is_err());
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

    #[test]
    fn tilde() {
        let modifier = get_modifier("tilde").unwrap();

        // valid
        let path = "~/games/amogus";
        assert_eq!(String(shellexpand::tilde(path).to_string()), modifier.evaluate(String(path.to_string()), vec![]).unwrap());

        // invalid
        assert!(matches!(modifier.evaluate(Boolean(false), vec![]), Err(_)));
        assert!(matches!(modifier.evaluate(String("asdf".to_string()), vec![Boolean(true)]), Err(_)));
    }
}
