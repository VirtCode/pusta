use serde::{Deserialize, Serialize};
use crate::module::transaction::change::{AtomicChange, ChangeError, ChangeResult, ChangeRuntime};

mod change;
mod shell;
mod worker;

/// Represents a single transaction on the system
struct TransactionData {
    /// In the prepared order in which execution can succeed
    /// Depends and because variables will be checked anyway.
    modules: Vec<ModuleTransaction>,
}

/// Represents the transaction of a change to a module
struct ModuleTransaction {
    /// Temporary id of the transaction.
    id: u64,

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

    /// Has to be run as root
    root: bool,

    /// Changes
    changes: Vec<ChangeTransaction>
}

#[derive(Serialize, Deserialize)]
struct ChangeTransaction {
    id: u32,
    change: Box<dyn AtomicChange>
}

