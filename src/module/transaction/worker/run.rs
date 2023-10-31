use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::exit;
use anyhow::Context;
use log::error;
use uuid::Uuid;
use crate::module::transaction::change::{ChangeError, ChangeResult, ChangeRuntime};
use crate::module::transaction::worker::{SOCKET_PATH, WORKER_TMP_PATH, WorkerRequest, WorkerResponse};

/// Handles the worker and prints errors
pub fn handle_worker(socket_id: Uuid, id: Uuid) {
    if let Err(e) = worker(socket_id, id) {
        error!("worker failed fatally: {}", e.to_string());
        exit(32);
    }
}

/// Main body of a worker, connects to socket and runs jobs
fn worker(socket_id: Uuid, id: Uuid) -> anyhow::Result<()> {
    let mut path = PathBuf::from(SOCKET_PATH);
    path.push(socket_id.to_string());

    // Establish connection
    let mut socket = UnixStream::connect(path)
        .context("socket should be online to connect to")?;

    // Create runtime
    let runtime = ChangeRuntime { dir: {
        let mut path = PathBuf::from(WORKER_TMP_PATH);
        path.push(id.to_string());
        path
    }};
    let mut changes = HashMap::new();

    // Ready
    write_event(&mut socket, WorkerResponse::Login(id))?;

    while let Ok(request) = read_event(&mut socket) {
        match request {
            WorkerRequest::Load(c) => {
                for (k,v) in c { changes.insert(k,v); }
            }
            WorkerRequest::Request(id, apply) => {
                let response = if let Some(change) = changes.get_mut(&id) {
                    if apply { change.apply(&runtime) }
                    else { change.revert(&runtime) }
                } else {
                    Err(ChangeError::fatal("change was not loaded onto worker".to_string()))
                };

                write_event(&mut socket, WorkerResponse::Response(response))?;
            }
        }
    }

    Ok(())
}

/// Writes an event to the unix socket
fn write_event(stream: &mut UnixStream, request: WorkerResponse) -> anyhow::Result<()> {
    let mut string = serde_json::to_string(&request)
        .context("failed to serialize worker request")?;
    string.push('\n');

    stream.write_all(string.as_bytes())
        .context("failed to write string to socket stream")?;
    stream.flush()
        .context("failed to flush socket stream")
}

/// Reads an event from the unix socket
fn read_event(stream: &mut UnixStream) -> anyhow::Result<WorkerRequest> {
    let mut stream = BufReader::new(stream);

    let mut buffer = String::new();
    stream.read_line(&mut buffer)
        .context("failed to read string from socket stream")?;

    serde_json::from_str(&buffer)
        .context("failed to deserialize response from worker")
}