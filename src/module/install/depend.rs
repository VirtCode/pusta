use anyhow::anyhow;
use log::error;
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::qualifier::{ModuleQualifier};
use crate::output::prompt_choice_module;
use crate::registry::index::{Index, Indexable};

#[derive(Default)]
pub struct ModuleMotivation {
    pub because: Vec<ModuleQualifier>,
    pub depends: Vec<ModuleQualifier>
}

impl ModuleMotivation {
    pub fn no_longer_satisfied(&self, failed: &Vec<ModuleQualifier>) -> bool {
        // either it depends on something failed
        self.depends.iter().any(|q| failed.contains(q)) ||
            // or it was only installed because of the failed item
            (!self.because.is_empty() && !self.because.iter().any(|q| !failed.contains(q)))
    }

}

#[derive(Default)]
pub struct Resolver {
    // dependencies which are installed and still used
    used: Vec<String>,
    // dependencies which will be installed
    installed: Vec<ModuleQualifier>
}

impl Resolver {
    pub fn resolve(&mut self, module: &Module, local: &Index<InstalledModule>, available: &Index<Module>) -> anyhow::Result<Vec<(Module, ModuleMotivation)>> {
        let mut modules = vec![];
        let mut dependencies = vec![];

        for dep in &module.dependencies {
            // not already installed
            if local.providers(dep).is_empty() {

                // see if dependency was already installed
                if let Some(a) = self.installed.iter().find(|i| i.does_provide(dep)) {
                    dependencies.push(a.clone());
                    continue;
                }

                // find and install dependency
                let providers = available.providers(dep);
                let provider = prompt_choice_module(&providers, &format!("Choose provider for dependency '{dep}'"));

                if let Some(index) = provider {
                    let provider = providers.get(index).expect("prompt did not return expected index");
                    dependencies.push(provider.qualifier.clone());

                    // resolve dependencies for provider
                    let mut provider_deps = self.resolve(*provider, local, available)?;
                    for (module, info) in &mut provider_deps {
                        info.because.push(module.qualifier.clone());
                    }

                    // append new dependencies
                    modules.append(&mut provider_deps);

                } else {
                    error!("Could not find dependency '{dep}' for module {}", module.qualifier.unique());
                    return Err(anyhow!("could not resolve dependencies"));
                }

            } else {
                // set installed dependency to used
                self.used.push(dep.clone())
            }
        }

        self.installed.push(module.qualifier.clone());
        modules.push((module.clone(), ModuleMotivation {
            because: vec![],
            depends: dependencies,
        }));

        Ok(modules)
    }

    pub fn can_remove(&self, module: &ModuleQualifier) -> bool {
        !self.used.iter().any(|s| module.does_provide(s))
    }
}

