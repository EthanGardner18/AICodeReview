#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::LoopSignal;
use crate::actor::parse_function::ParsedCode;

use regex::Regex;
use std::fs::File;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::fs::OpenOptions;
use std::io::Write;




#[derive(Clone,Debug,Eq,PartialEq)]
pub(crate) struct CodeFunction {
    pub name: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub function_map: HashMap<String, String>,
}


impl Default for CodeFunction {
    fn default() -> Self {
        CodeFunction {
            name: String::new(),
            filepath: String::new(),
            start_line: 0,
            end_line: 0,
            content: String::new(),
            function_map: HashMap::new(),
        }
    }
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionscraperInternalState {
}

/*
    Function: write_hashmap_to_file

    Description:
    Writes the contents of a given HashMap<String, String> to a file named "test_hashmap.txt".
    Each key-value pair is written on a new line in the format: "Key: ... | Value: ...".

    Parameters:
    - hashmap: &HashMap<String, String> — A reference to the HashMap containing data to be written.

    Returns:
    - Result<(), Box<dyn Error>> — Returns Ok(()) on success. On failure, returns an error if the file
      could not be created, opened, or written to.

    Errors:
    - Returns an error if the file operation fails (e.g., permission denied, I/O error).

    Side Effects:
    - Creates or overwrites a file named "test_hashmap.txt" in the current working directory.
    - Prints a confirmation message to standard output upon successful write.

    Notes:
    - The file is truncated before writing to ensure old data is cleared.
    - This function is typically used for debugging or persisting analysis results.
*/
fn write_hashmap_to_file(hashmap: &HashMap<String, String>) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("test_hashmap.txt")?;

    for (key, value) in hashmap.iter() {
        writeln!(file, "Key: {} | Value: {}", key, value)?;
    }

    println!("📝 Wrote HashMap contents to test_hashmap.txt");
    Ok(())
}

/*
    Function: extract_function_details

    Description:
    Parses a file containing structured function metadata entries, extracts relevant information,
    and returns a HashMap mapping a composite key of "filepath:function_name" to the function's file path.
    This is typically used to index available function definitions from a pre-parsed file.

    Parameters:
    - file_path: &str — Path to the file that contains lines of function metadata in the format:
      {"function_name", "file_path", start_line, end_line}.

    Returns:
    - Result<HashMap<String, String>, Box<dyn Error>> — On success, returns a HashMap where each key is a
      "filepath:function_name" composite string and the value is the function's file path. On failure,
      returns an error indicating what went wrong (e.g., file read error, regex parse error).

    Errors:
    - Returns an error if the file cannot be opened or the regex fails to compile.
    - Lines that fail to read or parse correctly are skipped with errors logged.

    Side Effects:
    - Attempts to write the extracted HashMap to a file using `write_hashmap_to_file`.
      Any failure in this operation is logged but does not halt execution.

    Notes:
    - Lines that do not match the expected regex pattern are silently ignored.
    - This function is tolerant of partial failures: it continues processing on individual line errors.
*/
fn extract_function_details(file_path: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    // Attempt to open the file and create a buffered reader.
    let file = File::open(file_path).map_err(|e| {
        error!("Failed to open file {}: {:?}", file_path, e);
        e
    })?;
    let reader = BufReader::new(file);
    
    // Compile the regex to match the pattern: {"function_name", "file_path", start_line, end_line}.
    let re = Regex::new(r#"\{\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\s*\}"#).map_err(|e| {
        error!("Failed to compile regex: {:?}", e);
        e
    })?;
    
    let mut function_details = HashMap::new();
    
    // Process each line from the file.
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                error!("Error reading a line from {}: {:?}", file_path, e);
                continue; // Skip lines that produce an error.
            }
        };
        if let Some(captures) = re.captures(&line) {
            // Extract function name and file path.
            let function_name = captures[1].to_string();
            let filepath = captures[2].to_string();
            
            // Create a composite key in the format "filepath:function_name".
            let composite_key = format!("{}:{}", filepath, function_name);
            
            // Debug print (only active in debug builds to avoid cluttering production logs).
            #[cfg(debug_assertions)]
            trace!("🔍 Extracted -> Composite Key: {} | Path: {}", composite_key, filepath);

            // Insert the extracted data into the HashMap.
            function_details.insert(composite_key, filepath);
        }
    }
    
    // Write the populated HashMap to a file.
    // Note: This side effect is intentional and should be documented by the caller.
    if let Err(e) = write_hashmap_to_file(&function_details) {
        error!("❌ Failed to write HashMap to file: {:?}", e);
    }
    
    Ok(function_details)
}

