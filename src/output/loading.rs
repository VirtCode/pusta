use std::io::{stdout, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use colored::Colorize;
use crate::output::loading::LoadingAction::{Failure, Success, Update};

enum LoadingAction {
    Update, Success, Failure
}

pub struct Loading {
    tx: Sender<(LoadingAction, String)>
}

impl Loading {
    pub fn start(string: &str) -> Self {

        let string = string.to_string();
        let (mut tx, mut rx) : (Sender<(LoadingAction, String)>, Receiver<(LoadingAction, String)>) = channel();

        thread::spawn(move || {
            let animation = vec!["⡏ ", "⠏⠁", "⠋⠉", "⠉⠙", "⠈⠹", " ⢹", " ⣸", "⢀⣰", "⣀⣠", "⣄⣀", "⣆⡀", "⣇ "];

            let mut frame = 0;
            let mut message = string;
            println!();

            loop {
                while let Ok((action, msg)) = rx.try_recv() {
                    match action {
                        Update => { message = msg }
                        Success => { println!("\x1B[2K{} {}", "::".green(), msg); break } // Alternative ⣏⣹
                        Failure => { println!("\x1B[2K{} {}", "::".bright_red(), msg); break }
                    }
                }

                print!("\r\x1B[2K{} {}\r", animation.get(frame).unwrap().bright_blue(), &message);
                stdout().flush().unwrap();

                sleep(Duration::from_millis(50));

                frame += 1;
                if frame == animation.len() { frame = 0}
            }
        });

        Loading {
            tx
        }
    }

    pub fn stop(&mut self, success: bool, message: &str) {
        self.tx.send((if success { Success } else { Failure }, message.to_string())).unwrap_or(());
    }

    pub fn change(&mut self, message: &str) {
        self.tx.send((Update, message.to_string())).unwrap_or(());
    }
}