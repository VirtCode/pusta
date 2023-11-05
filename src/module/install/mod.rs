use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use std::fs;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use std::time::SystemTime;
use anyhow::{anyhow, Context};
use colored::Colorize;
use log::{error, info};
use serde::{Deserialize, Serialize};
use crate::config::Config;
use crate::jobs::BuiltJob;
use crate::module::install::build::{BuiltModule, ModuleEnvironment, ModuleInstructions};
use crate::module::install::depend::{ModuleMotivation, Resolver};
use crate::module::Module;
use crate::module::qualifier::ModuleQualifier;
use crate::output::prompt;
use crate::registry::cache::Cache;
use crate::registry::index::{Index, Indexable};
use crate::variables::{generate_magic, load_system, Variable};

mod build;
pub mod depend;
mod run;

/// This struct stores a gathered change
enum Gathered {
    Install(ModuleQualifier),
    Remove(ModuleQualifier),
    Update(ModuleQualifier)
}

/// This struct helps gathering module changes
#[derive(Default)]
pub struct Gatherer {
    gathered: Vec<Gathered>
}

impl Gatherer {

    pub fn install(&mut self, module: ModuleQualifier) {
        self.gathered.push(Gathered::Install(module));
    }

    pub fn update(&mut self, module: ModuleQualifier) {
        self.gathered.push(Gathered::Update(module))
    }

    pub fn reinstall(&mut self, module: ModuleQualifier) {
        self.gathered.push(Gathered::Remove(module.clone()));
        self.gathered.push(Gathered::Install(module));
    }

    pub fn remove(&mut self, module: ModuleQualifier) {
        self.gathered.push(Gathered::Remove(module));
    }

    fn gather(self: Self, index: &Index<Module>, local: &Index<InstalledModule>) -> anyhow::Result<Vec<Scheduled>> {
        let mut modules = vec![];

        info!("Resolving dependencies...");
        let mut resolver = Resolver::default();

        for gathered in self.gathered {
            match gathered {
                Gathered::Install(module) => {
                    let module = index.get(&module).context("module disappeared unexpectedly")?;

                    modules.append(&mut resolver.resolve(module, local, index)?.into_iter().map(|(m, i)| {
                        Scheduled::Install { module: m, motivation: i, }
                    }).collect());
                }
                Gathered::Remove(module) => {
                    let module = local.get(&module).context("installed module disappeared unexpectedly")?;

                    // TODO: Check dependencies and Free modules
                    if resolver.can_remove(module.qualifier()) {
                        modules.push(Scheduled::Remove { module: module.clone() })
                    } else {
                        return Err(anyhow!("screw dependencies"));
                    }
                }
                Gathered::Update(module) => {
                    let old = local.get(&module).context("installed module disappeared unexpectedly")?;
                    let new = index.get(&module).context("module disappeared unexpectedly")?;

                    // TODO: Check dependencies and free modules
                    modules.push(Scheduled::Update { old: old.clone(), new: new.clone() })
                }
            }
        }

        Ok(modules)
    }
}

enum Scheduled {
    Install {
        module: Module,
        motivation: ModuleMotivation
    },
    Remove {
        module: InstalledModule,
    },
    Update {
        old: InstalledModule,
        new: Module,
    }
}

/// Holds an installed instance of a module
#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledModule {
    pub module: Module,
    pub built: BuiltModule,
}

impl Indexable for InstalledModule {
    fn dependencies(&self) -> &Vec<String> { self.module.dependencies() }

    fn qualifier(&self) -> &ModuleQualifier { self.module.qualifier() }
}

/// Indicates what a modification is doing
enum ModifyType {
    Install, Remove, Update
}
impl Display for ModifyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModifyType::Install => { f.write_str("installation") }
            ModifyType::Remove => { f.write_str("removal") }
            ModifyType::Update => { f.write_str("update") }
        }
    }
}

/// Builds the changes to a uniform format
fn build(scheduled: Vec<Scheduled>, cache: &Cache, config: &Config) -> anyhow::Result<Vec<(Module, ModuleInstructions, ModuleMotivation, ModifyType)>>{
    info!("Building modules...");
    let mut built = vec![];

    let env = ModuleEnvironment {
        package_config: config.system.package_manager.clone(),
        magic_variables: generate_magic(),
        system_variables: load_system(config).unwrap_or_else(|| Variable::base())
    };

    for scheduled in scheduled {
        built.push(match scheduled {
            Scheduled::Install { module, motivation } => {
                let repository = cache.get_repository(module.qualifier.repository()).expect("module from no repository");
                let built = build::install(&module, repository, &env)?;
                (module, built, motivation, ModifyType::Install)
            }
            Scheduled::Remove { module } => {
                let core = module.module.clone();
                let built = build::remove(module)?;
                (core, built, ModuleMotivation::default(), ModifyType::Remove)
            }
            Scheduled::Update { old, new } => {
                let repository = cache.get_repository(new.qualifier.repository()).expect("module from no repository");
                let built = build::update(old, &new, repository, &env)?;
                (new, built, ModuleMotivation::default(), ModifyType::Update)
            }
        });
    }

    Ok(built)
}

