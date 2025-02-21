use std::ops::Range;
use std::str::FromStr;
use lazy_regex::{Lazy, lazy_regex, regex};
use regex::Regex;
use crate::variables::VariableError;

/// Matches a keyword, e.g. !list
static REGEX_KEYWORD: Lazy<Regex> = lazy_regex!(r"^\s*!([A-Za-z]+)\s*");

/// Matches a boolean literal, e.g. true
const REGEX_LITERAL_BOOLEAN: Lazy<Regex> = lazy_regex!(r"^\s*([A-Za-z]+)\s*");
/// Matches a number literal, e.g. -1345.2768
const REGEX_LITERAL_NUMBER: Lazy<Regex> = lazy_regex!(r"^\s*(-?[0-9]+(?:\.[0-9]+)?)\s*");
/// Matches a string literal, e.g. "a string literal with inside \"quotes\" and newlines \n"
const REGEX_LITERAL_STRING: Lazy<Regex> = lazy_regex!(r#"^\s*"(.*?[^\\])"\s*"#);

/// Matches a variable name, e.g. theme.my-fancy-variable.color
const REGEX_VARIABLE_NAME: Lazy<Regex> = lazy_regex!(r"^\s*([A-Za-z-_]+(?:\.[A-Za-z-]+)*)\s*(?::)?");
/// Matches a variable modifier name, e.g. :my-awesome-modifier
const REGEX_VARIABLE_MODIFIER_NAME: Lazy<Regex> = lazy_regex!(r"^\s*:([A-Za-z-]+)");
/// Matches an opening brace for a modifier, e.g. (
const REGEX_VARIABLE_MODIFIER_OPEN: Lazy<Regex> = lazy_regex!(r"^\(\s*");
/// matches a comma for a modifier e.g. ,
const REGEX_VARIABLE_MODIFIER_SPLIT: Lazy<Regex> = lazy_regex!(r"^\s*,\s*");
/// Matches a closing brace for a modifier, e.g. )
const REGEX_VARIABLE_MODIFIER_CLOSE: Lazy<Regex> = lazy_regex!(r"^\s*\)");


/// Stores a token and where it's from
#[derive(Debug)]
pub struct Token {
    pub range: Range<usize>,
    pub token: TokenType
}

/// Stores a type of token, either a variable, literal or keyword
#[derive(Debug)]
pub enum TokenType {
    // pusta.hostname, theme.background:color-format( variable another.variable )
    Variable {
        name: String,
        name_range: Range<usize>,
        modifiers: Vec<(String, Vec<Token>, Range<usize>)>
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

/// Stores a literal, either a number, string or boolean
#[derive(Debug, PartialEq)]
pub enum TokenLiteral {
    String(String),
    Number(f64),
    Boolean(bool)
}


/// Reads a keyword at the given position inside the input string
pub fn read_token_at(input: &str, position: usize) -> Result<Token, VariableError> {
    // Keyword
    if let Some(keyword) = REGEX_KEYWORD.captures(&input[position..]) {
        return Ok(Token {
            range: shift_range(keyword.get(0).expect("regex should have group").range(), position),
            token: TokenType::Keyword { word: keyword.get(1).expect("regex should have group").as_str().to_owned() }
        })
    }

    // Literals
    if let Some(boolean) = REGEX_LITERAL_BOOLEAN.captures(&input[position..]) {
        let text = boolean.get(1).expect("regex should have group").as_str();

        if text.eq_ignore_ascii_case("true") || text.eq_ignore_ascii_case("yes") {
            return Ok(Token {
                range: shift_range(boolean.get(0).expect("regex should have group").range(), position),
                token: TokenType::Literal { value: TokenLiteral::Boolean(true)}
            })
        }
        if text.eq_ignore_ascii_case("false") || text.eq_ignore_ascii_case("no") {
            return Ok(Token {
                range: shift_range(boolean.get(0).expect("regex should have group").range(), position),
                token: TokenType::Literal { value: TokenLiteral::Boolean(false)}
            })
        }
    }

    if let Some(number) = REGEX_LITERAL_NUMBER.captures(&input[position..]) {
        let m = number.get(1).expect("regex should have group");

        let num = f64::from_str(m.as_str()).map_err(|e| {
            VariableError {
                title: "invalid number literal".into(),
                primary: (shift_range(m.range(), position), "is not a valid number".into()),
                secondary: vec![],
                summary: format!("´{}´ is not a valid number:\n  {e}", m.as_str())
            }
        })?;

        return Ok(Token {
            range: shift_range(number.get(0).expect("regex should have group").range(), position),
            token: TokenType::Literal { value: TokenLiteral::Number(num) }
        })
    }

    if let Some(string) = REGEX_LITERAL_STRING.captures(&input[position..]) {
        let s = string.get(1).expect("regex should have group").as_str();

        return Ok(Token {
            range: shift_range(string.get(0).expect("regex should have group").range(), position),
            token: TokenType::Literal { value: TokenLiteral::String(
                s.replace("\\n", "\n").replace("\\\"", "\"")
            )}
        })
    }

    // Variables
    if let Some(m) = REGEX_VARIABLE_NAME.captures(&input[position..]) {
        let name = m.get(1).expect("regex should have group");

        // Read modifiers
        let mut modifiers = vec![];
        let mut pos = shift_range(name.range(), position).end;
        while let Some(modifier_capture) = REGEX_VARIABLE_MODIFIER_NAME.captures(&input[pos..]) {
            let modifier_name = modifier_capture.get(1).expect("regex should have group");
            let modifier_name_range = shift_range(modifier_name.range(), pos);

            pos = shift_range(modifier_name.range(), pos).end; // Preemptively set position of next token

            // Read arguments for modifier when a brace opens
            let mut arguments = vec![];
            if let Some(modifier_open_cap) = REGEX_VARIABLE_MODIFIER_OPEN.captures(&input[pos..]) {

                let mut token_pos = shift_range(modifier_open_cap.get(0).expect("regex should have group").range(),pos).end;
                let mut first = true;
                loop {
                    // either read close or comma
                    if let Some(close_cap) = REGEX_VARIABLE_MODIFIER_CLOSE.captures(&input[token_pos..]) {
                        pos = shift_range(close_cap.get(0).expect("regex should have group").range(), token_pos).end; // Position of next token is now after the brace
                        break;

                    } else if first {
                        // can match token without comma, since it may be the first
                        first = false;

                        let token = read_token_at(input, token_pos)?;
                        token_pos = token.range.end;
                        arguments.push(token);

                    } else if let Some(comma_cap) = REGEX_VARIABLE_MODIFIER_SPLIT.captures(&input[token_pos..]) {
                        // next token comes after this
                        token_pos = shift_range(comma_cap.get(0).expect("regex should have group").range(),token_pos).end;

                        let token = read_token_at(input, token_pos)?;
                        token_pos = token.range.end;
                        arguments.push(token);

                    } else {
                        return Err(VariableError {
                            title: "expected parameter or nothing".to_string(),
                            primary: (token_pos..token_pos, "expected another parameter separated by comma or ending brace of modifier".to_string()),
                            secondary: vec![],
                            summary: "did not encounter ',', or ')', to end or continue parameter list".to_string(),
                        })
                    }
                }
            }

            modifiers.push((modifier_name.as_str().to_owned(), arguments, modifier_name_range));
        }

        return Ok(Token {
            range: (shift_range(m.get(0).expect("regex should have group").range(), position).start)..(pos),
            token: TokenType::Variable { name: name.as_str().to_owned(), modifiers, name_range: shift_range(name.range(), position)}
        })
    }

    Err(VariableError {
        title: "expected valid token".into(),
        primary: (position..position, "no valid token found here".into()),
        secondary: vec![],
        summary: "did not encounter a valid token, expected either a literal, keyword or variable".into()
    })
}

/// Shifts a range forward by a given position.
/// This has to be used to shift ranges returned from capture groups of regexes, so they are valid in the whole string and not only the substring provided to the method.
/// And yes, the captures_at method does not do the trick since anchors like ^ do not work there.
pub fn shift_range(mut range: Range<usize>, position: usize) -> Range<usize> {
    range.start += position;
    range.end += position;

    range
}

#[cfg(test)]
mod test {
    use crate::variables::token::{read_token_at, TokenLiteral, TokenType};

    #[test]
    fn variable_simple() {
        let input = "_.first.second.third:mod-one:mod-two:mod-three";
        let token = read_token_at(input, 0).unwrap();

        match token.token {
            TokenType::Variable { name, modifiers, .. } => {
                assert_eq!(name, "_.first.second.third");

                assert_eq!(modifiers[0].0, "mod-one");
                assert_eq!(modifiers[1].0, "mod-two");
                assert_eq!(modifiers[2].0, "mod-three");

                assert!(modifiers[0].1.is_empty());
                assert!(modifiers[1].1.is_empty());
                assert!(modifiers[2].1.is_empty());
            }
            _ => { unreachable!("it must be a variable") }
        }
    }

    #[test]
    fn variable_nested() {

        // with and without spaces
        let inputs = vec![
            "_.first.second.third:mod-one( _.fourth:mod-other, _.fourth-two ):mod-two( _.fifth:mod-another:mod-yet-another( pusta.sixth ) )", // this was the only possibility in v0.3.0
            "_.first.second.third:mod-one(_.fourth:mod-other,_.fourth-two):mod-two(_.fifth:mod-another:mod-yet-another(pusta.sixth))",
            "_.first.second.third:mod-one(\n            _.fourth:mod-other  ,\n      _.fourth-two \n):mod-two(    _.fifth:mod-another:mod-yet-another(   pusta.sixth))",
            "_.first.second.third\n    :mod-one(_.fourth:mod-other,_.fourth-two)\n    :mod-two(_.fifth:mod-another:mod-yet-another(pusta.sixth))",
        ];

        for i in inputs {
            let token = read_token_at(i, 0).map_err(|e| {
                e.print("test", i);
                panic!("parsing failed");
            }).unwrap();

            match token.token {
                TokenType::Variable { name, modifiers, .. } => {
                    assert_eq!(name, "_.first.second.third");

                    assert_eq!(modifiers[0].0, "mod-one");
                    assert_eq!(modifiers[1].0, "mod-two");

                    // first modifier
                    let params_mod_one = &modifiers[0].1;
                    match &params_mod_one[0].token {
                        TokenType::Variable { name, modifiers, .. } => {
                            assert_eq!(name, "_.fourth");
                            assert_eq!(modifiers[0].0, "mod-other");
                        }
                        _ => { unreachable!("it must be a variable") }
                    }
                    match &params_mod_one[1].token {
                        TokenType::Variable { name, .. } => { assert_eq!(name, "_.fourth-two") }
                        _ => { unreachable!("it must be a variable") }
                    }

                    // second modifier
                    let params_mod_two = &modifiers[1].1;
                    match &params_mod_two[0].token {
                        TokenType::Variable { name, modifiers, .. } => {
                            assert_eq!(name, "_.fifth");
                            assert_eq!(modifiers[0].0, "mod-another");
                            assert_eq!(modifiers[1].0, "mod-yet-another");

                            match &modifiers[1].1[0].token {
                                TokenType::Variable { name, .. } => { assert_eq!(name, "pusta.sixth") }
                                _ => unreachable!("it must be a variable")
                            }
                        }
                        _ => { unreachable!("it must be a variable") }
                    }
                }
                _ => { unreachable!("it must be a variable") }
            }
        }
    }

    #[test]
    fn variable_literals() {
        let input = "var:mod(-12,\"a  \\\"string\\\" with , inside    \"\n, yEs,nO, \"true\",-123.5423       )";
        let outputs = vec![
            TokenLiteral::Number(-12f64),
            TokenLiteral::String("a  \"string\" with , inside    ".to_string()),
            TokenLiteral::Boolean(true),
            TokenLiteral::Boolean(false),
            TokenLiteral::String("true".to_string()),
            TokenLiteral::Number(-123.5423)
        ];

        let token = read_token_at(input, 0).unwrap();

        match token.token {
            TokenType::Variable { name, modifiers, .. } => {
                assert_eq!(name, "var");
                assert_eq!(modifiers[0].0, "mod");

                for (token, literal) in modifiers[0].1.iter().zip(outputs.iter()) {
                    match &token.token {
                        TokenType::Literal { value } => { assert_eq!(literal, value) }
                        _ => { unreachable!("must be literal") }
                    }
                }
            }
            _ => { unreachable!("must be variable") }
        }
    }

    #[test]
    fn literal_string() {
        let input = r#"" this is a \"test\" string literal, \n this is a new line ""#;
        let token = read_token_at(input, 0).unwrap();

        match token.token {
            TokenType::Literal { value } => {
                match value {
                    TokenLiteral::String(s) => { assert_eq!(s, " this is a \"test\" string literal, \n this is a new line ") }
                    _ => { unreachable!("must be string")}
                }
            }
            _ => { unreachable!("must be literal") }
        }
    }

    #[test]
    fn literal_number() {
        let input = vec!["-1", "0", "9", "99.9009", "0000.99", "-9.13"];
        let output = vec![-1f64, 0f64, 9f64, 99.9009, 0.99, -9.13];

        for (input, output) in input.iter().zip(output.iter()) {
            let token = read_token_at(input, 0).unwrap();

            match token.token {
                TokenType::Literal { value } => {
                    match value {
                        TokenLiteral::Number(n) => { assert_eq!(n, *output) }
                        _ => { unreachable!("must be number")}
                    }
                }
                _ => { unreachable!("must be literal") }
            }
        }
    }

    #[test]
    fn literal_boolean() {

        // check normal values
        let input = vec!["true", "yEs", "tRuE", "false", "FAlse", "NO"];
        let output = vec![true, true, true, false, false, false];

        for (input, output) in input.iter().zip(output.iter()) {
            let token = read_token_at(input, 0).unwrap();

            match token.token {
                TokenType::Literal { value } => {
                    match value {
                        TokenLiteral::Boolean(n) => { assert_eq!(n, *output) }
                        _ => { unreachable!("must be boolean")}
                    }
                }
                _ => { unreachable!("must be literal") }
            }
        }

        // check that sub variables are allowed
        let input = vec!["pusta.true", "my-var.yes", "active.no"];

        for input in input {
            let token = read_token_at(input, 0).unwrap();

            match token.token {
                TokenType::Variable {name, ..} => { assert_eq!(input, name) }
                _ => { unreachable!("must be literal") }
            }
        }
    }

    #[test]
    fn keyword() {
        let input = vec!["!if", "!else", "!end", "!list", "!amogus"];
        let output = vec!["if", "else", "end", "list", "amogus"];

        for (input, output) in input.iter().zip(output.iter()) {
            let token = read_token_at(input, 0).unwrap();

            match token.token {
                TokenType::Keyword { word } => { assert_eq!(word, *output) }
                _ => { unreachable!("must be keyword") }
            }
        }
    }

}