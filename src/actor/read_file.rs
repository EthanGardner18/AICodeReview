/*
    File: readfile.rs

    Description:
    This module defines an internal actor responsible for recursively scanning a directory 
    (defined in the `.env` file via the `DIRECTORY` variable), reading source code files 
    with common programming language extensions, chunking their contents with line numbers, 
    and sending them asynchronously to a connected processing pipeline via SteadyState channels.

    Features:
    - Supports scanning nested directories
    - Filters files by predefined extensions (e.g., .rs, .py, .js)
    - Chunks file contents into 100KB segments with line numbers
    - Sends each chunk via a SteadyTx channel, marking the last chunk/file
    - Includes async runtime, error handling, and test coverage

    Dependencies:
    - Uses `dotenv` to load file path from environment
    - Relies on `steady_state` for actor and channel communication
    - Designed for use in modular pipelines or async task graphs

    Usage:
    Set `DIRECTORY` in `.env`, then call `run()` with appropriate context.
    See `test_simple_process` for a unit test example.
*/

#[allow(unused_imports)]
use log::*;
use std::time::Duration;
use steady_state::*;
// use crate::Args;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use dotenv::dotenv;
use std::env;

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileData {
    pub path: String,
    pub content: String,
    pub last_file: String,
}

#[derive(Default)]
pub(crate) struct ReadfileInternalState {}

#[cfg(not(test))]
pub async fn run(
    context: SteadyContext,
    file_data_tx: SteadyTx<FileData>,
    state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    // let _cli_args = context.args::<Args>();
    let cmd = into_monitor!(context, [], [file_data_tx]);
    internal_behavior(cmd, file_data_tx, state).await
}

