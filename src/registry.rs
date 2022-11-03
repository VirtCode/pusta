use std::collections::HashMap;
use std::path::PathBuf;
use crate::module::Module;
use crate::repository::{Repository};

pub const MAIN: &str = "main";

pub struct Registry {
    strict: bool,

    main: Option<String>,
    repositories: HashMap<String, Repository>
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            strict: false,
            main: None,
            repositories: HashMap::new()
        }
    }

    pub fn add(&mut self, repo: &PathBuf) -> anyhow::Result<()>{
        let repo = Repository::load(repo)?;

        self.repositories.insert(repo.name.clone(), repo);

        Ok(())
    }

    pub fn get_repository(&self, repo: &str) -> Option<&Repository> {

        let repo = if let Some(main) = &self.main {
            main.as_str()
        } else {
            repo
        };

        self.repositories.get(repo)
    }

    pub fn get_from(&self, repo: &str, qualifier: &str) -> Option<&Module> {
        let repo = if repo == MAIN && self.main.is_some() {
             self.main.clone().unwrap()
        } else { repo.to_string() };

        self.repositories.get(&repo).and_then(|r| r.module(qualifier))
    }

    pub fn provider(&self, qualifier: &str) -> Vec<&Module> {
        let mut vec = vec![];

        for (_, repo) in &self.repositories {
            vec.append(&mut repo.provider(qualifier))
        }

        vec
    }

    pub fn get(&self, qualifier: &str) -> Option<&Module> {

        let parts: Vec<&str> = qualifier.split('/').collect();

        if parts.len() == 2 {
            // Explicit repository
            self.get_from(parts.first().unwrap(), parts.get(1).unwrap())

        } else if parts.len() == 1 {

            let main = self.get_from(MAIN, parts.first().unwrap());

            if self.strict {
                // Strict, always meaning main if no explicit repo
                main
            } else {
                // Non-Strict, using first best repo
                if main.is_none() {
                    for (_, repo) in &self.repositories {
                        let possible = repo.module(parts.first().unwrap());
                        if possible.is_some() { return possible }
                    }

                    None

                } else { main }
            }

        } else { None }
    }
}