      
# llm-repl

[![Language: Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md) <!-- Add a LICENSE.md file! -->

An extensible Read-Eval-Print Loop (REPL) for interacting with various Large Language Models (LLMs) via different providers. Supports shell command execution, configurable Markdown rendering, themeable interface elements, LLM conversations, session history tracking, and an optional REST API server.

Built with Rust, leveraging async operations via Tokio for efficient handling of API requests.

## Key Features

*   **Interactive REPL:** Familiar command-line interface with input history and a dynamic, themed prompt showing the current provider and model.
*   **Extensible Commands:** Execute built-in commands (prefixed with `/`) or add your own easily.
*   **Extensible LLM Providers:** Interact with different LLM backends.
    *   Currently supports:
        *   **Ollama:** Connects to a running Ollama instance (expects Ollama running is default port).
           _(if you do not have ollama  go to https://ollama.com/download and follow the installation procedure.)_
        *   **Groq:** High-speed inference via GroqCloud API (requires `GROQ_API_KEY`).
          _(Needs a Groq account)_
        *   **Gemini:** Connects to Google's Gemini API (requires `GOOGLE_API_KEY`).
          _(Needs a Google account.)_
    *   Add support for new providers (e.g., OpenAI, Anthropic etc.,) by modifying the source code. Currently there is no simple way of doing this. you have to write rust code for different provider in a format expected by the REPL core structure. 
*   **Shell Integration:** Execute arbitrary shell commands directly from the REPL (prefixed with `!`).
*   **Markdown Rendering:** Renders LLM responses as formatted Markdown in the terminal. Selectable modes:
    *   `AppendFormatted` (Default): Shows raw stream, appends formatted output.
    *   `LiveStreaming`: Attempts experimental live rendering during streaming.
    *   `Off`: Disables Markdown rendering for raw text output.
*   **Theming:** Customize the look and feel with selectable themes (e.g., `Default`, `Nord`) affecting the prompt, messages, and Markdown output.
*   **LLM vs LLM Conversations:** Simulate conversations between two configured LLMs using the `/llmconvo` command with interactive setup.(_The command is still in dvelopment it uses default editor to provide text input. may work well in linux environment. I have not checked in windows environment._)
*   **Session History Reader:** View the history of the current REPL session (queries, responses, commands, errors) in a formatted, read-only view using the `/reader` command.
*   **Optional REST API Server:** Run `llm-repl` as a backend server (`--server` flag) exposing REST endpoints to query LLMs, execute commands, run shell commands, and retrieve status/history remotely. Includes graceful shutdown.
*   **Asynchronous:** Built on the `tokio` runtime for efficient handling of network requests and other operations.
*   **Unified Error Handling:** Uses `thiserror` for clear and consistent error reporting.

## Installation

1.  **Prerequisites:**
    *   Rust toolchain (stable recommended): [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
    *   (Optional) Ollama installed and running if you want to use the Ollama provider: [https://ollama.com/](https://ollama.com/)
    *   Git
2.  **Clone the Repository:**
    ```bash
    git clone https://github.com/orumayiru/llm-repl
    cd llm-repl
    ```
3.  **Build:**
    ```bash
    # For development
    cargo build
    # For release (optimized)
    cargo build --release
    ```
    The executable will be located at `target/debug/llm-repl` or `target/release/llm-repl`.

## Configuration

API keys for providers are configured via environment variables:

*   **Groq:** Set the `GROQ_API_KEY` environment variable to your GroqCloud API key.
    ```bash
    export GROQ_API_KEY="gsk_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    ```
*   **Gemini:** Set the `GOOGLE_API_KEY` environment variable to your Google AI Studio API key.
    ```bash
    export GOOGLE_API_KEY="AIzaSyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    ```
*   **Server Address (Optional):** The REST API server address can be configured via the `LLM_REPL_SERVER_ADDR` environment variable (or the `--addr` flag). Defaults to `127.0.0.1:3000`.
    ```bash
    export LLM_REPL_SERVER_ADDR="0.0.0.0:8080"
    ```

The application needs these variables set in its environment *before* starting if you intend to use the corresponding providers.

## Usage

### 1. REPL Mode (Default)

Start the interactive REPL:

```bash
cargo run
# or directly after building release
./target/release/llm-repl  

You'll see a prompt like [ollama:llama3:latest]>> .

Key Commands:

    /help: Shows the list of available commands and current settings.

    /provider [name]: Select the LLM provider.

        Run without [name] for an interactive fuzzy selection.

        Example: /provider groq (requires GROQ_API_KEY env var).

    /model [name]: Select the model for the current provider.

        Run without [name] for an interactive fuzzy selection.

        Example: /model llama3:latest (if using ollama).

    /theme [name]: Select the display theme.

        Run without [name] for interactive selection.

        Available: default, nord, gruvbox, grayscale.

        Example: /theme nord

    /theme_status: Show the currently active theme.

    /md: Set Markdown rendering to AppendFormatted (Default).

    /md_streaming: Set Markdown rendering to LiveStreaming (Experimental).

    /md_off: Disable Markdown rendering (show raw text).

    /md_status: Show the current Markdown rendering mode.

    /llmconvo: Starts an interactive setup to simulate a conversation between two LLMs.

    /reader: Displays the history of the current session (LLM responses, commands, errors) in a read-only, formatted view within the terminal.

    ! <command> [args]: Execute a shell command.

        Example: !ls -lha

        Example: !git status

    /exit or /quit: Exits the REPL. (Ctrl+C or Ctrl+D also work).

    (Default) Query: Any text entered that doesn't start with / or ! is sent as a query to the currently selected provider and model.

        Example: Explain the concept of Rust ownership.

2. Server Mode

Start the REST API server:

      
# Default address: 127.0.0.1:3000
cargo run -- --server

# Specify address and port
cargo run -- --server --addr 0.0.0.0:8080

# Use environment variable for address
export LLM_REPL_SERVER_ADDR="0.0.0.0:9000"
cargo run -- --server

The server provides the following endpoints:

    GET /status: Get current provider, model, theme, markdown mode.

    GET /providers: List available provider names (e.g., ["ollama", "groq", "gemini"]).

    GET /providers/{provider_name}/models: List models available for a specific provider (e.g., /providers/ollama/models).

    POST /query: Send a query to the current LLM.

        Body (JSON): { "prompt": "Your query text", "model": "optional_model_override:tag" }

        Example: curl -X POST -H "Content-Type: application/json" -d '{"prompt": "Hello"}' http://localhost:3000/query

    POST /command: Execute a REPL command (without the leading /).

        Body (JSON): { "command": "command_name args" }

        Example: curl -X POST -H "Content-Type: application/json" -d '{"command": "theme nord"}' http://localhost:3000/command

    POST /shell: Execute a shell command (without the leading !).

        Body (JSON): { "command": "ls -l /tmp" }

        Example: curl -X POST -H "Content-Type: application/json" -d '{"command": "pwd"}' http://localhost:3000/shell

    GET /history: Retrieve the stored session history.

Press Ctrl+C in the terminal where the server is running to shut it down gracefully.
Architecture Overview

Extending llm-repl

The modular design makes it easy to add new functionality.
Adding a New Command

    Create File: Create a new file for your command, e.g., src/commands/my_command.rs.

    Implement Trait: Inside the new file, define a struct and implement the Command trait:

          
    // src/commands/my_command.rs
    use async_trait::async_trait;
    use crate::{
        commands::Command,
        error::ReplResult,
        state::AppState, // Import AppState if needed
    };

    pub struct MyCommand {
        state: AppState, // Store state if needed
    }

    impl MyCommand {
        pub fn new(state: AppState) -> Self { // Constructor takes state
            Self { state }
        }
    }

    #[async_trait]
    impl Command for MyCommand {
        // The actual logic for your command
        async fn execute(&self, args: &str) -> ReplResult<String> {
            // Access state via self.state if needed (use .await for async methods)
            // let current_model = self.state.get_model().await;
            Ok(format!("Executed my_command with args: {}", args))
        }

        // The name used to invoke the command (e.g., /my_command)
        fn name(&self) -> &str {
            "my_command"
        }

        // Help text shown by /help
        fn help(&self) -> &str {
            "Description of what my_command does."
        }
    }

Declare Module: Open src/commands/mod.rs and add pub mod my_command; near the top with the other module declarations.

Register Command: Open src/state.rs. Inside the AppState::new function, find the section where other commands are registered and add your new command:

      
// Inside AppState::new in src/state.rs, where registry.register calls happen:
registry.register(Box::new(crate::commands::my_command::MyCommand::new(state_clone_for_commands.clone())));

Rebuild (cargo build), and your /my_command should now be available.

Adding a New LLM Provider

    Create File: Create src/providers/my_provider.rs.

    Implement Trait: Define a struct and implement the LlmProvider trait. You'll need to handle API requests (likely using reqwest), parse responses (serde), manage API keys (often via std::env::var), and implement the core methods:

          
    // src/providers/my_provider.rs
    use async_trait::async_trait;
    use std::pin::Pin;
    use futures::Stream; // If supporting streaming
    use crate::{
        error::{ReplError, ReplResult},
        providers::LlmProvider,
    };
    // Add other imports like reqwest::Client, serde::{Deserialize, Serialize}, std::env

    #[derive(Clone, Debug)] // Add Clone if needed
    pub struct MyProvider {
        client: reqwest::Client,
        api_key: Option<String>,
        // other config...
    }

    impl MyProvider {
        pub fn new() -> Self {
            let api_key = std::env::var("MY_PROVIDER_API_KEY").ok();
            if api_key.is_none() {
                println!("WARN: MY_PROVIDER_API_KEY not set.");
            }
            Self {
                client: reqwest::Client::new(),
                api_key,
                // ...
            }
        }
        // Helper methods for API calls...
    }

    #[async_trait]
    impl LlmProvider for MyProvider {
        fn get_name(&self) -> &str { "my_provider" }

        async fn check_readiness(&self) -> ReplResult<()> {
            if self.api_key.is_some() { Ok(()) }
            else { Err(ReplError::Provider("MY_PROVIDER_API_KEY environment variable not set.".to_string())) }
        }

        async fn get_models(&self) -> ReplResult<Vec<String>> {
            // Logic to call the provider's API endpoint for listing models
            // Parse the response and return Vec<String>
            Ok(vec!["model-a".to_string(), "model-b".to_string()]) // Placeholder
        }

        async fn query(&self, model: &str, prompt: &str) -> ReplResult<String> {
            // Logic to call the provider's non-streaming generation API
            // Check API key, build request, send, handle errors, parse response
             let key = self.api_key.as_ref().ok_or(ReplError::Provider("API Key missing".to_string()))?;
             // ... make API call ...
            Ok("Non-streaming response from my_provider".to_string()) // Placeholder
        }

        async fn query_stream(
            &self, model: &str, prompt: &str,
        ) -> ReplResult<Option<Pin<Box<dyn Stream<Item = ReplResult<String>> + Send>>>> {
            // Logic to call the provider's streaming API (if available)
            // Return Ok(None) if streaming is not supported
             let key = self.api_key.as_ref().ok_or(ReplError::Provider("API Key missing".to_string()))?;
            // ... setup streaming request ...
            // If streaming: create a stream (e.g., using unfold + StreamDeserializer or SSE parsing)
            // return Ok(Some(Box::pin(your_stream)));
             Ok(None) // Placeholder: Streaming not implemented
        }

        fn clone_box(&self) -> Box<dyn LlmProvider> {
            Box::new(self.clone()) // Requires MyProvider to derive Clone
        }
    }

Declare Module: Open src/providers/mod.rs and add pub mod my_provider;.

Register Provider: Open src/providers/mod.rs. Inside the ProviderRegistry::new function, add your new provider:

      
// Inside ProviderRegistry::new in src/providers/mod.rs:
registry.register(Box::new(my_provider::MyProvider::new()));

Rebuild, potentially set the MY_PROVIDER_API_KEY environment variable, and you should be able to use /provider my_provider.
Adding a New Theme

    Define Palette/Skin: Edit src/render.rs.

        Add color constants for your theme.

        Create a get_my_theme_palette() -> ThemePalette function.

        Create a create_my_theme_skin() -> MadSkin function.

    Update Enum: Add your theme variant to the RenderTheme enum in src/state.rs.

    Update Mappings: Edit src/commands/theme.rs.

        Add your theme to the SelectableTheme enum (for interactive selection).

        Update the From<SelectableTheme> for RenderTheme implementation.

        Update the theme_to_index function.

        Add your theme name to the match arg_lower.as_str() block in ThemeCommand::execute.

    Update Resources: Edit src/render.rs. Update the get_theme_resources function to return your new palette and skin for your RenderTheme variant.

Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs, feature requests, or suggestions.
License
