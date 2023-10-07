use std::ops::Range;
use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;
use crate::variables::token::{read_token_at, shift_range, Token, TokenLiteral, TokenType};
use crate::variables::{VariableError};

/// Matches the start of a statement
static REGEX_START: Lazy<Regex> = lazy_regex!(r"%%\s*");
/// Matches the end of a statement
static REGEX_END: Lazy<Regex> = lazy_regex!(r"^\s*%%");

const KEYWORD_CONDITIONAL_IF: &str = "if";
const KEYWORD_CONDITIONAL_ELSE: &str = "else";
const KEYWORD_LIST: &str = "list";
const KEYWORD_END: &str = "end";


/// Represents a file or part of a file with dynamic content
#[derive(Debug)]
pub struct Context {
    source: Range<usize>,
    statements: Vec<Statement>
}

/// Represents a single instance of dynamic content
#[derive(Debug)]
pub struct Statement {
    location: Range<usize>,
    content: StatementType
}

/// Represents all different types dynamic content can be
#[derive(Debug)]
pub enum StatementType {
    /// Expression like a variable reference
    Expression {
        content: Expression
    },
    /// Conditional, an if-else statement which is evaluated based on an expression
    Conditional {
        condition: Expression,
        context_true: Context,
        context_false: Option<Context>
    },
    /// List, which loops through an expression and adds context for each child
    List {
        expression: Expression,
        context: Context
    },
}

/// Represents an expression which is evaluated to a value
#[derive(Debug)]
pub enum Expression {
    Literal {
        content: TokenLiteral
    },
    Variable {
        name: String,
        modifiers: Vec<(String, Vec<Expression>)>
    }
}

enum NextStatement {
    Full(Statement),
    ControlFlow(String)
}

pub fn read_context(input: &str) -> Result<Context, VariableError> {
    let (context, control) = read_next_context(input, 0)?;

    if let Some((keyword, range)) = control {
        return Err(VariableError {
            title: "unexpected control flow keyword".into(),
            primary: (range, format!("did not expect keyword `!{keyword}` here")),
            secondary: vec![],
            summary: "this keyword is out of place, there are no open contexts".into()
        })
    }

    Ok(context)
}

/// Reads a context from the specified starting position
/// Exits early if it encounters a rouge control flow keyword
pub fn read_next_context(input: &str, start: usize) -> Result<(Context, Option<(String, Range<usize>)>), VariableError> {

    let mut read_start = start;

    let mut statements = vec![];
    let mut control_flow = None;
    let mut end = input.len() - 1;

    /// Match a new expression
    while let Some(start) = read_statement_start(input, read_start) {

        let (next_end, next) = read_next_statement(input, start.clone())?;
        match next {
            NextStatement::Full(s) => {
                statements.push(s);
                read_start = next_end;
            }
            NextStatement::ControlFlow(keyword) => {
                end = next_end;
                control_flow = Some((keyword, start.start..next_end));
                break;
            }
        }
    }

    Ok((
        Context {
            source: start..end,
            statements
        },
        control_flow
    ))
}

/// Reads the start of a statement, if present
fn read_statement_start(input: &str, start: usize) -> Option<Range<usize>> {
    REGEX_START.captures(&input[start..]).map(|cap| {
        shift_range(cap.get(0).expect("0th capture always exists").range(), start)
    })

}

/// Reads the end of a statement
/// If no statement end is present, an error will be thrown with the summary provided
fn read_statement_end(input: &str, start: usize, open_range: &Range<usize>, summary: &str) -> Result<Range<usize>, VariableError>{
    if let Some(end) = REGEX_END.captures(&input[start..]) {
        Ok(shift_range(end.get(0).expect("0th capture always exists").range(), start))
    } else {
        Err(VariableError {
            title: "expected end of statement".into(),
            primary: (start..start, "expected end of statement, found other unknown token instead".into()),
            secondary: vec![(open_range.clone(), "statement started here".into())],
            summary: summary.into()
        })
    }
}

