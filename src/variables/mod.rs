pub mod token;
pub mod context;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::ops::Range;
use std::path::PathBuf;
use anyhow::{anyhow, Context};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use log::debug;
use serde::{Deserialize, Serialize};
use crate::config::Config;

pub const LEVEL_SEPARATOR: char = '.';

/// Represents a variable, may either be a value, a list oder a group
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Variable {
    Group(HashMap<String, Variable>),
    List(Vec<Variable>),
    Value(String),
}

impl Variable {
    /// Merges another variable into this object
    /// Follows the following rules:
    /// - Properties which are not the same kind are overwritten
    /// - Properties which are the same kind are:
    ///     - Merged when Group or List
    ///     - Overwritten when Value
    pub fn merge(&mut self, other: &Self) {
        match (self, other) {
            (Self::Group(mine), Self::Group(not)) => {
                for (key, variable) in not {
                    match mine.get_mut(key) {
                        None => { mine.insert(key.clone(), variable.clone()); }
                        Some(current) => { current.merge(variable); }
                    }
                }
            }
            (Self::List(mine), Self::List(not)) => {
                mine.append(&mut not.clone());
            }
            (mine, other) => {
                *mine = other.clone();
            }
        }
    }

    /// Finds a variable inside this under a given name.
    pub fn find(&self, variable: &str) -> Option<&Variable> {
        match self {
            Variable::Group(map) => {
                match variable.find(LEVEL_SEPARATOR) {
                    Some(index) => {
                        map.get(&variable[..index]).and_then(|v| v.find(&variable[(index + 1)..]))
                    }
                    None => { map.get(variable) }
                }
            }
            _ => { None }
        }
    }
}

pub const DEFAULT_PARENT: &str = "~/.config";
pub const DEFAULT_SYSTEM_VARIABLES: &str = "/pusta/variables.yml";

/// Returns the default path for the system variables ($XDG_CONFIG_HOME/pusta/variables.yml)
pub fn default_system_variables() -> String {
    let parent = match env::var("XDG_CONFIG_HOME") {
        Ok(s) => { s }
        Err(_) => { DEFAULT_PARENT.to_owned() }
    };

    parent + DEFAULT_SYSTEM_VARIABLES
}

/// Loads the system variables from the filesystem
pub fn load_system(config: &Config) -> Option<Variable> {
    debug!("Reading system variables");

    let path = PathBuf::from(shellexpand::tilde(&config.system_variables).to_string());
    if !path.exists() {
        debug!("System variables are not defined, skipping");
        return None;
    }

    match File::open(&path).map_err(|e| anyhow!(e))
        .and_then(|f| serde_yaml::from_reader(f).context("Failed to deserialize config")) {

        Ok(var) => { Some(var) }
        Err(e) => {
            debug!("Failed to read system variables: {}", e.to_string());
            None
        }
    }
}

/// Generates the magic variables which are the top most level
pub fn generate_magic() -> Variable {
    Variable::Group(HashMap::from([
        ("pusta".into(), Variable::Group(HashMap::from([
            ("username".into(), Variable::Value(whoami::username())),
            ("hostname".into(), Variable::Value(whoami::hostname()))
        ])))
    ]))
}

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