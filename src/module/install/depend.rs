use std::collections::{HashMap, HashSet};
use anyhow::{anyhow, Context};
use log::error;
use crate::module::install::build::install;
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

#[derive(Clone)]
pub enum ResolvingAction {
    Install,
    Reinstall,
    Update,
    Remove,
    Placeholder // means that the module is already installed and only a placeholder in the tree
}

#[derive(Default)]
pub struct Resolver {
    /// modules which are going to be installed or are updated
    dependency: HashMap<ModuleQualifier, Vec<ModuleQualifier>>,
    /// modules which are going to be removed
    removals: Vec<ModuleQualifier>,

    /// what module is what
    action: HashMap<ModuleQualifier, ResolvingAction>,
}

impl Resolver {

    /// mark a module as installed
    pub fn install(&mut self, module: &ModuleQualifier, local: &Index<InstalledModule>, available: &Index<Module>) -> anyhow::Result<()> {
        let module = available.get(&module).context("module disappeared unexpectedly")?;
        self.resolve(module, ResolvingAction::Install, local, available)
    }

    /// mark a module for update
    pub fn update(&mut self, module: &ModuleQualifier, local: &Index<InstalledModule>, available: &Index<Module>) -> anyhow::Result<()> {
        let module = available.get(&module).context("module disappeared unexpectedly")?;
        self.resolve(module, ResolvingAction::Update, local, available)
    }

    /// mark a module for reinstall
    pub fn reinstall(&mut self, module: &ModuleQualifier, local: &Index<InstalledModule>, available: &Index<Module>) -> anyhow::Result<()> {
        let module = available.get(&module).context("module disappeared unexpectedly")?;
        self.resolve(module, ResolvingAction::Reinstall, local, available)
    }

    /// mark a module for removal
    pub fn remove(&mut self, module: &ModuleQualifier, local: &Index<InstalledModule>) -> anyhow::Result<()> {
        if let Some(m) = local.specific_dependents(module).first() {
            println!("to be removed module {} is still being depended upon by {}", module.unique(), m.qualifier().unique());
            return Err(anyhow!("failed to resolve dependencies"))
        }

        self.removals.push(module.clone());
        self.action.insert(module.clone(), ResolvingAction::Remove);

        Ok(())
    }

    /// inserts a change into the list with the given action
    fn insert_change(&mut self, action: ResolvingAction, qualifier: ModuleQualifier, dependencies: Vec<ModuleQualifier>) {
        self.dependency.insert(qualifier.clone(), dependencies);
        self.action.insert(qualifier, action);
    }

    /// resolve a module and its dependencies
    fn resolve(&mut self, module: &Module, action: ResolvingAction, local: &Index<InstalledModule>, available: &Index<Module>) -> anyhow::Result<()> {

        let mut dependencies = vec![];

        // iterate over dependencies of module
        for dep in &module.dependencies {

            // check whether dependency is already in tree
            if let Some(q) = self.dependency.keys().find(|q| q.does_provide(&dep)) {
                dependencies.push(q.clone());
                continue
            }

            // check whether a possible module would be installed, select first installed if so
            let mut providers = local.providers(&dep);
            providers.sort_by(|a, b| a.built.time.cmp(&b.built.time));
            if let Some(first) = providers.first() {

                self.insert_change(ResolvingAction::Placeholder, first.qualifier().clone(), vec![]);
                dependencies.push(first.qualifier().clone());
                continue
            }

            // search through installable
            let mut providers = available.providers(&dep);
            if let Some(m) = prompt_choice_module(
                &providers,
                &format!("Multiple modules provide dependency '{dep}' for {}, choose:", module.qualifier.unique())).and_then(|i| providers.get(i).copied()) {

                self.resolve(m, ResolvingAction::Install, local, available)?;
                dependencies.push(m.qualifier().clone());

            } else {
                error!("failed to find module for dependency '{dep}' required by {}", module.qualifier.unique());
                return Err(anyhow!("could not resolve dependencies"));
            }
        }

        self.insert_change(action, module.qualifier().clone(), dependencies);

        Ok(())
    }



    /// collect the resolved modules into a single vector of the correct order
    pub fn collect(self) -> anyhow::Result<Vec<(ModuleQualifier, ModuleMotivation, ResolvingAction)>>{
        // calculate opposite of depends, because
        let mut because = HashMap::new();

        for (q, qualifiers) in &self.dependency {
            for dep in qualifiers {
                because.entry(dep).or_insert_with(|| vec![]).push(q.clone());
            }
        }

        // check that removals won't interfere
        for r in &self.removals {
            if self.dependency.contains_key(r) {
                error!("module {} is being removed but is being depended on by new modules", r.unique());
                return Err(anyhow!("failed to resolve dependencies"))
            }
        }

        let order = self.get_order();

        // get dfs ordering and clone modules
        Ok(self.removals.into_iter().chain(order)
            .map(|q| {
                let motivation = if self.dependency.contains_key(&q) {
                    ModuleMotivation {
                        because: because.get(&q).unwrap_or(&vec![]).clone(),
                        depends: self.dependency.get(&q).expect("should contain every node").clone()
                    }
                } else { ModuleMotivation::default() };

                let action = self.action.get(&q).expect("should contain every node").clone();

                (q, motivation, action)
            }).collect())
    }

