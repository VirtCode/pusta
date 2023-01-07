pub mod logger;
pub mod loading;

use std::fmt::format;
use std::fs::{File, metadata};
use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::{JoinHandle, sleep};
use std::time::Duration;
use colored::Colorize;
use log::{info, Level, Log, Metadata, Record};
use log::LevelFilter::Debug;

pub static CURRENT_INDENT: AtomicUsize = AtomicUsize::new(0);

pub fn print_error(message: &str) {
    println!("\x1B[2K{: >width$} {}", "error:".bright_red().bold(), message, width = CURRENT_INDENT.load(Ordering::Relaxed) + 6);
}

pub fn print_warn(message: &str) {
    println!("\x1B[2K{: >width$} {}", "warning:".bright_yellow().bold(), message, width = CURRENT_INDENT.load(Ordering::Relaxed) + 8);
}

pub fn print_info(message: &str) {
    println!("\x1B[2K{: >width$}{}", "", message, width = CURRENT_INDENT.load(Relaxed));
}

pub fn print_verbose(message: &str) {
    println!("\x1B[2K{: >width$}{}", "", message.dimmed().italic(), width = CURRENT_INDENT.load(Relaxed))
}

pub fn print_shell(message: &str) {
    println!("\x1B[2K{: >width$}{}", "", message.dimmed(), width = CURRENT_INDENT.load(Relaxed))
}

pub fn prompt_yn(question: &str, default: bool) -> bool {
    print!("{} {} {} ", "??".bright_blue().bold(), question, (if default { "[Y/n]" } else { "[y/N]" }).bold());
    stdout().flush().unwrap_or(());

    let mut line = String::new();
    if stdin().read_line(&mut line).is_err() {
        print_error("Failed to read from stdin, assuming no");
        return false;
    }
    line = line.trim().to_lowercase();

    if line.is_empty() { default }
    else { line.starts_with('y') } // Assume no for garbage input
}

pub fn prompt_choice(question: &str, choices: Vec<&String>, default: Option<usize>) -> String {
    println!("{} {}", "??".bright_blue().bold(), question);

    for (i, choice) in choices.iter().enumerate() {
        println!("   {}: {choice}", (i + 1).to_string().bold());
    }

    let def = default.map(|i| format!(" (default: {i})")).unwrap_or_else(|| "".to_owned());

    loop {
        print!("{} Enter the number of your choice{def}: ", "??".bright_blue().bold());
        stdout().flush().unwrap_or(());

        let mut line = String::new();
        if stdin().read_line(&mut line).is_err() {
            print_error("Failed to read from stdin");
            return "".to_owned();
        }
        
        // If default, use default
        if default.is_some() && line.is_empty() { return (**choices.get(default.unwrap()).unwrap()).clone() }
        
        if let Ok(i) = usize::from_str(line.trim()) {
                      
            if i < 1 || i > choices.len() { print_error("Please enter a number within range") }
            else {
                return (**choices.get(i - 1).unwrap()).clone();
            }
        } else {
            print_error("Please enter a valid number");
        }
    }
}

pub fn start_section(message: &str) {
    println!("{} {}", "::".bright_blue().bold(), message);
    CURRENT_INDENT.store(3, Relaxed);
}

pub fn start_shell(message: &str) {
    info!("{}", message);
    println!("{}", "╭──╯".dimmed());
}

pub fn end_shell(success: bool, message: &str) {
    println!("{}", "╰──╮".dimmed());
    info!("{}", message);
}

pub fn end_section(success: bool, message: &str) {
    CURRENT_INDENT.store(0, Relaxed);
    println!("{} {}", if success { "::".bright_green().bold() } else { "::".bright_red().bold() }, message);
}
