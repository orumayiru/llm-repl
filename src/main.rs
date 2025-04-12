// src/main.rs
mod commands;
mod error;
mod providers;
mod repl;
mod server; // <-- Add server module
mod state;
mod shell;
mod render;
mod signal;

use crate::{
    error::ReplResult, // Use our result type
    repl::Repl,
    state::AppState, // Ensure AppState is imported
};
use clap::Parser;
use std::{net::SocketAddr, str::FromStr}; // For parsing SocketAddr

/// An extensible REPL for interacting with LLMs. Includes an optional REST API server.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Run the REST API server instead of the interactive REPL.
    #[arg(long)]
    server: bool,

    /// Host and port for the REST API server.
    #[arg(long, value_name = "HOST:PORT", default_value = "127.0.0.1:3000", env = "LLM_REPL_SERVER_ADDR")]
    addr: String,
}

// Use tokio main for async startup if running server
#[tokio::main]
async fn main() -> ReplResult<()> { // Return our result type
    let args = CliArgs::parse();

    // Register signal handlers (useful for both REPL and server)
    if let Err(e) = signal::register_signal_handlers() {
        eprintln!("WARN: Failed to register signal handlers: {}", e);
        // Decide if this is fatal? Probably not for now.
    }

    // Initialize shared state
    // AppState::new is sync, so we can call it here.
    // If it becomes async later, adjust accordingly.
    let app_state = AppState::new();

    if args.server {
        // --- Run Server ---
        println!("Starting in server mode...");
        let socket_addr = SocketAddr::from_str(&args.addr).map_err(|e| {
            error::ReplError::Command(format!("Invalid server address '{}': {}", args.addr, e))
        })?;

        // Run the server - handle potential errors
        if let Err(e) = server::run_server(app_state, socket_addr).await {
            eprintln!("Server error: {}", e);
            // Convert Box<dyn Error> to ReplError if needed, or just exit
             return Err(error::ReplError::Command(format!("Server failed: {}", e))); // Example conversion
        }
         Ok(()) // Server finished (likely due to signal)
    } else {
        // --- Run REPL ---
        println!("Starting in REPL mode...");
        // Repl::new() is sync
        match Repl::new() {
            Ok(mut repl) => {
                // Repl::run is blocking in its current form (uses block_on internally)
                // If run needs to be async later, adjust how it's called.
                 // For now, we wrap the potentially blocking call.
                 // If Repl::run becomes async, we can just `.await` it here.
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = repl.run() {
                       eprintln!("REPL error: {}", e);
                       // Return error state if needed, depends on desired exit code
                    }
                }).await.map_err(|e| error::ReplError::Command(format!("REPL task failed: {}", e)))?;
                 Ok(())
            }
            Err(e) => {
                eprintln!("Failed to initialize REPL: {}", e);
                 Err(e) // Propagate initialization error
            }
        }
    }
}