use anyhow::anyhow;
use log::{error, info};
use crate::module::install::{InstalledModule, InstallReason};
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::output::{prompt_choice_module, prompt_yn};
use crate::registry::depend::ChangeType::{Real, Marker};
use crate::registry::index::{Index, Indexable};
use crate::registry::Registry;
use crate::registry::transaction::ModuleTransaction;

/// This struct can resolve and free dependencies for multiple modules and perform them on a module basis, performing rollbacks when needed.
/// Warning: understanding the logic implemented here may cause headaches
pub struct DependencyResolver<'a> {
    available: &'a Index<Module>,
    installed: &'a Index<InstalledModule>,

    add: Vec<Change<'a, Module>>,
    remove: Vec<Change<'a, InstalledModule>>,

    current_group: u32
}

impl<'a> DependencyResolver<'a> {

    /// Starts a new dependency resolver
    pub fn new(available: &'a Index<Module>, installed: &'a Index<InstalledModule>) -> Self {
        Self {
            available,
            installed,
            add: vec![],
            remove: vec![],
            current_group: 0
        }
    }

    /// Starts the resolving process of a module
    pub fn resolve(&mut self, module: &Module) -> anyhow::Result<()>{

        self.current_group += 1; // Create new change group
        self.add.push(Change::mark(self.current_group, module));

        self.resolve_module(module)
    }

    /// Resolves the actual dependencies of a module recursively
    fn resolve_module(&mut self, module: &Module) -> anyhow::Result<()>{
        for dep in &module.dependencies {

            // Check whether already satisfied
            if self.dependency_satisfied(dep) { continue; }

            // Install dependency to satisfy
            let providers = self.available.providers(&dep);
            if let Some(m) = prompt_choice_module(
                &providers,
                &format!("Multiple modules provide dependency '{dep}' for {}, choose:", module.qualifier.unique())).and_then(|i| providers.get(i).copied()) {

                self.add.push(Change::real(self.current_group, m));
                self.resolve_module(m)?;

            } else {
                error!("Failed to find module for dependency '{dep}' required by {}", module.qualifier.unique());
                if !prompt_yn("Continue without this dependency?", false) {
                    self.rollback();
                    return Err(anyhow!("Failed to resolve dependencies"));
                }
            }
        }

        Ok(())
    }

    /// Rolls the dependency changes back for the last group
    /// (and only for the last group, arbitrary group rollbacks are not permitted since later groups may depend on them)
    fn rollback(&mut self) {
        self.add.retain(|c| c.group != self.current_group);
        self.remove.retain(|c| c.group != self.current_group);
    }

    /// Starts freeing the dependencies used by the given module
    pub fn free(&mut self, module: &'a InstalledModule) {
        self.current_group += 1;
        self.remove.push(Change::mark(self.current_group, module));

        self.free_module(module)
    }

    /// Frees the dependencies used by the given module recursively
    fn free_module(&mut self, module: &'a InstalledModule) {
        // Go through every dependency and its providers that are installed as a dependency
        for provider in module.module.dependencies.iter().flat_map(|dep| self.installed.providers(dep).into_iter())
                .filter(|i| matches!(i.reason, InstallReason::Dependency)) {

            // Continue if already removed
            if self.remove.iter().any(|f| f.qualifier() == provider.qualifier()) {
                continue;
            }

            // Are they being depended upon and should remove
            if !self.depended_upon(provider.qualifier()) &&
                    prompt_yn(&format!("The module {} is no longer being depended upon, remove?", provider.qualifier().unique()), true){

                self.remove.push(Change::real(self.current_group, provider));
                self.free_module(provider);
            }
        }
    }

    /// Checks whether a dependency is satisfied based on the current state
    fn dependency_satisfied(&self, dep: &str) -> bool {
        // is installed and not being uninstalled
        self.installed.providers(dep).iter().any(|m| !self.remove.iter().any(|f| f.qualifier() == m.qualifier()))||
        // is being installed
        self.add.iter().any(|q| q.qualifier().does_provide(dep))
    }

    /// Checks whether a module is being depended upon
    fn depended_upon(&self, dep: &ModuleQualifier) -> bool {
        // has installed dependent
        self.installed.loose_dependents(dep).iter()
            .any(|m| {
                // and not being uninstalled
                !self.remove.iter().any(|f| f.qualifier() == m.qualifier()) &&

                // and no one else does provide the same dependency
                // (need to redo the dependency check as in index, since we now know which are already removed)
                // see Index#specific_dependents() for the origins of this implementation
                m.dependencies().iter().any(|s| {
                    dep.does_provide(s) && // is always true at least once
                    !self.installed.providers(s).iter()
                        .filter(|m| !self.remove.iter().any(|c| c.qualifier() == m.qualifier()))
                        .any(|m| m.qualifier() != dep)
                })
            }) ||
        // dependent is being installed
        self.add.iter().any(|q| q.dependencies().iter().any(|d| dep.does_provide(d)))
    }

    /// Creates transactions for the resolved dependencies
    pub fn create_transactions(&self) -> Vec<ModuleTransaction> {
        let mut transactions = vec![];

        for change in &self.remove {
            if let Real(module) = change.change {
                // Skip those which were installed again
                if self.add.iter().any(|c| c.qualifier() == change.qualifier()) { continue; }

                transactions.push(ModuleTransaction::Remove(module.clone()))
            }
        }

        for change in &self.add {
            if let Real(module) = change.change {
                // Continue if module was removed and then installed again, but actually is installed
                if self.remove.iter().any(|m| *m.qualifier() == module.qualifier) &&
                    self.installed.get(module.qualifier()).is_some() { continue; }

                transactions.push(ModuleTransaction::Install(module.clone(), InstallReason::Dependency))
            }
        }

        transactions
    }
}

/// Represents a single change in modules that are installed
struct Change<'a, T: Indexable> {
    group: u32,
    change: ChangeType<'a, T>
}

/// Indicate whether a module change is real or only a marker
enum ChangeType<'a, T: Indexable> {
    Marker(ModuleQualifier, Vec<String>),
    Real(&'a T)
}

impl<'a, T> Change<'a, T> where T: Indexable {
    /// Marks a module as changed, which does not include it in the transactions afterwards
    fn mark(group: u32, module: &T) -> Self {
        Self {
            group,
            change: Marker(module.qualifier().clone(), module.dependencies().clone())
        }
    }

    /// Sets a module as changed, which will be included in transactions afterwards
    fn real(group: u32, module: &'a T) -> Self {
        Self {
            group,
            change: Real(module)
        }
    }

    /// Get dependencies of changed module
    fn dependencies(&self) -> &Vec<String> {
        match &self.change {
            Marker(_, deps) => { deps }
            Real(module) => { module.dependencies() }
        }
    }

    /// Get qualifier of changed module
    fn qualifier(&self) -> &ModuleQualifier {
        match &self.change {
            Marker(q, _) => { q }
            Real(module) => { module.qualifier() }
        }
    }
}