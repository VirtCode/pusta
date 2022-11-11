use std::fs::File;
use log::{Level, Log, Metadata, Record};
use log::LevelFilter::Debug;
use crate::output::{print_error, print_info, print_verbose, print_warn};

pub const LOG_DIR: &str = "~/.config/pusta/log/";

struct Logger {
    file: Option<File>,
    verbose: bool
}

impl Logger {
    fn new(file_logging: bool, verbose: bool) -> Self {
        Logger {
            file: None, // TODO: Implement file logging
            verbose
        }
    }
}

impl Log for Logger {

    // Enable logging for every level, since log files are used
    fn enabled(&self, metadata: &Metadata) -> bool { true }

    fn log(&self, record: &Record) {
        match record.level() {
            Level::Error => { print_error(&record.args().to_string()) }
            Level::Warn => { print_warn(&record.args().to_string()) }
            Level::Info => { print_info(&record.args().to_string()) }
            Level::Debug => { if self.verbose { print_verbose(&record.args().to_string()) } }
            Level::Trace => {}
        }
    }

    fn flush(&self) {

    }
}

pub fn enable_logging(file_logging: bool, verbose: bool) {
    log::set_boxed_logger(Box::new(Logger::new(file_logging, verbose))).unwrap_or_else(|e| eprintln!("Failed to instantiate Logger: {}", e));
    log::set_max_level(Debug);
}