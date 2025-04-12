// src/commands/markdown.rs
use async_trait::async_trait;

use crate::{
    commands::Command,
    error::ReplResult,
    state::{AppState, MarkdownMode}, // Import MarkdownMode
};

// --- Command for /md (Append Formatted) ---
#[derive(Clone)]
pub struct MdCommand {
    state: AppState,
}

impl MdCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Command for MdCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        self.state.set_markdown_mode(MarkdownMode::AppendFormatted).await;
        Ok("Markdown rendering mode set to: Append Formatted (Stream raw, append formatted below)".to_string())
    }

    fn name(&self) -> &str { "md" }
    fn help(&self) -> &str { "Set Markdown rendering to stream raw text, then append formatted." }
}


// --- Command for /md_streaming (Live Rendering) ---
#[derive(Clone)]
pub struct MdStreamingCommand {
    state: AppState,
}

impl MdStreamingCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Command for MdStreamingCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        self.state.set_markdown_mode(MarkdownMode::LiveStreaming).await;
        Ok("Markdown rendering mode set to: Live Streaming (Experimental, may flicker)".to_string())
    }

    fn name(&self) -> &str { "md_streaming" }
    fn help(&self) -> &str { "Set Markdown rendering to attempt live formatting (Experimental)." }
}


// --- Command for /md_off (Disable Rendering) ---
#[derive(Clone)]
pub struct MdOffCommand {
    state: AppState,
}

impl MdOffCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Command for MdOffCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        self.state.set_markdown_mode(MarkdownMode::Off).await;
        Ok("Markdown rendering disabled (Raw text output only)".to_string())
    }

    fn name(&self) -> &str { "md_off" }
    fn help(&self) -> &str { "Disable all Markdown rendering." }
}


// --- (Optional) Command for /md_status ---
#[derive(Clone)]
pub struct MdStatusCommand {
    state: AppState,
}

impl MdStatusCommand {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Command for MdStatusCommand {
    async fn execute(&self, _args: &str) -> ReplResult<String> {
        let mode = self.state.get_markdown_mode().await;
        let mode_str = match mode {
            MarkdownMode::AppendFormatted => "Append Formatted (Stream raw, append formatted below)",
            MarkdownMode::LiveStreaming => "Live Streaming (Experimental, may flicker)",
            MarkdownMode::Off => "Off (Raw text output only)",
        };
        Ok(format!("Current Markdown rendering mode: {}", mode_str))
    }

    fn name(&self) -> &str { "md_status" }
    fn help(&self) -> &str { "Show the current Markdown rendering mode." }
}