/*
    Function: read_function_content

    Description:
    Reads and extracts a block of lines corresponding to a function definition from a given source file,
    based on specified starting and ending line numbers. This is typically used to retrieve the source
    code for a function when its line boundaries are known.

    Parameters:
    - filepath: &str — The path to the file containing the function definition.
    - start_line: usize — The line number where the function starts (1-indexed).
    - end_line: usize — The line number where the function ends (inclusive, 1-indexed).

    Returns:
    - Result<String, Box<dyn Error>> — On success, returns the concatenated string containing the
      lines of the function. On failure, returns an error indicating the cause (e.g., file not found,
      invalid line range, I/O failure).

    Errors:
    - Returns an error if the file cannot be read.
    - Returns an error if the provided line range is invalid (e.g., start > end, or lines out of bounds).

    Notes:
    - The line numbers are assumed to be 1-based (e.g., the first line in the file is line 1).
*/
fn read_function_content(filepath: &str, start_line: usize, end_line: usize) -> Result<String, Box<dyn Error>> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<Vec<_>, _>>()?;
    
    // Validate the provided line range.
    if start_line == 0 || start_line > end_line || end_line > lines.len() {
        return Err(format!(
            "Invalid line range: start_line = {}, end_line = {} for file '{}' with {} total lines.",
            start_line, end_line, filepath, lines.len()
        ).into());
    }
    
    // Read and join the specified range of lines (1-indexed).
    let content = lines[start_line - 1..end_line].join("\n");
    
    Ok(content)
}


/*
    Function: extract_function_from_signal

    Description:
    Extracts function details from a given LoopSignal by searching a parse file.
    It attempts to find a matching function entry based on the function name or its base name.

    Parameters:
    - signal: &LoopSignal — A reference to the signal containing function information.

    Returns:
    - Result<CodeFunction, Box<dyn Error>> — Returns Ok(CodeFunction) on success with extracted details.
      On failure, returns an error if the function could not be found or read.

    Errors:
    - Returns an error if:
      - The parse file cannot be opened.
      - A line cannot be read from the file.
      - The regex pattern fails to match function details.
      - Function content retrieval encounters an issue.

    Side Effects:
    - Reads from the file "parse_function.txt".
    - Logs trace details for debugging during the search process.
    - Can return a formatted error message upon failure.

    Notes:
    - Uses regular expressions for structured parsing of function details.
    - Both full function names and base names are considered for matches.
    - The function content is retrieved separately once a match is found.
*/

fn extract_function_from_signal(signal: &LoopSignal) -> Result<CodeFunction, Box<dyn Error>> {
    trace!("Extracting function from signal: {:?}", signal);
    
    // Retrieve the complete function name and explicitly extract the base name (portion after the last colon).
    let full_function_name = signal.key.clone();
    let base_function_name = signal.key
        .split(':')
        .last()
        .map(|name| name.to_string())
        .unwrap_or_else(|| {
            // Although split always returns at least one element, we explicitly log and return the full key.
            error!("No colon found in signal key; using full key as base name: {}", signal.key);
            signal.key.clone()
        });
    let expected_filepath = signal.filepath.clone();
    
    // Define the file path that contains the function details.
    let parse_file_path = "parse_function.txt";
    let file = File::open(parse_file_path).map_err(|e| {
        error!("Failed to open parse file '{}': {:?}", parse_file_path, e);
        e
    })?;
    let reader = BufReader::new(file);
    
    // Define the regex pattern as a constant for clarity and maintainability.
    const FUNCTION_REGEX_PATTERN: &str = r#"\{\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\s*\}"#;
    let function_re = Regex::new(FUNCTION_REGEX_PATTERN).map_err(|e| {
        error!("Regex pattern compilation failed for pattern '{}': {:?}", FUNCTION_REGEX_PATTERN, e);
        e
    })?;

    trace!(
        "🔍 Searching for function '{}' (base name: '{}') in file '{}' for expected filepath '{}'",
        full_function_name, base_function_name, parse_file_path, expected_filepath
    );

    // Iterate over each line in the parse file.
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| {
            error!("Error reading a line from '{}': {:?}", parse_file_path, e);
            e
        })?;
        trace!("Checking line: {}", line);

        if let Some(captures) = function_re.captures(&line) {
            let captured_function_name = captures[1].to_string();
            let captured_filepath = captures[2].to_string();
            let start_line: usize = captures[3].parse().map_err(|e| {
                error!("Error parsing start line from captured data: {:?}", e);
                e
            })?;
            let end_line: usize = captures[4].parse().map_err(|e| {
                error!("Error parsing end line from captured data: {:?}", e);
                e
            })?;

            // Explicitly extract the base name for the captured function.
            let captured_base_name = captured_function_name
                .split(':')
                .last()
                .map(|name| name.to_string())
                .unwrap_or_else(|| captured_function_name.clone());

            // Match either the full function name or the base name.
            let is_match = captured_function_name == full_function_name ||
                           captured_base_name == base_function_name;

            if is_match && captured_filepath == expected_filepath {
                trace!(
                    "✅ Found matching function '{}' in file '{}' (lines {}-{})",
                    captured_function_name, captured_filepath, start_line, end_line
                );

                // Read the actual function content from the file.
                let content = read_function_content(&captured_filepath, start_line, end_line)?;

                return Ok(CodeFunction {
                    name: captured_function_name, // Retain the original captured name.
                    filepath: captured_filepath,
                    start_line,
                    end_line,
                    content,
                    function_map: signal.remaining_functions.clone(),
                });
            } else {
                trace!(
                    "❌ No match: Expected '{}' or '{}' in '{}' but found '{}' in '{}'",
                    full_function_name, base_function_name, expected_filepath,
                    captured_function_name, captured_filepath
                );
            }
        }
    }

    Err(format!(
        "Function '{}' (base name: '{}') not found in file '{}' for expected filepath '{}'",
        full_function_name, base_function_name, parse_file_path, expected_filepath
    ).into())
}