const MAX_CHUNK_SIZE: usize = 100 * 1024; // 100 KB
/*
    Function: scan_directory_for_files

    Description:
    Recursively scans the specified directory and all of its subdirectories 
    for files that match a given list of file extensions. Returns a list of 
    file paths that have the specified extensions.

    Parameters:
    path: A reference to a `Path` representing the directory to start scanning from.
    extensions: A slice of string slices (`&[&str]`) representing file extensions 
                to filter by (e.g., ["rs", "txt", "json"]).

    Returns:
    Vec<PathBuf>: A vector of file paths that have the specified extensions.

    Notes:
    - The function silently skips over directories that can't be read.
    - Extensions are matched without the leading dot and are case-sensitive.

    Example:
    let files = scan_directory_for_files(Path::new("src"), &["rs"]);
    for file in files {
        println!("{}", file.display());
    }
*/
fn scan_directory_for_files(path: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    // Initialize a vector to store the paths of matched files
    let mut found_files = Vec::new();

    // Check if the given path is a directory
    if path.is_dir() {
        // Try to read entries (files/folders) in the directory
        if let Ok(entries) = fs::read_dir(path) {
            // Iterate over all entries, ignoring any that result in an error
            for entry in entries.flatten() {
                // Get the full path of the current entry
                let entry_path = entry.path();

                // If the entry is a directory, recursively scan it
                if entry_path.is_dir() {
                    // Recursively scan subdirectories and add the results to found_files
                    found_files.extend(scan_directory_for_files(&entry_path, extensions));
                } 
                // If the entry is a file, check if it matches the desired extensions
                else if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    // If the file extension is in the list of desired extensions, store it
                    if extensions.contains(&ext) {
                        found_files.push(entry_path);
                    }
                }
            }
        } else {
            // If the directory couldn't be read, print an error message
            eprintln!("Failed to read directory '{}'", path.display());
        }
    }

    // Return the list of matching files found
    found_files
}

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,                                        // The commander object controlling the actor
    file_data_tx: SteadyTx<FileData>,                  // The sending channel used to transmit FileData
    state: SteadyState<ReadfileInternalState>,         // Persistent internal state for this actor
) -> Result<(), Box<dyn Error>> {

    
    
    // Initialize or retrieve the persistent state for this actor
    let mut state_guard = steady_state(&state, || ReadfileInternalState::default()).await;

    // Only proceed if we successfully got a mutable reference to the state
    if let Some(mut _state) = state_guard.as_mut() {
        // Lock the sending channel so we can send data through it
        let mut file_data_tx = file_data_tx.lock().await;

        // Read the DIRECTORY path from the .env file
        dotenv().ok();
        let input_path = env::var("DIRECTORY").unwrap_or_else( |err |{
            println!("{:#?}", err);
            panic!("ERROR INPUT_PATH");
        });
        

        // Convert the directory string to a PathBuf for filesystem operations
        let input_dir = PathBuf::from(&input_path);

        // List of supported coding file extensions
        let coding_extensions = [
            "py", "cpp", "hpp", "cc", "cxx", "rs", "c", "js", "jsx", "ts", "tsx", "java", "go",
            "sh", "php", "rb", "kt", "kts", "swift", "pl", "pm", "r", "pas", "f90", "lisp", "cbl",
            "zig",
        ];

        // Recursively scan the input directory for code files with the given extensions
        let found_files = scan_directory_for_files(&input_dir, &coding_extensions);

        // Total number of files found
        let total_files = found_files.len();

        // Counter to track how many files we've processed so far
        let mut processed_files = 0;

        // Main loop: while the actor is still running, wait and process files
        while cmd.is_running(&mut || file_data_tx.mark_closed()) {
            // Wait 1 second between iterations (simulate a periodic heartbeat)
            let _clean = await_for_all!(cmd.wait_periodic(Duration::from_millis(1000)));

            // Loop over each file found in the directory
            for file_path in &found_files {
                processed_files += 1; // Increment file counter
                let is_last_file = processed_files == total_files; // Check if it's the last file

                // Try reading the file contents into a string
                let content = fs::read_to_string(file_path).unwrap_or_else(|_| {
                    eprintln!("Failed to read file: {}", file_path.display());
                    String::new()
                });

                // Buffers to store chunked file content before sending
                let mut chunk_buf = String::new();
                let mut current_chunk_size = 0;
                let mut line_number = 1;

                // Go through each line in the file
                for line in content.lines() {
                    // Add line numbers to each line of the file
                    let numbered_line = format!("{:>6}: {}\n", line_number, line);
                    let line_bytes_len = numbered_line.as_bytes().len();
                    line_number += 1;

                    // If current chunk size exceeds MAX_CHUNK_SIZE, send it as a chunk
                    if current_chunk_size + line_bytes_len > MAX_CHUNK_SIZE {
                        let data = FileData {
                            path: file_path.display().to_string(),
                            content: chunk_buf.clone(),
                            last_file: "F".to_string(), // "F" indicates this is not the last chunk
                        };

                        // Try sending the chunk through the channel
                        match cmd.try_send(&mut file_data_tx, data) {
                            Ok(()) => {
                               eprintln!("Sent chunk ({} bytes)", chunk_buf.len());
                            }
                            Err(msg) => trace!("Error sending: {:?}", msg),
                        }

                        // Clear the buffer and reset size counter for the next chunk
                        chunk_buf.clear();
                        current_chunk_size = 0;
                    }

                    // Add the current line to the buffer and update size
                    chunk_buf.push_str(&numbered_line);
                    current_chunk_size += line_bytes_len;
                }

                // After all lines are processed, send any remaining data as the final chunk
                if !chunk_buf.is_empty() {
                    let data = FileData {
                        path: file_path.display().to_string(),
                        content: chunk_buf.clone(),
                        last_file: if is_last_file { "T".to_string() } else { "F".to_string() }, // "T" means final chunk of final file
                    };

                    // Send the final chunk
                    match cmd.try_send(&mut file_data_tx, data) {
                        Ok(()) => {
                            eprintln!(
                                "Sent final chunk ({} bytes) {}",
                                chunk_buf.len(),
                                if is_last_file { "(last chunk of last file)" } else { "" }
                            );

                            // If it's the last file, close the channel and exit
                            if is_last_file {
                                file_data_tx.mark_closed();
                                
                                return Ok(());
                            }
                        }
                        Err(msg) => trace!("Error sending: {:?}", msg),
                    }
                }
            }
        }
    }

    // Function ends successfully if all logic completes
    Ok(())
}


#[cfg(test)]
pub async fn run(
    context: SteadyContext,
    file_data_tx: SteadyTx<FileData>,
    _state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut cmd = into_monitor!(context, [], [file_data_tx]);
    if let Some(responder) = cmd.sidechannel_responder() {
        let mut file_data_tx = file_data_tx.lock().await;
        while cmd.is_running(&mut || file_data_tx.mark_closed()) {
            let _did_echo = responder.echo_responder(&mut cmd, &mut file_data_tx).await;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;
    use steady_state::*;
    use super::*;

    #[async_std::test]
    pub(crate) async fn test_simple_process() {
        let mut graph = GraphBuilder::for_testing().build(());
        let (file_data_tx, test_file_data_rx) = graph.channel_builder().with_capacity(4).build();
        let state = new_state();
        graph.actor_builder()
            .with_name("UnitTest")
            .build_spawn(move |context| internal_behavior(context, file_data_tx.clone(), state.clone()));
        graph.start();
        graph.request_stop();
        graph.block_until_stopped(Duration::from_secs(15));
        let _results_file_data_vec = test_file_data_rx.testing_take().await;
    }
}