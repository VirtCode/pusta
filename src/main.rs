use std::env;
use std::path::PathBuf;
use log::LevelFilter;
use simplelog::{ColorChoice, TerminalMode, TermLogger};
use crate::command::{Command, ModuleCommand, RepositoryCommand, SubCommand};
use crate::config::Config;
use crate::manager::Manager;
use clap::Parser;

mod command;
mod module;
mod manager;
mod config;

fn main() {
    TermLogger::init(LevelFilter::Debug, simplelog::Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    let command: Command = Command::parse();

    let mut config = Config::read();

    let mut manager = Manager::load(&config);

    match command.topic {
        SubCommand::Module { action } => {

            match action {
                ModuleCommand::Install { module } => {

                    manager.install_module(&module).unwrap();

                }
                ModuleCommand::Remove { module } => {}
                ModuleCommand::Update { modules } => {}
            }

        },
        SubCommand::Repo { action } => {
            match action {
                RepositoryCommand::Add { path,  alias }  => {

                    // Add repository
                    let dir = if let Some(path) = path {
                        PathBuf::from(path)
                    } else {
                        env::current_dir().unwrap()

                    };

                    manager.add_repository(&dir, alias.as_ref()).unwrap();


                }
                RepositoryCommand::Remove { alias }  => {

                    // Remove repository
                    manager.remove_repository(&alias).unwrap();

                }
                RepositoryCommand::Main { alias } => {

                    if let Some(a) = alias {
                        // Set new main repository
                        config.repositories.main = Some(a);
                        config.write();
                    } else {
                        // Get main repository
                        if let Some(current) = &config.repositories.main { println!("The current main repository is '{}'", current); }
                        else { println!("There is no main repository set"); }
                    }

                }
            }
        },
        _ => {}
    }
}
