use std::ops::Range;
use std::str::FromStr;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use lazy_regex::{Lazy, lazy_regex, regex};
use regex::{Regex, Replacer};

/// Intermediate type for an error occurring when resolving variables.
/// This type is intended to be converted to a codespan-reporting error.
#[derive(Debug)]
pub struct VariableError {
    title: String,
    primary: (Range<usize>, String),
    secondary: Vec<(Range<usize>, String)>,
    summary: String
}

impl VariableError {

    /// Prints the error to stdout using codespan-reporting.
    pub fn print(&self, filename: &str, content: &str) {

        let mut files = SimpleFiles::new();
        let input = files.add(filename, content);

        let mut labels = vec![ Label::primary(input, self.primary.0.clone()).with_message(&self.primary.1)];
        labels.append(&mut self.secondary.iter().map(|(range, s)| Label::secondary(input, range.clone()).with_message(s)).collect());

        let diagnostic = Diagnostic::error()
            .with_message(&self.title)
            .with_labels(labels)
            .with_notes(vec![ self.summary.clone() ]);

        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = term::Config::default();

        term::emit(&mut writer.lock(), &config, &files, &diagnostic).unwrap(); // TODO: Handle this gracefully
    }
}

/// Matches a keyword, e.g. !list
static REGEX_KEYWORD: Lazy<Regex> = lazy_regex!(r"^\s*!([A-Za-z]+)\s+");

/// Matches a boolean literal, e.g. true
const REGEX_LITERAL_BOOLEAN: Lazy<Regex> = lazy_regex!(r"^\s*([A-Za-z]+)\s+");
/// Matches a number literal, e.g. -1345.2768
const REGEX_LITERAL_NUMBER: Lazy<Regex> = lazy_regex!(r"^\s*(-?[0-9]+(?:\.[0-9]+)?)\s+");
/// Matches a string literal, e.g. "a string literal with inside \"quotes\" and newlines \n"
const REGEX_LITERAL_STRING: Lazy<Regex> = lazy_regex!(r#"^\s*"(.*?[^\\])"\s+"#);

/// Matches a variable name, e.g. theme.my-fancy-variable.color
const REGEX_VARIABLE_NAME: Lazy<Regex> = lazy_regex!(r"^\s*([A-Za-z-]+(?:\.[A-Za-z-]+)*)(?:(?:\s+)|(?::))");
/// Matches a variable modifier name, e.g. :my-awesome-modifier
const REGEX_VARIABLE_MODIFIER_NAME: Lazy<Regex> = lazy_regex!(r"^:([A-Za-z-]+)");
/// Matches an opening brace for a modifier, e.g. (
const REGEX_VARIABLE_MODIFIER_OPEN: Lazy<Regex> = lazy_regex!(r"^\(\s*");
/// Matches a closing brace for a modifier, e.g. )
const REGEX_VARIABLE_MODIFIER_CLOSE: Lazy<Regex> = lazy_regex!(r"^\s*\)");


/// Stores a token and where it's from
#[derive(Debug)]
pub struct Token {
    range: Range<usize>,
    token: TokenType
}

/// Stores a type of token, either a variable, literal or keyword
#[derive(Debug)]
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

/// Stores a literal, either a number, string or boolean
#[derive(Debug)]
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
            pos = shift_range(modifier_name.range(), pos).end; // Preemptively set position of next token

            // Read arguments for modifier when a brace opens
            let mut arguments = vec![];
            if let Some(modifier_open_cap) = REGEX_VARIABLE_MODIFIER_OPEN.captures(&input[pos..]) {

                let mut token_pos = shift_range(modifier_open_cap.get(0).expect("regex should have group").range(),pos).end;
                loop {
                    // Break when having read closing brace
                    if let Some(close_cap) = REGEX_VARIABLE_MODIFIER_CLOSE.captures(&input[token_pos..]) {
                        pos = shift_range(close_cap.get(0).expect("regex should have group").range(), token_pos).end; // Position of next token is now after the brace
                        break;
                    }

                    // Read token which is argument
                    let mut token = read_token_at(input, token_pos)?;
                    token_pos = token.range.end;
                    arguments.push(token);
                }
            }

            modifiers.push((modifier_name.as_str().to_owned(), arguments));
        }

        return Ok(Token {
            range: (shift_range(m.get(0).expect("regex should have group").range(), position).start)..(pos),
            token: TokenType::Variable { name: name.as_str().to_owned(), modifiers }
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
fn shift_range(mut range: Range<usize>, position: usize) -> Range<usize> {
    range.start += position;
    range.end += position;

    range
}