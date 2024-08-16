use std::path::Path;
use log::{debug, error, info, warn};
use crate::config::Config;
use crate::jobs::BuiltJob;
use crate::module::install::build::ModuleInstructions;
use crate::module::install::depend::ModuleMotivation;
use crate::module::Module;
use crate::module::change::{AtomicChange, ChangeError};
use crate::module::change::worker::WorkerPortal;
use crate::registry::cache::Cache;

pub(super) fn run(instructions: &Vec<(&ModuleInstructions, &Module, &ModuleMotivation)>, config: &Config, cache: &Cache) -> anyhow::Result<Vec<Option<bool>>> {

    info!("Spawning workers...");
    let mut workers = WorkerPortal::open()?;

    debug!("Spawning non-root worker");
    workers.summon(false, &config.system.root_elevator, config.system.clean_terminal)?;

    // check if any root jobs are present
    if instructions.iter()
        .any(|(i, _, _)| {
            let removal = if let Some(new) = &i.new {
                new.jobs.iter().zip(&i.apply).any(|(j, b)| *b && j.root)
            } else { false };
            let apply = if let Some(old) = &i.old {
                old.jobs.iter().zip(&i.revert).any(|(j, b)| *b && j.root)
            } else { false };

            removal || apply
        }) {

        debug!("Spawning root worker");
        workers.summon(true, &config.system.root_elevator, config.system.clean_terminal)?;
    }

    let mut results = vec![None; instructions.len()];
    let mut failed = vec![];

    info!("Applying changes...");
    'install: for (index, (instruction, source, motivation)) in instructions.iter().enumerate() {
        info!("Processing module {}", source.qualifier.unique());
        results[index] = Some(true);

        if motivation.no_longer_satisfied(&failed) {
            results[index] = Some(false);
            info!("Skipping because of failed reason or dependency");
            continue;
        }

        let cache = match cache.get_module_cache(*source) {
            Ok(p) => {p}
            Err(e) => {
                error!("Fatal error occurred whilst creating cache directory: {e}");
                results[index] = None;
                break 'install;
            }
        };

        // revert revertible changes
        if let Some(module) = &instruction.old {
            debug!("Reverting old changes");
            let jobs: Vec<&BuiltJob> = module.jobs.iter()
                .zip(&instruction.revert)
                .filter_map(|(j, exec)| if *exec { Some(j) } else { None }).collect();

            match revert_jobs(&jobs, &cache, &mut workers) {
                Ok(true) => {}
                Ok(false) => {
                    warn!("Reversal steps for module {} did not go gracefully", source.qualifier.unique())
                }
                Err(e) => {
                    error!("Fatal error occurred whilst applying modules: {e}");
                    results[index] = Some(false);
                    break 'install;
                }
            }
        }

        // apply changes
        if let Some(module) = &instruction.new {
            debug!("Applying new changes");

            let jobs: Vec<&BuiltJob> = module.jobs.iter()
                .zip(&instruction.apply)
                .filter_map(|(j, exec)| if *exec { Some(j) } else { None }).collect();

            match apply_jobs(&jobs, &cache, &mut workers) {
                Ok(true) => {}
                Ok(false) => {
                    error!("Apply steps for module {} did not go gracefully, removing its dependencies again", source.qualifier.unique());
                    results[index] = Some(false);
                    failed.push(source.qualifier.clone());

                    // revert dependencies of this failure
                    for (index, (instruction, module, motivation)) in instructions[0..index].iter().enumerate().rev() {
                        if !motivation.no_longer_satisfied(&failed) { continue }

                        results[index] = Some(false);

                        let install_jobs: Vec<&BuiltJob> = instruction.new.as_ref().map(|m| m.jobs.iter().collect()).unwrap_or_default();
                        match revert_jobs(&install_jobs, &cache, &mut workers) {
                            Ok(true) => {}
                            Ok(false) => {
                                warn!("Reversal steps because of dependency failure for module {} did not go gracefully", source.qualifier.unique())
                            }
                            Err(e) => {
                                error!("Fatal error occurred whilst applying modules: {e}");
                                break 'install;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Fatal error occurred whilst applying modules: {e}");
                    results[index] = Some(false);
                    break 'install;
                }
            }
        }
    }

    Ok(results)
}

/// Applies a list of jobs
fn apply_jobs(jobs: &[&BuiltJob], cache :&Path, portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    for (index, job) in jobs.iter().enumerate() {
        let result = apply_changes(&job.changes, job.root, cache, portal)?;

        if !result {
            debug!("Reverting previous jobs");
            revert_jobs(&jobs[0..index], cache, portal)?;
            return Ok(false)
        }
    }

    Ok(true)
}

/// Applies a list of changes
fn apply_changes(changes: &[Box<dyn AtomicChange>], root: bool, cache: &Path, portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    for (index, change) in changes.iter().enumerate() {
        debug!("Dispatching change '{}', root: {root}", change.describe());

        let result = portal.dispatch(change, root, cache, true)?;
        if let Err(e) = result {
            process_change_error(change, root, e, true);

            debug!("Reverting previous changes of job");
            revert_changes(&changes[0..index], root, cache, portal)?;
            return Ok(false)
        }
    }

    Ok(true)
}

/// Reverts a list of jobs
fn revert_jobs(jobs: &[&BuiltJob], cache: &Path, portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    let mut graceful = true;

    for job in jobs.iter().rev() {
        let result = revert_changes(&job.changes, job.root, cache, portal)?;
        if !result { graceful = false }
    }

    Ok(graceful)
}

/// Reverts a list of changes
fn revert_changes(changes: &[Box<dyn AtomicChange>], root: bool, cache: &Path, portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    let mut graceful = true;

    for change in changes.iter().rev() {
        debug!("Dispatching reversal of change '{}', root: {root}", change.describe());

        let result = portal.dispatch(change, root, cache, false)?;
        if let Err(e) = result {
            process_change_error(change, root, e, false);
            graceful = false;
        }
    }

    Ok(graceful)
}

fn process_change_error(change: &Box<dyn AtomicChange>, root: bool, error: ChangeError, apply: bool) {
    if apply {
        error!("Failed to apply the change{}: {}", if root { "(as root)" } else {""}, change.describe());
        error!("  {}", error.to_string());
    } else {
        warn!("Failed to revert the change{}: {}", if root { "(as root)" } else {""}, change.describe());
        warn!("  {}", error.to_string());
        warn!("Change may have left unwanted traces on your system.");
    }
}
