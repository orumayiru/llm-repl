// src/shell.rs
use std::process::Command;
use crate::error::{ReplError, ReplResult};

/// Executes a command line string using the default system shell.
///
/// Captures and returns the standard output (stdout) of the command.
/// If the command fails to run or exits with a non-zero status,
/// it returns an error containing stderr or status information.
pub fn execute_shell_command(command_line: &str) -> ReplResult<String> {
    if command_line.trim().is_empty() {
        return Ok("".to_string()); // Nothing to execute
    }

    let command_output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C") // Tells cmd to execute the following string and then exit
            .arg(command_line)
            .output() // Executes the command and waits for completion
    } else {
        // Assume Unix-like shell (sh) for Linux, macOS, etc.
        Command::new("sh")
            .arg("-c") // Tells sh to execute the following string
            .arg(command_line)
            .output() // Executes the command and waits for completion
    };

    match command_output {
        Ok(output) => {
            if output.status.success() {
                // Command succeeded, try to convert stdout to String
                String::from_utf8(output.stdout)
                    .map_err(|e| ReplError::Command(format!("Shell command output was not valid UTF-8: {}", e)))
            } else {
                // Command executed but failed (non-zero exit code)
                let stderr_string = String::from_utf8_lossy(&output.stderr); // Show stderr if possible
                let status_code = output.status.code().map_or_else(|| "Signal".to_string(), |c| c.to_string());
                Err(ReplError::Command(format!(
                    "Shell command failed (Exit Code: {}):\n{}",
                    status_code,
                    stderr_string.trim()
                )))
            }
        }
        Err(e) => {
            // Failed to even start the command (e.g., shell not found)
            Err(ReplError::Command(format!("Failed to execute shell command: {}", e)))
        }
    }
}