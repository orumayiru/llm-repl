// src/repl.rs
use crate::{
    commands::CommandRegistry,
    error::{ReplError, ReplResult},
    render::get_theme_resources, // Theme resources
    shell::execute_shell_command,
    state::{AppState, HistoryContentType, HistoryEntry, MarkdownMode, RenderTheme}, // Added History types
};
use colored::*; // For applying colors
use futures::StreamExt;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::{self, Write}; // Added io::Write
use tokio::runtime::Runtime;

// --- Repl Struct Definition ---
pub struct Repl {
    command_registry: CommandRegistry,
    state: AppState,
    runtime: Runtime,
}
// --- End Struct Definition ---

// --- Start impl Repl ---
impl Repl {
    pub fn new() -> ReplResult<Self> {
        let state = AppState::new();
        let runtime = Runtime::new().map_err(ReplError::Io)?;
        let command_registry = CommandRegistry::new(state.clone());
        Ok(Repl {
            command_registry,
            state,
            runtime,
        })
    }

    // Helper to apply color using RGB tuple from the palette
    fn colorize(&self, text: &str, color: (u8, u8, u8)) -> colored::ColoredString {
        text.truecolor(color.0, color.1, color.2)
    }

    // Render markdown using the specified theme's skin
    fn render_markdown(&self, markdown_text: &str, theme: RenderTheme) -> String {
        let (skin, _palette) = get_theme_resources(theme); // Get skin for the theme
        skin.term_text(markdown_text).to_string() // Convert FmtText to String
    }

    // --- Helper to add history entries ---
    async fn add_history(&self, entry_type: HistoryContentType, content: String) {
        self.state
            .add_history_entry(HistoryEntry { entry_type, content })
            .await;
    }
    // --- End Helper ---

