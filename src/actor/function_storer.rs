#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::ArchivedFunction;

use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufRead, BufReader};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use std::env;
use std::fs;



//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionstorerInternalState {
}

/*
    Function: get_file_modified_time

    Description:
    Retrieves the last modified timestamp of a file and formats it as a human-readable string.

    Parameters:
    - file_path: String — The path to the file whose modified time is to be retrieved.

    Returns:
    - Result<String, String> — Returns Ok(String) containing the formatted date and time on success.
      On failure, returns an error message detailing the issue.

    Errors:
    - Returns an error if:
      - The file metadata cannot be retrieved.
      - The modified time cannot be accessed or converted.

    Side Effects:
    - Reads the metadata of the file located at the given path.
    - Converts the modification timestamp into a formatted string.

    Notes:
    - Uses Chrono for date-time formatting.
    - Formats timestamps as "YYYY-MM-DD HH:MM:SS" in local time.
    - Helps track file modifications for logging or auditing purposes.
*/
fn get_file_modified_time(file_path: String) -> Result<String, String> {
    let path = Path::new(&file_path);

    // Get metadata
    let metadata = fs::metadata(path).map_err(|e| format!("Failed to get metadata: {}", e))?;

    // Get modified time
    let modified_time = metadata.modified().map_err(|e| format!("Failed to get modified time: {}", e))?;

    // Convert to chrono DateTime
    let datetime: DateTime<Local> = modified_time.into();

    // Format as string and return
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

/*
    Function: generate_markdown

    Description:
    Generates a formatted Markdown report for a reviewed function, including severity, description, and file details.

    Parameters:
    - archived_fn: &ArchivedFunction — A reference to the reviewed function containing metadata and review details.

    Returns:
    - String — A formatted Markdown string representing the function's review.

    Errors:
    - Does not return an error explicitly, but:
      - If review components are missing, default placeholders are used.
      - If the file modified time retrieval fails, an error message is embedded in the output.

    Side Effects:
    - Reads the last modified time of the function’s file.
    - Parses structured AI-generated review text using double backtick delimiters.

    Notes:
    - Uses color-coded severity levels (green for low, orange for medium, red for high).
    - Extracts function names from composite keys when applicable.
    - Formats information using a Markdown table for clarity.
*/
pub fn generate_markdown(archived_fn: &ArchivedFunction) -> String {
    // Parse the review message using double backticks
    let review_msg = archived_fn.review_message
        .trim_matches('{')
        .trim_matches('}')
        .trim();
    
    // Split by double backticks
    let parts: Vec<&str> = review_msg.split("``").collect();
    
    // Extract components with safe indexing
    // let function_name = parts.get(0).map_or("Unknown", |s| s.trim());
    let severity = parts.get(1).map_or("Unknown", |s| s.trim());
    let review_text = parts.get(2).map_or("No review provided", |s| s.trim());
    
    // Determine color based on severity
    let severity_color = match severity {
        "1" => "<span style=\"color:green;\">Low Severity</span>",
        "2" => "<span style=\"color:orange;\">Medium Severity</span>",
        "3" => "<span style=\"color:red;\">High Severity</span>",
        _ => "<span style=\"color:gray;\">Unknown Severity</span>",
    };

    // Extract function name from composite key if present
    let display_name = if archived_fn.name.contains(':') {
        archived_fn.name.split(':').nth(1).unwrap_or(&archived_fn.name)
    } else {
        &archived_fn.name
    };

    let modified_time = match get_file_modified_time(archived_fn.filepath.clone()) {
        Ok(time) => time,
        Err(e) => format!("Error: {}", e),
    };

    // Return the formatted markdown string
    format!(
        "## Function: `{}`\n\n\
        | **Aspect**        | **Details** |\n\
        |-------------------|------------|\n\
        | **Severity**      | {} |\n\
        | **Description**   | {} |\n\
        | **File Location** | {} (Lines {}-{}) |\n\
        | **Last Modified** | {} |\n",
        display_name,
        severity_color,
        review_text,
        archived_fn.filepath,
        archived_fn.start_line,
        archived_fn.end_line,
        modified_time
    )
}

/*
    Function: get_base_directory

    Description:
    Determines the base directory for storing review outputs by checking environment variables and a `.env` file.
    Falls back to the user's home directory or current directory if no configuration is found.

    Parameters:
    - None (The function operates based on environment variables and file content).

    Returns:
    - io::Result<String> — Returns Ok(String) with the determined base directory.
      If an error occurs while reading the `.env` file, the default directory is returned.

    Errors:
    - Does not return an explicit error, but:
      - Logs a warning if the home directory cannot be determined.
      - Logs a warning if the `.env` file cannot be opened or read.
      - Defaults to the current directory if no valid configuration is found.

    Side Effects:
    - Reads the `HOME` environment variable as the default directory.
    - Attempts to read and parse the `.env` file for a `REVIEW_OUTPUT` variable.
    - Logs warnings if the expected configuration values are missing or cannot be accessed.

    Notes:
    - Ensures a fallback mechanism in case configuration values are missing.
    - Strips quotes from extracted values to maintain cleanliness.
    - Provides a flexible method for dynamically setting the base directory for review outputs.
*/
fn get_base_directory() -> io::Result<String> {
    // Get home directory from environment variable
    let default_dir = env::var("HOME")
        .unwrap_or_else(|_| {
            warn!("⚠️ Could not determine home directory. Using current directory.");
            ".".to_string()
        });
    
    // Try to open the .env file
    let env_file = match File::open(".env") {
        Ok(file) => file,
        Err(e) => {
            warn!("⚠️ Could not open .env file: {}. Using default directory.", e);
            return Ok(default_dir);
        }
    };

    let reader = BufReader::new(env_file);
    
    // Read the .env file line by line
    for line in reader.lines() {
        let line = line?;
        
        // Skip empty lines and comments
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Look for the DIRECTORY variable
        if line.starts_with("REVIEW_OUTPUT=") {
            let directory = line
                .split('=')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| default_dir.clone());
                
            // Remove quotes if present
            let directory = directory
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
                
            return Ok(directory);
        }
    }
    
    // If DIRECTORY variable not found, return default
    warn!("⚠️ DIRECTORY variable not found in .env file. Using default directory.");
    Ok(default_dir)
}

