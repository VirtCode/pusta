use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::module::repository::Repository;

/// This trait marks a module struct as indexable. This is used to expose qualifier and dependencies of a module so it can be indexed properly.
pub trait Indexable {
    /// Gets the dependencies of the module
    fn dependencies(&self) -> &Vec<String>;
    /// Gets the qualifier of the module
    fn qualifier(&self) -> &ModuleQualifier;
}

/// This struct indexes indexables, aka modules, so one can query them in all sorts of manners
pub struct Index<T> where T: Indexable {
    pub modules: Vec<T>
}

impl<T> Index<T> where T: Indexable {
    
    /// Creates a new index
    pub fn new() -> Self {
        Self { modules: vec![] }
    }

    /// Returns a list of modules which are matched by a given query
    pub fn query(&self, query: &str) -> Vec<&T> {

        self.modules.iter()
            .filter(|m| {
                if query.contains('/') {
                    m.qualifier().unique() == query
                } else {
                    m.qualifier().name() == query
                }
            })
            .collect()

    }

    /// Returns a list of modules which provide a certain dependency
    pub fn providers(&self, dependency: &str) -> Vec<&T> {
        self.modules.iter()
            .filter(|m| m.qualifier().does_provide(dependency))
            .collect()
    }

    /// Returns a list of modules which MAY depend on a given module.
    /// The critical dependency of any module returned here MAY also be fulfilled by another module (but doesn't have to).
    /// For a specific list, see specific_dependents
    pub fn loose_dependents(&self, dependency: &ModuleQualifier) -> Vec<&T> {
        self.modules.iter()
            .filter(|m| {
                // Avoid modules that depend on themselves
                m.qualifier() != dependency &&

                m.dependencies().iter().any(|s| dependency.does_provide(s))
            })
            .collect()
    }

    /// Returns a list of modules which SPECIFICALLY depend on a given module.
    pub fn specific_dependents(&self, dependency: &ModuleQualifier) -> Vec<&T> {
        self.modules.iter()
            .filter(|m| {
                // Avoid modules that depend on themselves
                m.qualifier() != dependency &&

                // Check every dependency whether it is provided and there are no other providers
                m.dependencies().iter().any(|s| {
                    dependency.does_provide(s) &&
                    !self.providers(s).iter().any(|m| m.qualifier() != dependency) // Ignore checked dependency
                })
            })
            .collect()
    }

    /// Returns a module for the given qualifier
    pub fn get(&self, qualifier: &ModuleQualifier) -> Option<&T> {
        self.modules.iter().find(|m| m.qualifier() == qualifier)
    }

    /// Adds a module or replaces a given one if needed
    pub fn add(&mut self, module: T) {
        // Remove possible duplicates
        self.remove(module.qualifier());

        self.modules.push(module);
    }

    /// Adds all modules and replaces duplicates if present
    pub fn add_all(&mut self, modules: Vec<T>) {
        for module in modules {
            self.add(module)
        }
    }
    
    /// Removes a module from the index if present
    pub fn remove(&mut self, qualifier: &ModuleQualifier) -> Option<T> {
        self.modules.iter().position(|f| f.qualifier() == qualifier)
            .map(|pos| self.modules.remove(pos))
    }
}

/// This provides some special methods only used to interface with the global index
impl Index<Module> {
    /// Unloads all modules belonging to an added repository
    pub fn remove_repository(&mut self, repo: &Repository) {
        self.modules.retain(|r| *r.qualifier.repository() != repo.name);
    }
}