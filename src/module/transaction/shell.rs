use std::{env, thread};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread::JoinHandle;

/// Represents a result of a ran shell
pub type RunResult = Result<RunData, String>;

/// Stores the data retrieved from a ran shell
pub struct RunData {
    /// Exit status
    pub status: ExitStatus,
    /// Captured stdout of the shell
    pub stdout: String,
    /// Captured stderr of the shell
    pub stderr: String
}

/// Runs a given command, capturing and printing the output.
fn run(mut command: Command, interactive: bool) -> RunResult {

    // Set output settings
    command.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if interactive { Stdio::inherit() } else { Stdio::null() });

    let mut child = command.spawn().map_err(|_| "could not invoke command".to_string())?;

    let stdout = child.stdout.take().expect("stdout is always captured");
    let stdout_join = read_output_parallel(stdout, interactive, "    ");

    let stderr = child.stderr.take().expect("stderr is always captured");
    let stderr_join = read_output_parallel(stderr, interactive, "err:");

    let status = child.wait().map_err(|_| "command did not run when expected to".to_string())?;
    let stdout = stdout_join.join().map_err(|_| "could not properly read stdout".to_string())?;
    let stderr = stderr_join.join().map_err(|_| "could not properly read stderr".to_string())?;

    Ok(RunData { status, stdout, stderr })
}

/// Returns the executable of the shell to use
fn shell_executable() -> String {
    env::var("SHELL").unwrap_or_else(|_| "sh".into())
}

/// Runs a command on the shell
pub fn run_command(command: &str, dir: &Path, interactive: bool) -> RunResult {
    let mut c = Command::new(shell_executable());
    c.current_dir(dir).arg("-c").arg(command);

    run(c, interactive)
}

/// Runs a script with the shell
pub fn run_script(path: &Path, dir: &Path, interactive: bool) -> RunResult {
    let mut c = Command::new(shell_executable());
    c.current_dir(dir).arg(path.as_os_str());

    run(c, interactive)
}

/// Reads the output of a given stream to a string and may print it to the console in the process
fn read_output_parallel<T: Read + Send + 'static>(output: T, print: bool, prefix: &str) -> JoinHandle<String> {
    let prefix = prefix.to_owned();

    thread::spawn(move || {
        let mut buffer = String::new();

        BufReader::new(output)
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|l| {
                buffer.push_str(&l);
                if print { println!("{prefix}{l}") };
            });

        buffer
    })
}