/// Asks the user whether the changes should be applied, can enter a detailed view if required
fn ask(changes: &Vec<(Module, ModuleInstructions, ModuleMotivation, ModifyType)>, previewer: &str) -> bool {
    info!("Scheduled module changes:");
    for (m, _, _, what) in changes {
        match what {
            ModifyType::Install => { info!("    {} ({}-{})", m.name.bold(), m.qualifier.unique(), m.version.dimmed()) }
            ModifyType::Remove => { info!("    {} ({}-{})", m.name.bold(), m.qualifier.unique(), "removal".dimmed())  }
            ModifyType::Update => { info!("    {} ({}-{})", m.name.bold(), m.qualifier.unique(), m.version.dimmed())  }
        }
    }

    loop {
        let response = prompt("Apply changes or fine grained view? [Y/n/f] ").to_lowercase();
        let response = response.trim();

        if response.starts_with("y") || response.is_empty() { return true; }
        if response.starts_with("n") { return false; }
        if response.starts_with("f") { break; }
    }

    // fine grained view
    info!("{}", "These specific changes are going to be applied:".bold());

    let mut file_map = HashMap::new();

    // will be used to print jobs to console
    fn print_jobs(built: &Vec<BuiltJob>, really: &Vec<bool>, apply: bool, file_map: &mut HashMap<usize, String>) {
        for job in built.iter().zip(really)
            .filter_map(|(j, r)| if *r { Some(j) } else { None }) {

            for change in &job.changes {
                let files = change.files().into_iter().map(|(name, data)| {
                    let id = file_map.len() + 1;
                    let entry = format!("{} ({})", id, &name);
                    file_map.insert(id, data);
                    entry
                }).collect::<Vec<String>>();
                let files = if files.is_empty() { "".to_string() } else { format!(", content: {}", files.join(", ")) };

                info!("    {}", change.describe());
                info!("        action: {}{}{}",
                    if apply { "applying" } else { "reverting" }, files,
                    if job.root { "root".bright_red().bold().to_string() } else { "".to_string() })
            }
        }
    }

    for (module, built, motivation, what) in changes {
        info!("Module: {}", module.qualifier.unique().bold());
        info!("        type: {}{}{}", what, {
                if !motivation.depends.is_empty() {
                    let string = motivation.depends.iter().map(|q| q.unique()).collect::<Vec<String>>().join(", ");
                    format!(", depends on: {}", string.italic())
                } else { String::new() }
            }, {
                if !motivation.because.is_empty() {
                    let string = motivation.because.iter().map(|q| q.unique()).collect::<Vec<String>>().join(", ");
                    format!(", because of: {}", string.italic())
                } else { String::new() }
            });

        if let Some(old) = &built.old {
            print_jobs(&old.jobs, &built.revert, false, &mut file_map);
        }

        if let Some(new) = &built.new {
            print_jobs(&new.jobs, &built.apply, true, &mut file_map);
        }

        info!("");
    }

    loop {
        let response = prompt("Apply changes or preview content? [Y/n/content] ").to_lowercase();
        let response = response.trim();

        if response.starts_with("y") || response.is_empty() { break true; }
        if response.starts_with("n") { break false; }

        if let Some(data) = usize::from_str(&response).ok().and_then(|id| file_map.get(&id)) {
            if let Err(e) = preview_file(&previewer, data) { error!("failed to open preview: {}", e); }
        }
    }
}

/// Saves the changes to the disk
fn save(changes: Vec<(Module, ModuleInstructions, ModuleMotivation, ModifyType)>, result: Vec<Option<bool>>, cache: &mut Cache) -> anyhow::Result<()> {
    info!("Persisting changes to disk...");

    for ((module, instr, _, t), real) in changes.into_iter().zip(result).filter_map(|(t, result)| result.map(|b| (t,b))) {
        if !real {
            // failed
            cache.remove_module(module.qualifier())?;
            cache.delete_module_cache(&module)?;

        } else  {
            // success
            match t {
                ModifyType::Install | ModifyType::Update => {
                    let built = instr.new.expect("installed module should contain this");

                    let installed = InstalledModule {
                        module, built,
                    };

                    cache.install_module(installed)?;
                }
                ModifyType::Remove => {
                    cache.remove_module(module.qualifier())?;
                    cache.delete_module_cache(&module)?;
                }
            }
        }
    }

    Ok(())
}

pub fn modify(gatherer: Gatherer, index: &Index<Module>, cache: &mut Cache, config: &Config) {
    // 1. gather
    let scheduled = match gatherer.gather(index, &cache.index) {
        Ok(s) => { s }
        Err(e) => {
            error!("failed to resolve dependencies: {e}");
            return;
        }
    };

    // 2. build
    let built = match build(scheduled, &cache, config) {
        Ok(b) => { b }
        Err(e) => {
            error!("{e}");
            return;
        }
    };

    // 3. ask
    if !ask(&built, &config.system.file_previewer) {
        error!("installation cancelled by user");
        return;
    }

    // 4. run
    let result = match run::run(&built.iter().map(|(m, i, mo, _)| (i, m, mo)).collect(), &config, &cache) {
        Ok(r) => { r }
        Err(e) => {
            error!("failed to initiate run: {e}");
            return;
        }
    };

    // 5. save
    if let Err(e) = save(built, result, cache) {
        error!("failed to save changes to disk: {e}");
        error!("pusta will not remember that these changes were made");
        return;
    }
}


const TMP_PREVIEW_PATH: &str = "/tmp/pusta/preview";

/// Previews data in a file previewer, by first saving it to disk
pub fn preview_file(previewer: &str, data: &str) -> anyhow::Result<()>{
    fs::create_dir_all("/tmp/pusta")?;

    fs::write(Path::new(TMP_PREVIEW_PATH), data)?;

    let mut command = Command::new(previewer);
    command.arg(TMP_PREVIEW_PATH);

    command.spawn()?.wait()?;

    Ok(())
}