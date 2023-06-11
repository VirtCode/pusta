use colored::Colorize;
use log::{debug, error, info};
use crate::module::install::{InstalledModule, Installer, InstallReason};
use crate::module::Module;
use crate::output;
use crate::registry::cache::Cache;
use crate::registry::index::Indexable;

/// This enum represents a single action that is done to the current modules
pub enum ModuleTransaction {
    Install(Module, InstallReason),
    Update(InstalledModule, Module),
    Reinstall(InstalledModule, Module),
    Remove(InstalledModule)
}

impl ModuleTransaction {

    /// Performs the given transactions
    fn perform(self, installer: &Installer, cache: &mut Cache) -> bool {

        match self {
            ModuleTransaction::Install(module, reason) => {
                output::start_section(&format!("Installing {}-{}", module.qualifier.unique(), module.version.dimmed()));

                if let Some(m) = installer.install(module, cache, reason) {
                    cache.install_module(m).unwrap_or_else(|e| error!("Failed to persist install: {e}"));

                    output::end_section(true, "Module installed successfully");
                    true
                } else {
                    output::end_section(false, "Module install failed");
                    false
                }
            }
            ModuleTransaction::Update(installed, module) => {
                output::start_section(&format!("Updating {} ({} -> {})", module.qualifier.unique(), module.version.dimmed(), installed.module.version.dimmed()));

                if let Some(module) = installer.update(&installed, module, cache) {
                    cache.install_module(module).unwrap_or_else(|e| error!("Failed to persist update: {e}"));

                    output::end_section(true, "Module updated successfully");
                    true
                } else {
                    cache.remove_module(&installed.qualifier()).unwrap_or_else(|e| error!("Failed to persist removal: {e}"));

                    output::end_section(false, "Module update failed, it is no longer installed");
                    false
                }
            }
            ModuleTransaction::Reinstall(installed, module) => {
                output::start_section(&format!("Reinstalling {}", module.qualifier.unique()));

                installer.uninstall(&installed, cache);

                if let Some(m) = installer.install(module, cache, installed.reason.clone()) {
                    cache.install_module(m).unwrap_or_else(|e| error!("Failed to persist reinstall: {e}"));

                    output::end_section(true, "Module reinstalled successfully");
                    true
                } else {
                    cache.remove_module(&installed.qualifier()).unwrap_or_else(|e| error!("Failed to persist failed reinstall: {e}"));

                    output::end_section(false, "Module reinstall failed, it is no longer installed");
                    false
                }
            }
            ModuleTransaction::Remove(installed) => {
                output::start_section(&format!("Removing {}", installed.module.qualifier.unique()));

                installer.uninstall(&installed, cache);

                cache.delete_module_cache(&installed.module).unwrap_or_else(|e| {
                    debug!("Failed to delete module cache ({}), filesystem may stay polluted", e.to_string());
                });
                cache.remove_module(&installed.qualifier()).unwrap_or_else(|e| error!("Failed to persist module removal: {e}"));

                output::end_section(true, "Successfully removed module");
                true
            }
        }
    }

    /// Produces a string that is shown to the user to indicate the current transaction.
    /// Example message for installation: Pusta (pusta-0.2)
    fn message(&self) -> String {
        match self {
            ModuleTransaction::Install(m, _) => { format!("{} ({}-{})", m.name.bold(), m.qualifier.unique(), m.version.dimmed()) }
            ModuleTransaction::Update(i, m) => { format!("{} ({}-{} -> {}-{})", m.name, i.module.qualifier.unique(), i.module.version.dimmed(), m.qualifier.unique(), m.version.dimmed()) }
            ModuleTransaction::Reinstall(i, m) => { format!("{} ({}-{} ~> {}-{})", m.name, i.module.qualifier.unique(), i.module.version.dimmed(), m.qualifier.unique(), m.version.dimmed()) }
            ModuleTransaction::Remove(m) => { format!("{} (remove)", m.module.name.strikethrough()) }
        }
    }
}

/// This function executes a number of transactions in a row and also prompts the user in the process.
pub fn transact(transactions: Vec<ModuleTransaction>, cache: &mut Cache, installer: &Installer) {
    info!("Scheduled module changes:");
    for t in &transactions {
        println!("   {}", t.message());
    }

    if !output::prompt_yn("Do you want to make these changes now?", true) {
        error!("Changes cancelled by user");
        return;
    }

    println!();

    let len = transactions.len();
    for (i, t) in transactions.into_iter().enumerate() {
        let result = t.perform(installer, cache);

        if !result {
            println!();
            if i < len - 1 && !output::prompt_yn("Previous change failed, continue with the other changes?", true) {
                error!("Changes interrupted by the user");
                return;
            }
            println!();
        }
    }

    println!();
    info!("Made changes successfully");
}