/// Reads a statement at a given position, with the start already read.
/// Returns the end position as well as the statement type
fn read_next_statement(input: &str, open_range: Range<usize>) -> Result<(usize, NextStatement), VariableError> {
    let first = read_token_at(input, open_range.end)?;

    Ok(match first.token {

        TokenType::Variable { name, modifiers } => { // Expression statement

            let end = read_statement_end(input, first.range.end, &open_range, "a variable reference must only contain a single token")?;

            (end.end, NextStatement::Full(Statement {
                location: open_range.start..end.end,
                content: StatementType::Expression { content: to_variable_expression(name, modifiers)? }
            }))
        }

        TokenType::Keyword { ref word } => { // More complex statement

            match word.as_str() {
                KEYWORD_CONDITIONAL_IF => {
                    let (end, statement) = read_if_statement(input, open_range, &first)?;
                    (end, NextStatement::Full(statement))
                },

                KEYWORD_LIST => {
                    let (end, statement) = read_list_statement(input, open_range, &first)?;
                    (end, NextStatement::Full(statement))
                },

                KEYWORD_CONDITIONAL_ELSE | KEYWORD_END => {
                    let end = read_statement_end(input, first.range.end, &open_range, "control flow statements do only contain one token")?;
                    (end.end, NextStatement::ControlFlow(word.to_owned()))
                },

                _ => { return Err(VariableError {
                    title: "keyword not recognized".into(),
                    primary: (first.range, "this keyword invalid and not recognized".into()),
                    secondary: vec![],
                    summary: "valid keywords are e.g. `!if`, `!list`, etc.".into()
                }) }
            }
        }

        TokenType::Literal { .. } => { // Illegal literal statement
            return Err(VariableError {
                title: "expected keyword or variable reference".into(),
                primary: (first.range, "literal as the first token of an expression is not expected".into()),
                secondary: vec![],
                summary: "just literals are not allowed, use a variable instead".into()
            })
        }
    })
}

/// Reads an if statement with all the body etc.
fn read_if_statement(input: &str, open_range: Range<usize>, first: &Token) -> Result<(usize, Statement), VariableError> {

    let condition_token = read_token_at(input, first.range.end)?;
    let condition = match condition_token.token {
        TokenType::Variable { name, modifiers } => { to_variable_expression(name, modifiers)? }
        TokenType::Literal { value } => { to_literal_expression(value) }
        _ => {
            return Err(VariableError {
                title: "expected variable reference or literal".into(),
                primary: (condition_token.range, "expected variable or literal here".into()),
                secondary: vec![(first.range.clone(), "if statement begun here".into())],
                summary: format!("a variable or literal is required after an `!{KEYWORD_CONDITIONAL_IF}` statement")
            })
        }
    };

    // Read end of statement
    let end_index = read_statement_end(input, condition_token.range.end, &open_range, "an if statement only expects one condition token and nothing more")?;

    // Read true context
    let (context_true, next) = read_next_context(input, end_index.end)?;

    // Read false context or end
    let (context_false, end) = if let Some((keyword, end)) = next {
        match keyword.as_str() {
            KEYWORD_CONDITIONAL_ELSE => {
                let (context_false, next) = read_next_context(input, end.end)?;
                if let Some((keyword, end)) = next {
                    if keyword.as_str() == KEYWORD_END {
                        (Some(context_false), end.end)
                    } else {
                        return Err(VariableError {
                            title: "expected valid control flow statement".into(),
                            primary: (end.clone(), format!("expected `!{KEYWORD_END}` here, found other keyword instead")),
                            secondary: vec![(open_range.start..end_index.end, "if statement opened context here".into()), (end.clone(), "else statement opened context here".into())],
                            summary: format!("expected `!{KEYWORD_END}` instead of `!{keyword}` to finish else context")
                        })
                    }
                } else {
                    return Err(VariableError {
                        title: "expected control flow statement".into(),
                        primary: (context_false.source.end..context_false.source.end, format!("expected `!{KEYWORD_END}` here, found end of file")),
                        secondary: vec![(open_range.start..end_index.end, "if statement opened context here".into()), (end.clone(), "else statement opened context here".into())],
                        summary: format!("expected `!{KEYWORD_END}` to finish else context")
                    })
                }
            },
            KEYWORD_END => {
                (None, end.end)
            },

            _ => {
                return Err(VariableError {
                    title: "unexpected control flow statement".into(),
                    primary: (end, format!("expected a fitting control flow statement here")),
                    secondary: vec![(open_range.start..end_index.end, "if statement opened context here".into())],
                    summary: "expected either `!{KEYWORD_CONDITIONAL_ELSE}` or `!{KEYWORD_END}` after an if statement".into()
                })
            }
        }
    } else {
        return Err(VariableError {
            title: "expected control flow statement".into(),
            primary: (context_true.source.end..context_true.source.end, format!("expected `!{KEYWORD_CONDITIONAL_ELSE}` or `!{KEYWORD_END}` here, found end of file")),
            secondary: vec![(open_range.start..end_index.end, "if statement opened context here".into())],
            summary: format!("expected `!{KEYWORD_CONDITIONAL_ELSE}` or `!{KEYWORD_END}` to finish if context")
        })
    };

    Ok((end, Statement {
        location: open_range.start..end,
        content: StatementType::Conditional { condition, context_true, context_false }
    }))
}