    /// performs dfs on the tree
    fn get_order(&self) -> Vec<ModuleQualifier> {
        let mut post: HashMap<ModuleQualifier, i32> = HashMap::new();
        let mut counter = 0u32;

        while let Some(q) = self.find_source(&post) {
            self.visit(q, &mut post, &mut counter);
        }

        if post.len() != self.dependency.len() { unreachable!("all nodes should have been reached") }

        // collect post and sort with post order to get topological sorting
        let mut post: Vec<(ModuleQualifier, i32)> = post.into_iter().collect();
        post.sort_by(|(_, a), (_, b)| b.cmp(a));

        // remove installed modules
        post.retain(|(q, _)| !matches!(self.action.get(q), Some(ResolvingAction::Placeholder)));

        post.into_iter().map(|(q, _)| q).collect()
    }

    /// visits a node in dfs
    fn visit(&self, q: &ModuleQualifier, post: &mut HashMap<ModuleQualifier, i32>, counter: &mut u32) {
        *counter += 1; // technically for the pre but we don't need that
        post.insert(q.clone(), -1);

        for x in self.dependency.get(q).expect("node should be in tree") {
            if post.contains_key(x) { continue }
            self.visit(x, post, counter);
        }

        *counter += 1;
        post.insert(q.clone(), *counter as i32);
    }

    /// finds an unmarked source in dfs
    fn find_source(&self, post: &HashMap<ModuleQualifier, i32>) -> Option<&ModuleQualifier> {
        let mut depended_on = HashSet::new();

        for v in self.dependency.values() {
            for x in v {
                depended_on.insert(x);
            }
        }

        self.dependency.keys()
            .filter(|q| !post.contains_key(*q) && !depended_on.contains(q))
            .map(|q| q)
            .next()
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use crate::module::install::depend::{Resolver, ResolvingAction};
    use crate::module::qualifier::ModuleQualifier;

    /// tests the depth first search algorithm used for dependency resolving
    #[test]
    fn dfs() {
        fn qualifier(name: &str) -> ModuleQualifier{
            ModuleQualifier::new("q".to_string(), &PathBuf::from(name), None, None)
        }

        // graph from lecture lol
        let mut map = HashMap::new();
        map.insert(qualifier("a"), vec![qualifier("b"), qualifier("c"), qualifier("f")]);
        map.insert(qualifier("b"), vec![qualifier("e")]);
        map.insert(qualifier("c"), vec![qualifier("d")]);
        map.insert(qualifier("d"), vec![qualifier("h")]);
        map.insert(qualifier("e"), vec![qualifier("f"), qualifier("h"), qualifier("g")]);
        map.insert(qualifier("f"), vec![qualifier("g")]);
        map.insert(qualifier("g"), vec![]);
        map.insert(qualifier("h"), vec![qualifier("g")]);

        let resolver = Resolver {
            dependency: map,
            action: HashMap::new(),
            removals: vec![]
        };

        let order = resolver.get_order().into_iter().map(|q| q.name().clone()).collect::<Vec<String>>();

        assert_eq!(order, vec!["a", "c", "d", "b", "e", "h", "f", "g"]);
    }

    /// tests the depth first search algorithm used for dependency resolving, also checks that installed modules are not counted
    #[test]
    fn dfs_installed() {
        fn qualifier(name: &str) -> ModuleQualifier{
            ModuleQualifier::new("q".to_string(), &PathBuf::from(name), None, None)
        }

        // graph from lecture lol
        let mut map = HashMap::new();
        map.insert(qualifier("a"), vec![qualifier("b"), qualifier("c"), qualifier("f")]);
        map.insert(qualifier("b"), vec![qualifier("e")]);
        map.insert(qualifier("c"), vec![qualifier("d")]);
        map.insert(qualifier("d"), vec![qualifier("h")]);
        map.insert(qualifier("e"), vec![qualifier("f"), qualifier("h"), qualifier("g")]);
        map.insert(qualifier("f"), vec![qualifier("g")]);
        map.insert(qualifier("g"), vec![]);
        map.insert(qualifier("h"), vec![]);

        // these modules are already installed
        let mut installed = HashMap::new();
        installed.insert(qualifier("g"), ResolvingAction::Placeholder);
        installed.insert(qualifier("h"), ResolvingAction::Placeholder);

        let resolver = Resolver {
            dependency: map,
            action: installed,
            removals: vec![]
        };

        let order = resolver.get_order().into_iter().map(|q| q.name().clone()).collect::<Vec<String>>();

        assert_eq!(order, vec!["a", "c", "d", "b", "e", "f"]);
    }
}
