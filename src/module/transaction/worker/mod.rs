pub mod run;

use std::collections::HashMap;
use std::{env, fs, io, thread};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use anyhow::{anyhow, Context};
use lazy_regex::{Lazy, lazy_regex};
use log::{debug, error};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::module::transaction::change::{AtomicChange, ChangeResult, ChangeRuntime};
use crate::module::transaction::worker::WorkerResponse::Login;

const WORKER_SUBCOMMAND: &str = "worker";
const WORKER_SPAWN_TIMEOUT: u32 = 60000;

const SOCKET_PATH: &str = "/tmp/pusta/portal/";
const WORKER_TMP_PATH: &str = "/tmp/pusta/";

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
    pub fn summon(&mut self, root: bool, elevator: &str) -> anyhow::Result<()> {
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
            .context("failed to run command to spawn worker")?;

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

        debug!("worker logged in successfully");
        self.workers.insert(worker_id.clone(), Worker {
            id: worker_id,
            root,
            stream
        });

        Ok(())
    }

    /// Loads a map of changes onto the workers for later execution
    pub fn load(&mut self, changes: HashMap<Uuid, (bool, Box<dyn AtomicChange>)>) -> anyhow::Result<()>{
        let mut non = HashMap::new();
        let mut root = HashMap::new();

        for (id, (as_root, change)) in changes {
            if as_root { &mut root } else { &mut non }
                .insert(id, change);
        }

        // load jobs
        if !non.is_empty() {
            self.load_to_worker(non, false)?;
        }

        if !root.is_empty() {
            self.load_to_worker(root, true)?;
        }

        Ok(())
    }

    /// Loads a specific set of changes to a worker with the given privilege level
    fn load_to_worker(&mut self, changes: HashMap<Uuid, Box<dyn AtomicChange>>, root: bool) -> anyhow::Result<()> {
        if let Some(worker) = self.workers.values_mut().find(|w| w.root == root) {

            // Update job lexicon
            for (id, _) in &changes{
                self.changes.insert(id.clone(), worker.id.clone());
            }

            write_event(&mut worker.stream, WorkerRequest::Load(changes))?;

            Ok(())
        } else {
            return Err(anyhow!("could not find worker with sufficient (root: {root}) privileges to load change"))
        }
    }

    /// Runs a change on the loaded worker
    pub fn dispatch(&mut self, id: &Uuid, apply: bool) -> anyhow::Result<ChangeResult> {

        let worker_id = self.changes.get(id)
            .context("job was not loaded for any worker")?;

        let worker = self.workers.get_mut(worker_id)
            .context("worker where job was registered could not be found")?;

        write_event(&mut worker.stream, WorkerRequest::Request(id.clone(), apply))?;

        if let WorkerResponse::Response(result) = read_event(&mut worker.stream)? {
            Ok(result)
        } else {
            Err(anyhow!("worker did not respond as expected"))
        }
    }
}

#[derive(Serialize,Deserialize)]
enum WorkerResponse {
    Login(Uuid),
    Response(ChangeResult)
}

#[derive(Serialize, Deserialize)]
enum WorkerRequest {
    Load(HashMap<Uuid, Box<dyn AtomicChange>>),
    Request(Uuid, bool)
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

