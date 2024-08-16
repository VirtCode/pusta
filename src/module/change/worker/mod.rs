pub mod run;

use std::collections::HashMap;
use std::{env, fs, io, thread};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use anyhow::{anyhow, Context};
use lazy_regex::{Lazy, lazy_regex};
use log::{debug, error, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::module::change::{AtomicChange, ChangeResult, ChangeRuntime, RunChange};
use crate::module::change::worker::WorkerResponse::Login;

const WORKER_SUBCOMMAND: &str = "worker";
const WORKER_SPAWN_TIMEOUT: u32 = 60000;

const SOCKET_PATH: &str = "/tmp/pusta/portal/";
const WORKER_TMP_PATH: &str = "/tmp/pusta/temp/";

/// This struct represents a worker portal to which multiple workers are attached and can run jobs
pub struct WorkerPortal {
    id: Uuid,
    socket: UnixListener,
    workers: HashMap<Uuid, Worker>,
    changes: HashMap<Uuid, Uuid> // Job - Worker
}

/// This represents a single worker
struct Worker {
    id: Uuid,
    root: bool,
    stream: UnixStream
}

impl WorkerPortal {

    /// Opens a portal
    pub fn open() -> anyhow::Result<Self> {
        // create id and path of socket
        let id = Uuid::new_v4();
        let mut path = PathBuf::from(SOCKET_PATH);
        fs::create_dir_all(&path).context("failed to create directory for socket")?;
        path.push(id.to_string());

        // open socket
        let mut socket = UnixListener::bind(&path)
            .with_context(|| format!("failed to bind to unix socket at '{}'", path.to_string_lossy()))?;

        socket.set_nonblocking(true)
            .context("failed to set socket to nonblocking")?;

        Ok(Self { id, socket, workers: HashMap::new(), changes: HashMap::new() })
    }

    /// Closes the portal and with it the socket and all connected clients
    pub fn close(self: Self) {
        // close socket by dropping
        drop(self);
    }

    /// Spawns a worker that will connect to the portal
    pub fn summon(&mut self, root: bool, elevator: &str, clean: bool) -> anyhow::Result<()> {
        let exe = env::current_exe()
            .context("failed to retrieve executable of myself")?;

        let mut command = if root {
            let mut command = Command::new(elevator);
            command.arg(exe);
            command
        } else {
            Command::new(exe)
        };

        command.arg(WORKER_SUBCOMMAND).arg(self.id.to_string());

        let worker_id = Uuid::new_v4();
        debug!("spawning worker with id {worker_id}");
        let mut child = command.arg(worker_id.to_string()).spawn()
            .map_err(|e| anyhow!("failed to spawn worker, {e}"))?;

        debug!("waiting for connection of worker {worker_id}");
        let now = Instant::now();
        let mut stream = loop {

            // check if child is alive
            if let Ok(Some(exit)) = child.try_wait() {
                return Err(anyhow!("worker exited too early with status code {}", exit.code().unwrap_or(-999)))
            }

            // get stream
            if let Ok((stream, addr)) = self.socket.accept() {
                break stream;
            }

            if now.elapsed().as_millis() >= WORKER_SPAWN_TIMEOUT as u128 {
                child.kill().context("failed to kill worker because of time limit")?;
                return Err(anyhow!("worker failed to spawn within time limit"));
            }

            sleep(Duration::from_millis(100));
        };

        debug!("reading login from worker");
        if let Login(id) = read_event(&mut stream)? {
            if id != worker_id { return Err(anyhow!("worker logged in with unexpected id")); }
        } else {
            return Err(anyhow!("worker did not log in properly"));
        }

        if root && clean {
            debug!("resetting terminal settings");

            if let Err(e) = Command::new("stty").arg("sane").spawn().and_then(|mut c| c.wait()) {
                warn!("failed to clean terminal: {e:#}");
            } else {
                debug!("successfully cleaned terminal settings");
            }
        }

        debug!("worker logged in successfully");
        self.workers.insert(worker_id.clone(), Worker {
            id: worker_id,
            root,
            stream
        });

        Ok(())
    }

    /// Runs a change on the loaded worker
    pub fn dispatch(&mut self, change: &Box<dyn AtomicChange>, root: bool, cache: &Path, apply: bool) -> anyhow::Result<ChangeResult> {
        if let Some(worker) = self.workers.values_mut().find(|w| w.root == root) {

            write_event(&mut worker.stream, WorkerRequest::Request(change.clone(), apply, cache.to_owned()))?;

            if let WorkerResponse::Response(result) = read_event(&mut worker.stream)? {
                Ok(result)
            } else {
                Err(anyhow!("worker did not respond as expected"))
            }
        } else { Err(anyhow!("no worker present with required privileges"))}
    }
}

#[derive(Serialize,Deserialize)]
enum WorkerResponse {
    Login(Uuid),
    Response(ChangeResult)
}

#[derive(Serialize, Deserialize)]
enum WorkerRequest {
    Request(Box<dyn AtomicChange>, bool, PathBuf)
}

/// Writes an event from the view of the portal
fn write_event(stream: &mut UnixStream, request: WorkerRequest) -> anyhow::Result<()> {
    let mut string = serde_json::to_string(&request)
        .context("failed to serialize worker request")?;
    string.push('\n');

    stream.write_all(string.as_bytes())
        .context("failed to write string to socket stream")?;
    stream.flush()
        .context("failed to flush socket stream")
}

/// Reads an event from the view of the portal
fn read_event(stream: &mut UnixStream) -> anyhow::Result<WorkerResponse> {
    let mut stream = BufReader::new(stream);

    let mut buffer = String::new();
    stream.read_line(&mut buffer)
        .context("failed to read string from socket stream")?;

    serde_json::from_str(&buffer)
        .context("failed to deserialize response from worker")
}