    pub fn run(&mut self) -> ReplResult<()> {
        println!("LLM REPL - Type '/help' for commands, !<cmd> for shell, /reader for history.");
        // Removed redundant mode/theme prints here, covered by /help

        let mut rl = DefaultEditor::new()?;
        if rl.load_history("history.txt").is_err() {
            println!("INFO: No previous history found or load failed.");
        }

        loop {
            // --- Get State for Prompt ---
            let current_provider = self.runtime.block_on(self.state.get_provider_name());
            let current_model = self.runtime.block_on(self.state.get_model());
            let current_theme = self.runtime.block_on(self.state.get_theme());
            let (_skin, palette) = get_theme_resources(current_theme); // Get palette

            // --- Build Colored Prompt ---
            let prompt = format!(
                "{}{}{}{}{}{}",
                self.colorize("[", palette.prompt_bracket),
                self.colorize(&current_provider, palette.prompt_provider),
                self.colorize(":", palette.prompt_separator),
                self.colorize(&current_model, palette.prompt_model),
                self.colorize("]", palette.prompt_bracket),
                self.colorize(">> ", palette.prompt_arrow)
            );

            // --- Read Line ---
            let readline = rl.readline(&prompt);
            match readline {
                Ok(line) => {
                    if let Err(e) = rl.add_history_entry(line.as_str()) {
                        eprintln!(
                            "{}",
                            self.colorize(
                                &format!("WARN: Failed to add rustyline history entry: {}", e),
                                palette.error
                            )
                        );
                    }

                    let trimmed_line = line.trim();
                    if trimmed_line.is_empty() { continue; }

                    // --- Command Handling ---
                    if line.starts_with('/') {
                        let parts: Vec<&str> = line[1..].splitn(2, ' ').collect();
                        let (cmd, args) = if parts.len() > 1 { (parts[0], parts[1]) } else { (parts[0], "") };

                        match cmd {
                            "exit" | "quit" => break,
                            // --- Special Handling for /reader ---
                            "reader" => {
                                // Execute reader command, print its output, but DON'T store its output in history
                                match self.runtime.block_on(self.execute_command(cmd, args)) {
                                    Ok(msg) => println!("{}", msg), // Prints "Reader view finished..."
                                    Err(e) => {
                                        // Still log errors executing the reader itself
                                        let err_msg = format!("Error executing reader: {}", e);
                                        eprintln!("{}", self.colorize(&err_msg, palette.error));
                                        self.runtime.block_on(self.add_history(
                                            HistoryContentType::Error { source: "/reader".to_string() },
                                            err_msg,
                                        ));
                                    }
                                }
                            }
                            // --- Handle other commands ---
                            _ => {
                                let command_result = self.runtime.block_on(self.execute_command(cmd, args));
                                let current_theme_for_output = self.runtime.block_on(self.state.get_theme()); // Re-fetch theme
                                let (_skin_output, palette_output) = get_theme_resources(current_theme_for_output);

                                match command_result {
                                    Ok(output_content) => {
                                        let string_to_print;
                                        let current_mode = self.runtime.block_on(self.state.get_markdown_mode());

                                        if cmd == "llmconvo" {
                                            string_to_print = self.colorize(&output_content, palette_output.success).to_string();
                                        } else if current_mode != MarkdownMode::Off {
                                            string_to_print = self.render_markdown(&output_content, current_theme_for_output);
                                        } else {
                                            string_to_print = self.colorize(&output_content, palette_output.command_output_raw).to_string();
                                        }

                                        // Print the processed output
                                        println!("{}", string_to_print);

                                        // Store the original, unprocessed output string
                                        self.runtime.block_on(self.add_history(
                                            HistoryContentType::CommandResult { command: cmd.to_string() },
                                            output_content, // Store original string
                                        ));
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Error: {}", e);
                                        eprintln!("{}", self.colorize(&err_msg, palette_output.error));
                                        // Store the error message
                                        self.runtime.block_on(self.add_history(
                                            HistoryContentType::Error { source: format!("/{}", cmd) },
                                            err_msg,
                                        ));
                                    }
                                }
                            }
                        }
                    // --- Shell Command Handling ---
                    } else if line.starts_with('!') {
                        let command_line = line[1..].trim();
                        let current_theme_for_output = self.runtime.block_on(self.state.get_theme());
                        let (_skin_output, palette_output) = get_theme_resources(current_theme_for_output);

                        match execute_shell_command(command_line) {
                            Ok(output_content) => {
                                println!("{}", output_content.trim_end()); // Print raw
                                // Store raw output
                                self.runtime.block_on(self.add_history(
                                    HistoryContentType::ShellOutput { command: command_line.to_string() },
                                    output_content,
                                ));
                            }
                            Err(e) => {
                                let err_msg = format!("Shell Error: {}", e);
                                eprintln!("{}", self.colorize(&err_msg, palette_output.error));
                                // Store error
                                self.runtime.block_on(self.add_history(
                                    HistoryContentType::Error { source: format!("!{}", command_line) },
                                    err_msg,
                                ));
                            }
                        }
                    // --- LLM Query Handling ---
                    } else {
                        let current_theme_for_output = self.runtime.block_on(self.state.get_theme());
                        let (_skin_output, palette_output) = get_theme_resources(current_theme_for_output);
                        let info_msg = "Querying...";
                        println!("{}", self.colorize(info_msg, palette_output.info));
                        // Optionally store info message
                        // self.runtime.block_on(self.add_history(HistoryContentType::Info, info_msg.to_string()));

                        // Use the helper function to query, print, and collect
                        let query_result = self.runtime.block_on(
                            self.query_llm_and_collect(&line, current_theme_for_output),
                        );

                        match query_result {
                            // Helper already printed the output correctly
                            Ok((original_content, _printed_content)) => {
                                // Just store the original content
                                let model_name = self.runtime.block_on(self.state.get_model());
                                self.runtime.block_on(self.add_history(
                                    HistoryContentType::LlmResponse { model: model_name },
                                    original_content, // Store original (potentially raw MD)
                                ));
                            }
                            Err(e) => {
                                let err_msg = format!("LLM Error: {}", e);
                                eprintln!("{}", self.colorize(&err_msg, palette_output.error));
                                // Store error
                                self.runtime.block_on(self.add_history(
                                    HistoryContentType::Error { source: "LLM Query".to_string() },
                                    err_msg,
                                ));
                            }
                        }
                    }
                }
                // --- Readline Error Handling ---
                Err(ReadlineError::Interrupted) => {
                    let (_skin_exit, palette_exit) = get_theme_resources(RenderTheme::Default);
                    println!("\n{}", self.colorize("CTRL-C received, exiting.", palette_exit.info));
                    break;
                }
                Err(ReadlineError::Eof) => {
                    let (_skin_exit, palette_exit) = get_theme_resources(RenderTheme::Default);
                    println!("\n{}", self.colorize("CTRL-D received, exiting.", palette_exit.info));
                    break;
                }
                Err(err) => {
                    let (_skin_exit, palette_exit) = get_theme_resources(RenderTheme::Default);
                    eprintln!("{}", self.colorize(&format!("Readline Error: {}", err), palette_exit.error));
                    // Maybe don't store readline errors in app history? Up to you.
                    return Err(ReplError::Readline(err.to_string()));
                }
            }
        } // --- End Loop ---

        if let Err(e) = rl.save_history("history.txt") {
            let (_skin_exit, palette_exit) = get_theme_resources(RenderTheme::Default);
            eprintln!("{}", self.colorize(&format!("WARN: Failed to save rustyline history: {}", e), palette_exit.error));
        }
        Ok(())
    } // --- End run() ---


    async fn execute_command(&self, cmd: &str, args: &str) -> ReplResult<String> {
        if let Some(command) = self.command_registry.get_command(cmd) {
            command.execute(args).await
        } else {
            Err(ReplError::UnknownCommand(cmd.to_string()))
        }
    }


