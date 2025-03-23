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
    pub namespace: String,
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
            namespace: String::new(),
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

fn extract_function_details(file_path: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    
    // New regex to match: {"function_name", "file_path", start_line, end_line}
    let re = Regex::new(r#"\{\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\s*\}"#)?;

    let mut function_details = HashMap::new();
    
    for line in reader.lines() {
        let line = line?;
        if let Some(captures) = re.captures(&line) {
            // Extract function name and file path
            let function_name = captures[1].to_string();
            let filepath = captures[2].to_string();
            
            // Create composite key in format "filepath:function_name"
            let composite_key = format!("{}:{}", filepath, function_name);
            
            // Debug print
            println!("🔍 Extracted -> Composite Key: {} | Path: {}", composite_key, filepath);

            // Insert into HashMap: Key = filepath:function_name, Value = filepath
            function_details.insert(composite_key, filepath);
        }
    }
    
    // Write the populated HashMap to file
    if let Err(e) = write_hashmap_to_file(&function_details) {
        error!("❌ Failed to write HashMap to file: {:?}", e);
    }
    
    Ok(function_details)
}

fn read_function_content(filepath: &str, start_line: usize, end_line: usize) -> Result<String, Box<dyn Error>> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines()
        .collect::<Result<Vec<_>, _>>()?;
    
    let content = lines[start_line - 1..end_line]
        .join("\n");
    
    Ok(content)
}

fn extract_function_from_signal(signal: &LoopSignal) -> Result<CodeFunction, Box<dyn Error>> {
    println!("Extracting function from signal: {:?}", signal);
    
    let function_name = signal.key.clone();
    let expected_filepath = signal.filepath.clone();
    
    let file = File::open("test.txt")?;
    let reader = BufReader::new(file);
    
    let re = Regex::new(r#"\{\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\s*\}"#)?;

    println!("🔍 Looking for function '{}' in file '{}'", function_name, expected_filepath);

    for line in reader.lines() {
        let line = line?;
        println!("Checking line: {}", line);

        if let Some(captures) = re.captures(&line) {
            let captured_name = captures[1].to_string();
            let filepath = captures[2].to_string();
            let start_line: usize = captures[3].parse()?;
            let end_line: usize = captures[4].parse()?;

            // Check if both function name and filepath match
            if captured_name == function_name && filepath == expected_filepath {
                println!("✅ Found matching function '{}' in file '{}' (lines {}-{})", 
                    captured_name, filepath, start_line, end_line);

                // Read the actual function content from the file
                let content = read_function_content(&filepath, start_line, end_line)?;

                return Ok(CodeFunction {
                    name: function_name,
                    namespace: String::from(""),
                    filepath,
                    start_line,
                    end_line,
                    content,
                    function_map: signal.remaining_functions.clone(),
                });
            } else {
                println!("❌ No match: Expected '{}' in '{}', found '{}' in '{}'",
                    function_name, expected_filepath, captured_name, filepath);
            }
        }
    }

    Err(format!("❌ Function '{}' not found in file '{}' at path '{}'", 
        function_name, "test.txt", expected_filepath).into())
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
    println!("🚀 FunctionScraper is fired up.");

    let mut state_guard = steady_state(&state, || FunctionscraperInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {
        let mut parsed_code_rx = parsed_code_rx.lock().await;
        let mut loop_feedback_rx = loop_feedback_rx.lock().await;
        let mut functions_tx = functions_tx.lock().await;

        println!("📌 Entering main loop of FunctionScraper...");
        let mut loop_check = true;

        while cmd.is_running(&mut || parsed_code_rx.is_closed_and_empty() && functions_tx.mark_closed() && loop_feedback_rx.is_closed_and_empty()) {
            println!("✅ Inside the loop!");

            // Wait for messages from both channels
            if loop_check {
                let _clean = await_for_all!(
                    cmd.wait_closed_or_avail_units(&mut parsed_code_rx, 1)
                    // cmd.wait_closed_or_avail_units(&mut loop_feedback_rx, 1)
                );
                while let Some(parsed_code) = cmd.try_take(&mut parsed_code_rx) {
                    println!("✅ Received ParsedCode: {:?}", parsed_code);
                    
                    // Create initial CodeFunction from the first function
                    if let Ok(functions) = extract_function_details("test.txt") {
                        if let Some(captures) = Regex::new(r#"\{\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\s*\}"#)
                            .unwrap()
                            .captures(&parsed_code.firstFunction) 
                        {
                            let function_name = captures[1].to_string();
                            let path = captures[2].to_string();
                            let start_line: usize = captures[3].parse().unwrap_or(0);
                            let last_line: usize = captures[4].parse().unwrap_or(0);
    
                            if let Ok(function_content) = read_function_content(&path, start_line, last_line) {
                                let container = CodeFunction {
                                    name: function_name,
                                    namespace: String::from("global"),
                                    filepath: path, 
                                    start_line,
                                    end_line: last_line,
                                    content: function_content,
                                    function_map: functions,
                                };
                                
                                println!("📤 Sending initial function: {:?}", container.name);
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


            // Process any available parsed code messages
            

            // Wait for and process any loop feedback signals
            // let _clean = await_for_all!(
            //     cmd.wait_closed_or_avail_units(&mut loop_feedback_rx, 1)
            // );


        }
    }
    Ok(())
}