/// Reads a list statement with its body
fn read_list_statement(input: &str, open_range: Range<usize>, first: &Token) -> Result<(usize, Statement), VariableError> {
    let expression_token = read_token_at(input, first.range.end)?;
    let expression = match expression_token.token {
        TokenType::Variable { name, modifiers } => { to_variable_expression(name, modifiers)? }
        _ => {
            return Err(VariableError {
                title: "expected variable reference".into(),
                primary: (expression_token.range, "expected variable reference here".into()),
                secondary: vec![(first.range.clone(), "list statement begun here".into())],
                summary: format!("a variable reference is required after a `!{KEYWORD_LIST}` statement")
            })
        }
    };

    // Read statement end
    let end = read_statement_end(input, expression_token.range.end, &open_range, "a list statement only takes one argument")?;

    // Read list context
    let (context, next) = read_next_context(input, end.end)?;

    // Make sure that the context ended successfully
    let end = if let Some((keyword, end)) = next {
        if keyword.as_str() != KEYWORD_END {
            return Err(VariableError {
                title: "expected valid control flow statement".into(),
                primary: (end.clone(), format!("expected `!{KEYWORD_END}` here, found other keyword instead")),
                secondary: vec![(open_range.start..end.end, "list statement opened context here".into())],
                summary: format!("expected `!{KEYWORD_END}` instead of `!{keyword}` to finish list context")
            })
        }

        end
    } else {
        return Err(VariableError {
            title: "expected control flow statement".into(),
            primary: (context.source.end..context.source.end, format!("expected `!{KEYWORD_END}` here, found end of file")),
            secondary: vec![(open_range.start..end.end, "list statement opened context here".into())],
            summary: format!("expected `!{KEYWORD_END}` to finish list context")
        })
    };

    Ok((end.end, Statement {
        location: open_range.start..end.end,
        content: StatementType::List { expression, context }
    }))
}

/// Converts a tokenized variable into an expression
fn to_variable_expression(name: String, modifiers: Vec<(String, Vec<Token>)>) -> Result<Expression, VariableError> {
    Ok(Expression::Variable {
        name,
        modifiers: modifiers.into_iter().map(|(name, tokens)| {
            Ok((name,
            tokens.into_iter().map(|token| {
                match token.token {
                    TokenType::Variable { name, modifiers } => { to_variable_expression(name, modifiers) }
                    TokenType::Literal { value } => { Ok(to_literal_expression(value)) }
                    TokenType::Keyword { .. } => {
                        return Err(VariableError {
                            title: "expected variable reference or literal".into(),
                            primary: (token.range, "found keyword, expected variable reference or literal".into()),
                            secondary: vec![],
                            summary: "variable modifiers cannot take keywords as parameters".into()
                        })
                    }
                }
            }).collect::<Result<Vec<Expression>, VariableError>>()?))
        }).collect::<Result<Vec<(String, Vec<Expression>)>, VariableError>>()?
    })
}

/// Converts a tokenized literal to an expression
fn to_literal_expression(value: TokenLiteral) -> Expression {
    Expression::Literal {content: value}
}