use log::{debug, error, info, warn};
use crate::config::Config;
use crate::jobs::BuiltJob;
use crate::module::installed::build::ModuleInstructions;
use crate::module::installed::neodepend::ModuleMotivation;
use crate::module::Module;
use crate::module::transaction::change::{AtomicChange, ChangeError};
use crate::module::transaction::worker::WorkerPortal;

pub fn run(instructions: &Vec<(&ModuleInstructions, &Module, &ModuleMotivation)>, config: &Config) -> Vec<Option<bool>>{

    info!("Spawning workers...");
    let mut workers = WorkerPortal::open()?;

    debug!("Spawning non-root worker");
    workers.summon(false, &config.system.root_elevator)?;

    // check if any root jobs are present
    if instructions.iter()
        .any(|i| {
            if let Some(new) = i.new {
                new.jobs.iter().zip(i.apply).any(|(j, b)| b && j.root)
            } else { false } ||
                if let Some(old) = i.old {
                    old.jobs.iter().zip(i.revert).any(|(j, b)| b && j.root)
                } else { false }
        }) {

        debug!("Spawning root worker");
        workers.summon(true, &config.system.root_elevator)?;
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

        // revert revertible changes
        if let Some(module) = &instruction.old {
            debug!("Reverting old changes");
            let jobs = module.jobs.iter()
                .zip(&instruction.revert)
                .filter_map(|(j, exec)| if exec { Some(j) } else { None }).collect();

            match revert_jobs(jobs, &mut workers) {
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

            let jobs = module.jobs.iter()
                .zip(&instruction.apply)
                .filter_map(|(j, exec)| if exec { Some(j) } else { None }).collect();

            match apply_jobs(jobs, &mut workers) {
                Ok(true) => {}
                Ok(false) => {
                    error!("Apply steps for module {} did not go gracefully, removing its dependencies again", source.qualifier.unique());
                    results[index] = Some(false);
                    failed.push(source.qualifier.clone());

                    // revert dependencies of this failure
                    for (index, (instruction, module, motivation)) in instructions[0..index].iter().enumerate().rev() {
                        if !motivation.no_longer_satisfied(&failed) { continue }

                        results[index] = Some(false);

                        match revert_jobs(jobs, &mut workers) {
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

    results
}

/// Applies a list of jobs
fn apply_jobs(jobs: &[&BuiltJob], portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    for (index, job) in jobs.iter().rev().enumerate() {
        let result = apply_changes(&job.changes, job.root, portal)?;

        if !result {
            debug!("Reverting previous jobs");
            revert_jobs(&jobs[0..index], portal)?;
        }
    }

    Ok(true)
}

/// Applies a list of changes
fn apply_changes(changes: &[Box<dyn AtomicChange>], root: bool, portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    for (index, change) in changes.iter().enumerate() {
        debug!("Dispatching change '{}', root: {root}", change.describe());

        let result = portal.dispatch(change, root, false)?;
        if let Err(e) = result {
            process_change_error(change, root, e, true);

            debug!("Reverting previous changes of job");
            revert_changes(&changes[0..index], root, portal)?;
            Ok(false)
        }
    }

    Ok(true)
}

/// Reverts a list of jobs
fn revert_jobs(jobs: &[&BuiltJob], portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    let mut graceful = true;

    for job in jobs.iter().rev() {
        let result = revert_changes(&job.changes, job.root, portal)?;
        if !result { graceful = false }
    }

    Ok(graceful)
}

/// Reverts a list of changes
fn revert_changes(changes: &[Box<dyn AtomicChange>], root: bool, portal: &mut WorkerPortal) -> anyhow::Result<bool> {
    let mut graceful = true;

    for change in changes.iter().rev() {
        debug!("Dispatching reversal of change '{}', root: {root}", change.describe());

        let result = portal.dispatch(change, root, false)?;
        if let Err(e) = result {
            process_change_error(change, root, e, false);
            graceful = false;
        }
    }

    Ok(graceful)
}

fn process_change_error(change: &Box<dyn AtomicChange>, root: bool, error: ChangeError, apply: bool) {
    if apply {
        error!("Failed to apply the change '{}'{}", change.describe(), if root { "as root" } else {""});
        error!("> {}", error.to_string());
    } else {
        warn!("Failed to revert the change '{}'{}", change.describe(), if root { "as root" } else {""});
        warn!("> {}", error.to_string());
        warn!("Change may have left unwanted traces on your system.");
    }
}
