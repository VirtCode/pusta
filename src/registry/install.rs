use anyhow::{anyhow, Context};
use log::{error, info};
use solvent::DepGraph;
use crate::module::install::InstalledModule;
use crate::module::Module;
use crate::module::qualifier::{DependencyQualifier, ModuleQualifier};
use crate::registry::depend::DependencyResolver;
use crate::registry::index::{Index, Indexable};
use crate::registry::neodepend::{DependencyInfo, Resolver};

pub enum Gathered<'a> {
    Install {
        module: &'a Module,
    },
    Remove {
        module: &'a InstalledModule,
    },
    Update {
        old: &'a InstalledModule,
        new: &'a Module,
    }
}

#[derive(Default)]
struct GatherInstaller<'a> {
    gathered: Vec<Gathered<'a>>
}

impl GatherInstaller {

    pub fn install(&mut self, module: &Module) {
        self.gathered.push(Gathered::Install { module });
    }

    fn update(&mut self, old: &InstalledModule, module: &Module) {
        self.gathered.push(Gathered::Update { old, new: module })
    }

    fn reinstall(&mut self, old: &InstalledModule, module: &Module) {
        self.gathered.push(Gathered::Remove { module: old, });
        self.gathered.push(Gathered::Install { module })
    }

    fn remove(&mut self, module: &InstalledModule) {
        self.gathered.push(Gathered::Remove { module })
    }

    fn gather(self: Self, index: &Index<Module>, local: &Index<InstalledModule>) -> anyhow::Result<PrepareInstaller> {
        let mut modules = vec![];

        info!("Resolving dependencies...");
        let mut resolver = Resolver::default();

        for gathered in self.gathered {
            match gathered {
                Gathered::Install { module } => {
                    modules.append(&mut resolver.resolve(module, local, index)?.into_iter().map(|(m, i)| {
                        Scheduled::Install { module: m, deps: i}
                    }).collect());
                }
                Gathered::Remove { module } => {
                    // TODO: Check dependencies and Free modules
                    if resolver.can_remove(module.qualifier()) {
                        modules.push(Scheduled::Remove { module: module.clone() })
                    } else {
                        return Err(anyhow!("screw dependencies"));
                    }
                }
                Gathered::Update { old, new } => {
                    // TODO: Check dependencies and free modules
                    modules.push(Scheduled::Update { old: old.clone(), new: new.clone() })
                }
            }
        }

        Ok(PrepareInstaller { scheduled: modules })
    }
}

pub enum Scheduled {
    Install {
        module: Module,
        deps: DependencyInfo
    },
    Remove {
        module: InstalledModule,
    },
    Update {
        old: InstalledModule,
        new: Module,
    }
}

struct PrepareInstaller {
    scheduled: Vec<Scheduled>
}

impl PrepareInstaller {
    fn prepare(&self) {

    }

}

