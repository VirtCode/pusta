use colored::Colorize;
use log::{Level, Log, Metadata, Record};
use log::LevelFilter::Debug;

static mut OUT: Output = Output {
    verbose: false,
    indent: false
};

struct Output {
    verbose: bool,
    indent: bool
}

impl Output {

    fn get_indent(&self) -> &str {
        if self.indent { "   " }
        else { "" }
    }

    fn print_error(&self, message: &str) {
        println!("{}{} {message}", self.get_indent(), "err:".bright_red().bold());
    }

    fn print_warn(&self, message: &str) {
        println!("{}{} {message}", self.get_indent(), "wrn:".bright_yellow().bold());
    }

    fn print_info(&self, message: &str) {
        println!("{}{message}", self.get_indent());
    }

    fn print_verbose(&self, message: &str) {
        if self.verbose {
            println!("{}{}", self.get_indent(), message.dimmed().italic());
        }
    }

    pub fn set_indent(&mut self, indent: bool) {
        self.indent = indent;
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }
}

impl Log for Output {

    fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() >= Level::Debug }

    fn log(&self, record: &Record) {
        match record.level() {
            Level::Error => { self.print_error(&record.args().to_string()) }
            Level::Warn => { self.print_warn(&record.args().to_string()) }
            Level::Info => { self.print_info(&record.args().to_string()) }
            Level::Debug => { self.print_verbose(&record.args().to_string()) }
            _ => {}
        }
    }

    fn flush(&self) { }
}

pub fn enable_logging(verbose: bool) {
    unsafe {
        // Set logger
        log::set_logger(&OUT).unwrap_or_else(|e| {
            eprintln!("fatal error: Failed to set logger ({})", e.to_string())
        });

        OUT.set_verbose(verbose);
    }

    // Optimize using max level
    log::set_max_level(Debug);
}

pub fn enable_indent() {
    unsafe {
        OUT.set_indent(true);
    }
}

pub fn disable_indent() {
    unsafe {
        OUT.set_indent(false);
    }
}

pub fn is_verbose() -> bool {
    unsafe { OUT.verbose }
}