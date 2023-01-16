pub mod logger;

use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use colored::Colorize;
use log::{error, info};
use crate::output::logger::{disable_indent, enable_indent};

pub fn prompt_yn(question: &str, default: bool) -> bool {
    print!("{} {} {} ", "??".bright_blue().bold(), question, (if default { "[Y/n]" } else { "[y/N]" }).bold());
    stdout().flush().unwrap_or(());

    let mut line = String::new();
    if stdin().read_line(&mut line).is_err() {
        error!("Failed to read from stdin, assuming no");
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
            error!("Failed to read from stdin");
            return "".to_owned();
        }
        
        // If default, use default
        if default.is_some() && line.is_empty() { return (**choices.get(default.unwrap()).unwrap()).clone() }
        
        if let Ok(i) = usize::from_str(line.trim()) {
                      
            if i < 1 || i > choices.len() { error!("Please enter a number within range") }
            else {
                return (**choices.get(i - 1).unwrap()).clone();
            }
        } else {
            error!("Please enter a valid number");
        }
    }
}

pub fn start_section(message: &str) {
    println!("{} {}", "::".bright_blue().bold(), message);
    enable_indent()
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
    disable_indent();
    println!("{} {}", if success { "::".bright_green().bold() } else { "::".bright_red().bold() }, message);
}
