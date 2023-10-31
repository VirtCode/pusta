use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use colored::Colorize;
use log::{error, info};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::module::qualifier::ModuleQualifier;
use crate::module::transaction::change::{AtomicChange, ChangeError, ChangeResult, ChangeRuntime};
use crate::output::{prompt, prompt_yn};

pub mod change;
mod shell;
pub mod worker;

/// Represents a single transaction on the system
struct TransactionData {
    /// In the prepared order in which execution can succeed
    /// Depends and because variables will be checked anyway.
    modules: Vec<ModuleTransaction>,
}

impl TransactionData {
    pub fn preview(&self, file_previewer: String) -> bool{
        info!("{}", "These specific changes are going to be applied:".bold());

        let mut file_map = HashMap::new();

        // Retrieve qualifiers for module ids
        let mut transaction_qualifier = HashMap::new();
        for t in &self.modules { transaction_qualifier.insert(t.id, t.qualifier.unique()); }

        for transaction in &self.modules {
            info!("Module: {}", transaction.qualifier.unique().bold());
            info!("        id: {}{}{}", transaction.id, {
                if !transaction.depends.is_empty() {
                    let string = transaction.depends.iter().filter_map(|id| transaction_qualifier.get(id).cloned()).collect::<Vec<String>>().join(", ");
                    format!(", depends on: {}", string.italic())
                } else { String::new() }
            }, {
                if !transaction.because.is_empty() {
                    let string = transaction.because.iter().filter_map(|id| transaction_qualifier.get(id).cloned()).collect::<Vec<String>>().join(", ");
                    format!(", because of: {}", string.italic())
                } else { String::new() }
            });

            for job in &transaction.jobs {
                for change in &job.changes {

                    let id = &change.id.to_string()[26..];

                    let mut flags = vec![];
                    if change.root { flags.push("root".bright_red().bold().to_string()) }
                    if job.optional { flags.push("optional".bright_blue().to_string()) }
                    if !job.revertible { flags.push("irreversible".dimmed().to_string()) }
                    let flags = if flags.is_empty() { "".to_string() } else { format!(", flags: {}", flags.join(" ")) };

                    let files = change.change.files().into_iter().map(|(name, data)| {
                        let id = file_map.len() + 1;
                        let entry = format!("{} ({})", id, &name);
                        file_map.insert(id, data);
                        entry
                    }).collect::<Vec<String>>();
                    let files = if files.is_empty() { "".to_string() } else { format!(", content: {}", files.join(", ")) };

                    info!("    {}", change.change.describe());
                    info!("        id: {}{}{}", id, files, flags);
                }
            }

            info!("");
        }

        info!("Confirm these changes with yes or no, use a content number to preview that content.");
        loop {
            let response = prompt("Apply changes or preview file? [Y/n/content] ").to_lowercase();
            if response.starts_with("y") { break true; }
            if response.starts_with("n") { break false; }

            if let Some(data) = usize::from_str(&response).ok().and_then(|id| file_map.get(&id)) {
                if let Err(e) = preview_file(&file_previewer, data) { error!("failed to open preview: {}", e); }
            }
        }
    }
}

const TMP_PREVIEW_PATH: &str = "/tmp/pusta/preview";

/// Previews data in a file previewer, by first saving it to disk
fn preview_file(previewer: &str, data: &str) -> anyhow::Result<()>{
    fs::write(Path::new(TMP_PREVIEW_PATH), data)?;

    let mut command = Command::new(previewer);
    command.arg(TMP_PREVIEW_PATH);

    command.spawn()?.wait()?;

    Ok(())
}

/// Represents the transaction of a change to a module
struct ModuleTransaction {
    /// Temporary id of the transaction.
    id: u64,

    /// Unique qualifier of the module that is changed
    qualifier: ModuleQualifier,

    /// This transaction depends on the success of these transactions.
    /// Can only be installed if those transactions succeeded.
    depends: Vec<u64>,

    /// This transaction is only run for these transactions.
    /// If all those fail, there is no purpose anymore for this module.
    because: Vec<u64>,

    /// Jobs to be executed, in the order as this array is
    jobs: Vec<JobTransaction>
}

/// Represents a transaction that a job does
struct JobTransaction {
    /// Continue module transaction when job failed.
    optional: bool,

    /// Should the changes be reverted on failure. This would be false on an uninstall action.
    revertible: bool,

    /// Changes
    changes: Vec<ChangeTransaction>
}

/// Represents a single change
struct ChangeTransaction {
    /// Id of the change used when communicating with workers
    id: Uuid,
    /// Change should be run as root
    root: bool,
    /// Change to perform
    change: Box<dyn AtomicChange>
}

impl ChangeTransaction {
    pub fn new(change: Box<dyn AtomicChange>, root: bool) -> Self {
        Self {
            id: Uuid::new_v4(), change, root
        }
    }
}