    // --- New Helper: query_llm_and_collect ---
    // Executes LLM query, handles printing based on mode, and returns
    // both the original content string and the string that was printed.
    async fn query_llm_and_collect(
        &self,
        prompt: &str,
        theme: RenderTheme,
    ) -> ReplResult<(String, String)> { // Returns (original_content, printed_content)
        if let Some(provider) = self.state.get_current_provider().await {
            let model = self.state.get_model().await;
            let current_mode = self.state.get_markdown_mode().await;
            let (skin, palette) = get_theme_resources(theme);

            match provider.query_stream(&model, prompt).await {
                 // --- Streaming Case ---
                Ok(Some(stream)) => {
                    let mut full_response = String::new(); // Collects original content
                    let mut printed_output_capture = String::new(); // Captures what's printed (approx)
                    let mut term = io::stdout();

                    match current_mode {
                        MarkdownMode::Off => {
                            let mut stream_pin = stream;
                            while let Some(chunk_result) = stream_pin.next().await {
                                let chunk = chunk_result?;
                                print!("{}", chunk); // Print directly
                                io::stdout().flush().map_err(ReplError::Io)?;
                                full_response.push_str(&chunk);
                                printed_output_capture.push_str(&chunk); // Capture raw
                            }
                            println!(); // Newline after stream
                            printed_output_capture.push('\n');
                            Ok((full_response, printed_output_capture))
                        }
                        MarkdownMode::AppendFormatted => {
                             let mut stream_pin = stream;
                             let mut raw_stream_print = String::new(); // Capture raw stream part
                             while let Some(chunk_result) = stream_pin.next().await {
                                 let chunk = chunk_result?;
                                 print!("{}", chunk); // Print raw chunk
                                 io::stdout().flush().map_err(ReplError::Io)?;
                                 full_response.push_str(&chunk);
                                 raw_stream_print.push_str(&chunk);
                             }
                             let separator = format!("\n\n{}", self.colorize("--- Formatted Response ---", palette.info));
                             let formatted = self.render_markdown(&full_response, theme);
                             println!("{}{}", separator, formatted); // Print separator + formatted

                             // Combine what was printed for history capture
                             printed_output_capture = format!("{}{}{}", raw_stream_print, separator, formatted);

                             Ok((full_response, printed_output_capture)) // Return raw MD, and combined printed string
                        }
                        MarkdownMode::LiveStreaming => {
                             // Capture final state for history, acknowledge live view was different
                              let mut stream_pin = stream;
                              let mut last_term_width = 0;
                              let mut previous_render_height = 0;
                              term.write_all(b"\x1B[?25l").map_err(ReplError::Io)?; // Hide cursor
                              term.flush().map_err(ReplError::Io)?;

                              let execution_result = async {
                                  while let Some(chunk_result) = stream_pin.next().await {
                                     match chunk_result {
                                        Ok(chunk) => {
                                            full_response.push_str(&chunk); // Collect original content
                                            // --- Live Rendering Logic ---
                                            let (width, _height) = termimad::terminal_size();
                                            let current_term_width = width as usize;
                                            if current_term_width == 0 { print!("{}", chunk); io::stdout().flush().map_err(ReplError::Io)?; continue; }
                                            let force_redraw = last_term_width != current_term_width;
                                            last_term_width = current_term_width;
                                            if previous_render_height > 0 && !force_redraw { term.write_all(format!("\x1B[{}A\x1B[J", previous_render_height).as_bytes()).map_err(ReplError::Io)?; }
                                            let rendered_string = skin.term_text(&full_response).to_string();
                                            term.write_all(rendered_string.as_bytes()).map_err(ReplError::Io)?;
                                            term.flush().map_err(ReplError::Io)?;
                                            previous_render_height = rendered_string.lines().count();
                                            // --- End Live Rendering ---
                                        }
                                        Err(e) => {
                                            // Print error during stream
                                             term.write_all(self.colorize("\n--- Stream Error Occurred (Mode: LiveStreaming) ---\n", palette.error).to_string().as_bytes()).map_err(ReplError::Io)?;
                                             return Err(e); // Propagate error
                                        }
                                     }
                                  }
                                   Ok(())
                              }.await;

                              // Cleanup cursor etc.
                              let _ = term.write_all(b"\x1B[?25h"); let _ = term.write_all(b"\n"); let _ = term.flush();

                              execution_result?; // Propagate error from streaming if any

                              // For history, store original MD, and maybe re-render final state?
                              // Let's store original MD, and the final rendered string as 'printed'
                              let final_rendered = self.render_markdown(&full_response, theme);
                              Ok((full_response, final_rendered))
                        }
                    }
                }
                 // --- Non-Streaming Case ---
                Ok(None) | Err(_) => {
                    // Fallback to non-streaming query
                    let response_content = provider.query(&model, prompt).await?;
                    if current_mode != MarkdownMode::Off {
                        let formatted = self.render_markdown(&response_content, theme);
                        Ok((response_content, formatted)) // Return raw and formatted
                    } else {
                        Ok((response_content.clone(), response_content)) // Return raw for both
                    }
                }
            }
        } else {
            let provider_name = self.state.get_provider_name().await;
            Err(ReplError::Provider(format!("Provider {} not found", provider_name)))
        }
    }
    // --- End New Helper ---

} // --- End impl Repl ---