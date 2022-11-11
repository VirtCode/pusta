use std::io::{BufRead, BufReader};
use std::{env, process};
use std::process::{Command, Stdio};
use log::{debug, warn};
use crate::output;
use crate::output::loading::Loading;

const FALLBACK_SHELL: &str = "/bin/sh";

pub fn run(exec: &str, output: bool) -> anyhow::Result<bool> {
    let shell = env::var("SHELL").unwrap_or_else(|_| { warn!("Current shell ($SHELL) is not defined, using {}", FALLBACK_SHELL); FALLBACK_SHELL.to_string()});

    let mut command = Command::new(&shell);
    command.stdout(Stdio::piped()).stdin(Stdio::inherit());
    command.arg("-c").arg(exec);

    debug!("Running shell command '{}' on {}", exec, shell);
    let mut result = command.spawn()?;

    if output {
        let reader = BufReader::new(result.stdout.as_mut().unwrap());
        for str in reader.lines() {
            output::print_shell(&str?)
        }
    }

    Ok(result.wait()?.success())
}

pub fn run_task(command: &str, message: &str, success_message: &str, failure_message: &str) -> anyhow::Result<bool> {
    //let mut spinner = Loading::start(message);
    output::start_section(message);

    let result = run(command, true);
    match &result {
        Ok(_) => {
            output::end_section()
            //spinner.stop(true, success_message);
        }
        Err(_) => {
            output::end_section()
            //spinner.stop(false, failure_message)
        }
    }

    result
}