/*
    Function: store_function

    Description:
    Saves a reviewed function's Markdown report to a structured directory based on its file path.
    Ensures the review output directory exists and maintains historical reviews using an append mode.

    Parameters:
    - archived_fn: &ArchivedFunction — A reference to the reviewed function containing metadata and review details.

    Returns:
    - io::Result<()> — Returns Ok(()) on success.
      If an error occurs during file or directory operations, it returns an appropriate I/O error.

    Errors:
    - Returns an error if:
      - The review output directory cannot be created.
      - The review file cannot be opened or written to.

    Side Effects:
    - Generates a Markdown-formatted review using `generate_markdown()`.
    - Reads the base directory from a `.env` file or falls back to a default location.
    - Creates a structured directory under `review_output` to store reviews.
    - Appends new review content to the file while preserving previous entries.

    Notes:
    - Adjusts file paths dynamically, ensuring absolute paths are handled appropriately.
    - Uses separators (`---`) between reviews to maintain clarity in multi-entry files.
    - Converts review files to `.md` format for easier readability.
*/
async fn store_function(archived_fn: &ArchivedFunction) -> io::Result<()> {
    let markdown = generate_markdown(archived_fn);

    // Get the base directory from .env file
    let base_dir = get_base_directory()?;
    let review_base_dir = PathBuf::from(base_dir).join("review_output");
    
    // Create the base review output directory
    fs::create_dir_all(&review_base_dir)?;

    // Create the full review file path by combining review_output with the original path
    let mut review_file_path = review_base_dir;
    
    // Handle absolute paths by removing the leading slash if present
    let relative_path = if archived_fn.filepath.starts_with('/') {
        archived_fn.filepath.trim_start_matches('/').to_string()
    } else {
        archived_fn.filepath.clone()
    };
    
    // Push each component of the path
    for component in Path::new(&relative_path).components() {
        match component {
            std::path::Component::Normal(c) => review_file_path.push(c),
            _ => continue, // Skip other component types (root, parent dir, etc.)
        }
    }

    // Create parent directories if they don't exist
    if let Some(parent) = review_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Change the extension to .md
    review_file_path.set_extension("md");

    println!("📝 Appending review to: {:?}", review_file_path);

    // Open the file in append mode, creating it if it doesn't exist
    let mut file = OpenOptions::new()
        .create(true)      // Create the file if it doesn't exist
        .append(true)      // Open in append mode
        .write(true)       // Open in write mode
        .open(&review_file_path)?;

    // Add a separator between reviews if the file is not empty
    if file.metadata()?.len() > 0 {
        writeln!(file, "\n---\n")?;
    }

    // Write the markdown content
    file.write_all(markdown.as_bytes())?;

    Ok(())
}



// #[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [archived_rx],[]);
  internal_behavior(cmd,archived_rx, state).await
}

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    archived_rx: SteadyRx<ArchivedFunction>, 
    state: SteadyState<FunctionstorerInternalState>
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || FunctionstorerInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {
        let mut archived_rx = archived_rx.lock().await;
        
        while cmd.is_running(&mut ||archived_rx.is_closed_and_empty()) {
            let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut archived_rx,1));

            match cmd.try_take(&mut archived_rx) {
                Some(function) => {
                    if let Err(e) = store_function(&function).await {
                        error!("Failed to store function: {}", e);
                    } else {
                        trace!("Stored function: {:?}", function);
                    }
                }
                None => {
                    if clean {
                        trace!("No function available to process");
                    }
                }
            }
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
       let (test_archived_tx,archived_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, archived_rx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_archived_tx.testing_send_all(vec![ArchivedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));}
}