pub async fn run(context: SteadyContext
        ,loop_feedback_rx: SteadyRx<LoopSignal>
        ,parsed_code_rx: SteadyRx<ParsedCode>
        ,functions_tx: SteadyTx<CodeFunction>, state: SteadyState<FunctionscraperInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [loop_feedback_rx,parsed_code_rx],[functions_tx]);
  internal_behavior(cmd,loop_feedback_rx,parsed_code_rx, functions_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    loop_feedback_rx: SteadyRx<LoopSignal>,
    parsed_code_rx: SteadyRx<ParsedCode>,
    functions_tx: SteadyTx<CodeFunction>,
    state: SteadyState<FunctionscraperInternalState>,
) -> Result<(), Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionscraperInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {
        let mut parsed_code_rx = parsed_code_rx.lock().await;
        let mut loop_feedback_rx = loop_feedback_rx.lock().await;
        let mut functions_tx = functions_tx.lock().await;

        trace!("📌 Entering main loop of FunctionScraper...");
        let mut loop_check = true;

        while cmd.is_running(&mut || parsed_code_rx.is_closed_and_empty() && functions_tx.mark_closed() && loop_feedback_rx.is_closed_and_empty()) {
            println!("✅ Beginning Funciton review loop!");

            // Wait for messages from both channels
            if loop_check {
                let _clean = await_for_all!(
                    cmd.wait_closed_or_avail_units(&mut parsed_code_rx, 1)
                );
                while let Some(parsed_code) = cmd.try_take(&mut parsed_code_rx) {
                    trace!("✅ Received ParsedCode: {:?}", parsed_code);
                    
                    // Create initial CodeFunction from the first function
                    if let Ok(functions) = extract_function_details("parse_function.txt") {
                        if let Some(captures) = Regex::new(r#"\{\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\s*\}"#)
                            .unwrap()
                            //TODO REVERT first_function to camelCase
                            .captures(&parsed_code.first_function) 
                        {
                            let function_name = captures[1].to_string();
                            let path = captures[2].to_string();
                            let start_line: usize = captures[3].parse().unwrap_or(0);
                            let last_line: usize = captures[4].parse().unwrap_or(0);
    
                            if let Ok(function_content) = read_function_content(&path, start_line, last_line) {
                                let container = CodeFunction {
                                    name: function_name,
                                    filepath: path, 
                                    start_line,
                                    end_line: last_line,
                                    content: function_content,
                                    function_map: functions,
                                };
                                
                                trace!("📤 Sending initial function: {:?}", container.name);
                                if let Err(e) = cmd.try_send(&mut functions_tx, container) {
                                    error!("❌ Failed to send initial function: {:?}", e);
                                }
                            }
                        }
                    }
                }
            } else {
                let _clean = await_for_all!(
                    cmd.wait_avail_units(&mut loop_feedback_rx, 1)
                );

                while let Some(signal) = cmd.try_take(&mut loop_feedback_rx) {
                    match extract_function_from_signal(&signal) {
                        Ok(next_function) => {
                            println!("📤 Sending next function: {:?}", next_function.name);
                            if let Err(e) = cmd.try_send(&mut functions_tx, next_function) {
                                error!("❌ Failed to send function to reviewer: {:?}", e);
                            }
                        },
                        Err(e) => {
                            error!("❌ Failed to extract function from signal: {:?}", e);
                            cmd.request_graph_stop();
                        }
                    }
                }
            }
            loop_check = false;
        }
    }
    Ok(())
}