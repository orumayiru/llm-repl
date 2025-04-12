// src/error.rs
use thiserror::Error;
// use clap; // Keep if clap errors are used (currently not)
use dialoguer;
use rustyline::error::ReadlineError; // Be specific

#[derive(Debug, Error)]
pub enum ReplError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Command error: {0}")]
    Command(String),

    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    // #[error("Argument parsing error: {0}")] // Keep if clap is used
    // ArgumentParsing(#[from] clap::error::Error),

    #[error("Readline error: {0}")]
    Readline(String), // Store as String to avoid lifetime issues with ReadlineError directly
}

// --- From Implementations ---

impl From<ReadlineError> for ReplError {
    fn from(err: ReadlineError) -> Self {
        ReplError::Readline(err.to_string())
    }
}

impl From<dialoguer::Error> for ReplError {
    // Correctly handle dialoguer Error variants
    fn from(err: dialoguer::Error) -> Self {
        match err {
            dialoguer::Error::IO(io_err) => ReplError::Io(io_err), // <-- Use IO (all caps)
            // Map other dialoguer errors (like user aborting) to a command error
            _ => ReplError::Command(format!("Input error: {}", err)),
        }
    }
}

// Add From<clap::error::Error> if/when clap is used for args
// impl From<clap::error::Error> for ReplError {
//     fn from(err: clap::error::Error) -> Self {
//         ReplError::ArgumentParsing(err)
//     }
// }

// --- End From Implementations ---

pub type ReplResult<T> = Result<T, ReplError>;