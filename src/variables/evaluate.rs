use std::collections::HashMap;
use std::ops::Range;
use crate::variables::context::{Context, Expression, ExpressionContent, Statement, StatementType};
use crate::variables::{Value, Variable, VariableError};
use crate::variables::modifier::evaluate_modifier;

#[derive(Default)]
pub struct VariableEvalCounter {
    variables: Vec<String>
}

impl VariableEvalCounter {
    /// Marks a variable as used
    fn used(&mut self, used: &str) {
        self.variables.push(used.into());
    }

    /// Returns all deduplicated variable usages
    pub fn usages(mut self: Self) -> Vec<String> {

        // remove variables which are local lists
        self.variables.retain(|e| !e.starts_with('_'));

        self.variables.dedup();
        self.variables
    }
}

/// Evaluates a context consisting of many statements
pub fn evaluate(input: &str, context: &Context, variables: &Variable, counter: &mut VariableEvalCounter) -> Result<String, VariableError> {

    let mut evals = context.statements.iter().map(|s| {
        Ok((&s.location,
            evaluate_statement(input, s, variables, counter)?))
    }).collect::<Result<Vec<(&Range<usize>, String)>, VariableError>>()?;

    evals.sort_by(|(l1, _), (l2, _)| l1.start.cmp(&l2.start));

    let mut result = String::new();
    let mut index = context.source.start;

    for (range, string) in evals {
        result += &input[index..range.start];
        index = range.end;

        result += &string;
    }

    result += &input[index..context.source.end];

    Ok(result)
}

/// Evaluates a single statement
fn evaluate_statement(input: &str, statement: &Statement, variables: &Variable, counter: &mut VariableEvalCounter) -> Result<String, VariableError> {
    Ok(match &statement.content {
        StatementType::Expression { content } => {
            evaluate_expression(content, variables, counter)?.to_string()
        }
        StatementType::Conditional { condition, context_true, context_false } => {
            let condition = match evaluate_expression(condition, variables, counter)? {
                Value::Boolean(b) => {b}
                v => {
                    return Err(VariableError {
                        title: "expected boolean condition".to_string(),
                        primary: (condition.range.clone(), format!("expected condition of type boolean, found `{}`", v.type_name())),
                        secondary: vec![],
                        summary: "the condition of an if statement has to be of type boolean".to_string(),
                    })
                }
            };

            match (condition, context_false) {
                (true, _) => { evaluate(input, context_true, variables, counter)? }
                (false, Some(context_false)) => { evaluate(input, context_false, variables, counter)? }
                (false, None) => { "".to_string() }
            }
        }
        StatementType::List { expression, context } => {
            let items = evaluate_expression_for_list(expression, variables, counter)?;

            let mut variables_custom = variables.clone(); // TODO: Implement this in a better way

            let mut string = String::new();
            for v in items {

                // overwrite _ entry
                match &mut variables_custom {
                    Variable::Group(g) => { g.insert("_".to_string(), v); }
                    _ => {}
                }

                string += &evaluate(input, context, &variables_custom, counter)?;
            }

            string
        }
    })
}

/// Evaluates the expression for a list. This should only be temporary, because modifiers should support lists and objects in the future too.
fn evaluate_expression_for_list(expr: &Expression, variables: &Variable, counter: &mut VariableEvalCounter) -> Result<Vec<Variable>, VariableError> {
    match (&expr.content, expr.modifiers.is_empty()) {
        (ExpressionContent::Variable(name), true) => {
            counter.used(&name);
            match variables.find(&name) {
                Some(Variable::List(list)) => { Ok(list.clone()) },
                Some(Variable::Group(group)) => {
                    let list = group.iter().map(|(key, value)| {
                        let mut group = HashMap::new();
                        group.insert(String::from("key"), Variable::Value(Value::String(key.clone())));
                        group.insert(String::from("value"), value.clone());
                        Variable::Group(group)
                    }).collect::<Vec<_>>();
                    Ok(list)
                }
                _ => {
                   return Err(VariableError {
                       title: "unexpected variable type for list".to_string(),
                       primary: (expr.range.clone(), "lists only accept the list variable type".to_string()),
                       secondary: vec![],
                       summary: "variables used in list statements must also be of type list".to_string(),
                   })
                }
            }
        }
        _ => {
            return Err(VariableError {
                title: "invalid type for list".to_string(),
                primary: (expr.range.clone(), "lists currently do only support variables of type lists with no modifiers".to_string()),
                secondary: vec![],
                summary: "remove any modifiers and make sure the expression base is a variable".to_string(),
            })
        }
    }
}

/// Evaluates an expression to a value
fn evaluate_expression(expr: &Expression, variables: &Variable, counter: &mut VariableEvalCounter) -> Result<Value, VariableError> {

    // evaluate content
    let mut state = match &expr.content {
        ExpressionContent::LiteralString(s) => { Value::String(s.clone()) }
        ExpressionContent::LiteralNumber(n) => { Value::Number(*n) }
        ExpressionContent::LiteralBool(b) => { Value::Boolean(*b) }
        ExpressionContent::Variable(name) => {
            counter.used(name);
            let var = variables.find(name);

            match var {
                Some(Variable::Value(v)) => { v.clone() },
                Some(_) => {
                    return Err(VariableError {
                        title: "unexpected variable type".to_string(),
                        primary: (expr.content_range.clone(), format!("expected `{name}` to be of type value, but found an object or list")),
                        secondary: vec![],
                        summary: "expressions expect value variables, lists can only be processed by list statements".to_string(),
                    })
                },
                None => {
                    return Err(VariableError {
                        title: "variable not found".to_string(),
                        primary: (expr.content_range.clone(), format!("variable `{name}` not found" )),
                        secondary: vec![],
                        summary: "could not find the referenced variable, is it defined in the right context?".to_string(),
                    })
                }
            }
        }
    };

    // evaluate modifiers
    for x in &expr.modifiers {
        let parameters = x.parameters.iter().map(|e| evaluate_expression(e, variables, counter)).collect::<Result<Vec<_>, VariableError>>()?;

        match evaluate_modifier(&x.name, state, parameters) {
            Some(result) => {
                state = result.map_err(|e| e.to_general(&expr, x))?;
            }
            None => {
                return Err(VariableError {
                    title: "no such modifier".to_string(),
                    primary: (x.name_range.clone(), format!("there exists no modifier under the name `{}`", &x.name)),
                    secondary: vec![],
                    summary: "there does not exist a modifier with the given name, it it spelled correctly?".to_string(),
                })
            }
        }
    }

    // all done
    Ok(state)
}