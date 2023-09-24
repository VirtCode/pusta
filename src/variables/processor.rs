use std::ops::Range;
use std::str::FromStr;
use regex::{Regex, Replacer};
use crate::variables::Variable;

/*
% pusta.hostname %

% list test.thing %

% end %

% if amongus %
% else %
% end %
 */

pub const PATTERN_EXPRESSION: &str = r"%\s*([^%]+)\s*%";
pub const PATTERN_TOKEN: &str = r"\s*([^\s]+)\s*";

pub const EXPRESSION_PATTERN: &str = r"%(?:\s*([^\s%]+)\s*)+%";

pub const VARIABLE_PATTERN: &str = r"([\w.-]+)";

pub const KEYWORD_LIST: &str = "list";
pub const KEYWORD_IF: &str = "if";
pub const KEYWORD_ELSE: &str = "else";
pub const KEYWORD_END: &str = "end";
pub const KEYWORD_EVAL: &str = "eval";
pub const KEYWORD_INCLUDE: &str = "include";

pub struct Token {
    range: Range<usize>,
    token: TokenType
}

pub enum TokenType {
    // pusta.hostname, theme.background:color-format( variable another.variable )
    Variable {
        name: String,
        modifiers: Vec<(String, Vec<Token>)>
    },
    // !if, !else, !list, !end
    Keyword {
        word: String
    },
    // "some string", -12, 2304.1, true, false
    Literal {
        value: TokenLiteral
    }
}

pub enum TokenLiteral {
    String(String),
    Number(f64),
    Boolean(bool)
}

pub struct VariableError {
    title: String,
    primary: (Range<usize>, String),
    secondary: Vec<(Range<usize>, String)>,
    summary: String
}

pub fn process_variables(input: &str, variables: Variable, expression_regex: Option<String>) -> anyhow::Result<String> {
    let token = Regex::new(PATTERN_TOKEN).expect("predefined regex pattern did not compile");
    let variable = Regex::new(VARIABLE_PATTERN).expect("predefined regex pattern did not compile");

    // possibly custom expression regex
    let expression = if let Some(string) = expression_regex {
        Regex::new(&string)?
    } else {
        Regex::new(PATTERN_EXPRESSION).expect("predefined regex pattern did not compile")
    };

    let mut result = String::new();

    let mut last: usize = 0;
    for expressions in expression.captures_iter(input) {
        let sequence = expressions.get(0).unwrap();
        let tokens = expressions.get(1).unwrap().as_str();

        result += &input[last..sequence.start()];
        last = sequence.end();

        let variable = variables.find(token.captures(tokens).unwrap().get(1).unwrap().as_str()).unwrap();
        if let Variable::Value(val) = variable {
            result += val;
        }
    }
    result += &input[last..];

    Ok(result)
}

const PATTERN_KEYWORD: &str = r"!([A-Za-z]+\s+)";
const PATTERN_LITERAL_BOOLEAN: &str = r"([A-Za-z]+)\s+";
const PATTERN_LITERAL_NUMBER: &str = r"(-?[0-9]+(?:\.[0-9]+)?)\s+";
const PATTERN_LITERAL_STRING: &str = r#""(.*[^\\])"\s+"#;
const PATTERN_VARIABLE_NAME: &str = r"([A-Za-z-]+(?:\.[A-Za-z-]+)*)(?:(?:\s+)|(?::))";
const PATTERN_VARIABLE_MODIFIER_NAME: &str = r":([A-Za-z-]+)";
const PATTERN_VARIABLE_MODIFIER_OPEN: &str = r"\(\s*";

pub fn read_token(input: &str, position: usize, keyword_allowed: bool) -> Result<Token, VariableError> {
    let regex_keyword = Regex::new(PATTERN_KEYWORD).expect("failed to compile regex");
    let regex_literal_boolean = Regex::new(PATTERN_LITERAL_BOOLEAN).expect("failed to compile regex");
    let regex_literal_number = Regex::new(PATTERN_LITERAL_NUMBER).expect("failed to compile regex");
    let regex_literal_string= Regex::new(PATTERN_LITERAL_STRING).expect("failed to compile regex");
    let regex_variable_name = Regex::new(PATTERN_VARIABLE_NAME).expect("failed to compile regex");
    let regex_variable_modifier_name = Regex::new(PATTERN_VARIABLE_MODIFIER_NAME).expect("failed to compile regex");
    let regex_variable_modifier_open = Regex::new(PATTERN_VARIABLE_MODIFIER_OPEN).expect("failed to compile regex");

    // Keyword
    if let Some(keyword) = regex_keyword.captures_at(input, position) {
        return Ok(Token {
            range: keyword.get(0).expect("regex should have group").range(),
            token: TokenType::Keyword { word: keyword.get(1).expect("regex should have group").into() }
        })
    }

    // Literals
    if let Some(boolean) = regex_literal_boolean.captures_at(input, position) {
        let text = boolean.get(1).expect("regex should have group").as_str();

        if text.eq_ignore_ascii_case("true") || text.eq_ignore_ascii_case("yes") {
            return Ok(Token {
                range: boolean.get(0).expect("regex should have group").range(),
                token: TokenType::Literal { value: TokenLiteral::Boolean(true)}
            })
        }
        if text.eq_ignore_ascii_case("false") || text.eq_ignore_ascii_case("no") {
            return Ok(Token {
                range: boolean.get(0).expect("regex should have group").range(),
                token: TokenType::Literal { value: TokenLiteral::Boolean(false)}
            })
        }
    }

    if let Some(number) = regex_literal_number.captures_at(input, position) {
        let m = number.get(1).expect("regex should have group");

        let num = f64::from_str(m.as_str()).map_err(|e| {
            VariableError {
                title: "invalid number literal".into(),
                primary: (m.range(), "is not a valid number".into()),
                secondary: vec![],
                summary: format!("´{}´ is not a valid number:\n  {e}", m.as_str())
            }
        })?;

        return Ok(Token {
            range: number.get(0).expect("regex should have group").range(),
            token: TokenType::Literal { value: TokenLiteral::Number(num) }
        })
    }

    if let Some(string) = regex_literal_string.captures_at(input, position) {
        let s = string.get(1).expect("regex should have group").as_str();

        return Ok(Token {
            range: string.get(0).expect("regex should have group").range(),
            token: TokenType::Literal { value: TokenLiteral::String(
                s.replace("\\n", "\n").replace("\\\"", "\"")
            )}
        })
    }

    // Variables
    if let Some(m) = regex_variable_name.captures_at(input, position) {
        let name = m.get(1).expect("regex should have group");

        let mut pos = name.end();
        while let Some(modifier_capture) = regex_variable_modifier_name.captures_at(input, pos) {
            let modifier_name = modifier_capture.get(1).expect("regex should have group");

            let mut arguments = vec![];
            if let Some(modifier_open_cap) = regex_variable_modifier_open.captures_at(input, modifier_name.end()) {
                let mut token_pos = modifier_open_cap.get(0).expect("regex should have group").end();

                // loop, try read closing brace, if not try read token

            }



        }


    }



    Ok(Token::Keyword { word: "asdf".to_string() })
}



