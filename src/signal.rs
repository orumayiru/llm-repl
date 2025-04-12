// src/signal.rs
use lazy_static::lazy_static;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::flag as signal_flag;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc; // Ensure Arc is imported
use std::io;

lazy_static! {
    // Global flag to indicate if an interrupt signal (like Ctrl+C) has been received.
    // Define it directly as an Arc<AtomicBool> as required by signal_flag::register.
    pub static ref STOP_CONVERSATION_FLAG: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

/// Registers signal handlers to set the STOP_CONVERSATION_FLAG.
/// This should be called once at the start of the application.
/// It clones the Arc for each signal handler registration.
pub fn register_signal_handlers() -> io::Result<()> {
    // signal_flag::register needs an Arc<AtomicBool> and clones it internally.
    signal_flag::register(SIGINT, STOP_CONVERSATION_FLAG.clone())?; // Ctrl+C
    signal_flag::register(SIGTERM, STOP_CONVERSATION_FLAG.clone())?; // Termination signal
    signal_flag::register(SIGHUP, STOP_CONVERSATION_FLAG.clone())?; // Hangup signal
    signal_flag::register(SIGQUIT, STOP_CONVERSATION_FLAG.clone())?; // Quit signal
    Ok(())
}

/// Resets the stop flag to false.
/// Should be called after a signal has been handled or a stoppable operation completes.
pub fn reset_stop_flag() {
    STOP_CONVERSATION_FLAG.store(false, Ordering::SeqCst);
}

/// Checks if the stop flag has been set (i.e., if an interrupt signal was received).
pub fn is_stop_requested() -> bool {
    STOP_CONVERSATION_FLAG.load(Ordering::SeqCst)
}