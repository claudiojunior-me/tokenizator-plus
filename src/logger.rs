use colored::*;
use std::env;

#[derive(PartialEq, PartialOrd)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

fn current_level() -> LogLevel {
    match env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase()
        .as_str()
    {
        "error" => LogLevel::Error,
        "warn" => LogLevel::Warn,
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        _ => LogLevel::Info,
    }
}

pub fn log(level: LogLevel, msg: &str) {
    let env_level = current_level();
    if level <= env_level {
        let level_str = match level {
            LogLevel::Error => "ERROR".red().bold(),
            LogLevel::Warn => "WARN".yellow().bold(),
            LogLevel::Info => "INFO".green().bold(),
            LogLevel::Debug => "DEBUG".blue().bold(),
        };
        println!("[{}] {}", level_str, msg);
    }
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogLevel::Error, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogLevel::Warn, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogLevel::Info, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogLevel::Debug, &format!($($arg)*));
    };
}

