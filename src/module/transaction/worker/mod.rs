use std::collections::HashMap;
use std::{io, thread};
use std::io::{BufRead, BufReader, Read};
use std::os::fd::AsFd;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use lazy_regex::{Lazy, lazy_regex};
use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::module::transaction::change::{AtomicChange, ChangeResult};

mod main;

#[derive(Serialize, Deserialize)]
struct WorkerSetup {
    woke_id: Uuid,
    exit_id: Uuid,
    changes: HashMap<Uuid, Box<dyn AtomicChange>>
}

struct Worker {
    process: Child,
    pub root: bool,
    pub setup: WorkerSetup,

    alive: bool,
    stdout: ChildStdout,
    stdin: ChildStdin,
}

impl Worker {
    fn spawn(root: bool, binary: &str, elevator: &str, changes: HashMap<Uuid, Box<dyn AtomicChange>>) -> Result<Self> {
        // Create setup
        let setup = WorkerSetup {
            changes,
            exit_id: Uuid::new_v4(),
            woke_id: Uuid::new_v4()
        };
        let setup_serialized = serde_json::to_string(&setup).unwrap();

        // Prepare command
        let mut command = if root {
            let mut c = Command::new(elevator);
            c.arg(binary); c
        } else {
            Command::new(binary);
        };
        command.arg(setup_serialized);

        // Configure stdout
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());

        let mut child = command.spawn().unwrap();

        // Take stdout
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Wait to receive woke
        let (rx, tx) = mpsc::channel();





        ;
    }


}

const WORKER_REGEX: Lazy<Regex> = lazy_regex!(r#"^([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}) \(pusta\) (.+)"#);

fn read_woke(stdout: &mut ChildStdout, stdin: &mut ChildStdin, woke: Uuid) {
    let buffer = [0u8; 100];

    stdout.read()


}

fn read_worker(stdout: ChildStdout, events: Sender<WorkerResponse>, woke: Uuid) {
    thread::spawn(|| {
        BufReader::new(stdout)
            .lines()
            .filter_map(|r| r.ok())
            .for_each(|s| {
                if let Some(capture) = WORKER_REGEX.captures(&s) {
                    let id = Uuid::from_str(capture.get(0).expect("capture must exist").as_str()).expect("regex should've checked uuid");
                    let message = capture.get(1).expect("capture must exist").as_str();

                    if id == woke {
                        events.send(WorkerResponse::Woke).expect("channel should only close when worker closes");
                    } else {
                        if let Ok(message) = serde_json::from_str(message) {
                            events.send(WorkerResponse::Result(id, message)).expect("channel should only close when worker closes")
                        } else {
                            error!("received invalid message from worker")
                        }
                    }
                } else {
                    // direct to stdout
                    println!("{s}");
                }
            });

        events.send(WorkerResponse::Dead)
    });
}

fn write_worker(stdin: ChildStdin, events: Receiver<Uuid>, )

enum WorkerResponse {
    Woke,
    Dead,
    Result(Uuid, ChangeResult),
}