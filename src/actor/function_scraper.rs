#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::LoopSignal;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct CodeFunction {
    pub name: String,
    pub namespace: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionscraperInternalState {
}

fn extract_function_details(file_path: &str) -> Result<Vec<CodeFunction>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let re = Regex::new(r#"\{"([^:]+):([^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\}"#)?;
    
    let mut function_details = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if let Some(captures) = re.captures(&line) {
            // Get the function content from the specified file and lines
            let name = captures[1].to_string();
            let namespace = captures[2].to_string();
            let filepath = captures[3].to_string();
            let start_line: usize = captures[4].parse()?;
            let end_line: usize = captures[5].parse()?;
            
            // Read the actual function content from the specified file
            let content = read_function_content(&filepath, start_line, end_line)?;
            
            let function = CodeFunction {
                name,
                namespace,
                filepath,
                start_line,
                end_line,
                content,
            };
            function_details.push(function);
        }
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

pub async fn run(context: SteadyContext
        ,loop_feedback_rx: SteadyRx<LoopSignal>
        ,functions_tx: SteadyTx<CodeFunction>, state: SteadyState<FunctionscraperInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [loop_feedback_rx],[functions_tx]);
  internal_behavior(cmd,loop_feedback_rx, functions_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    loop_feedback_rx: SteadyRx<LoopSignal>,
    functions_tx: SteadyTx<CodeFunction>, 
    state: SteadyState<FunctionscraperInternalState>
) -> Result<(),Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || FunctionscraperInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {
        let mut loop_feedback_rx = loop_feedback_rx.lock().await;
        let mut functions_tx = functions_tx.lock().await;

        // Initial scrape of functions
        match extract_function_details("test-1.txt") {
            Ok(functions) => {
                trace!("Found {} functions to process", functions.len());
                
                // Send each function through the channel
                for function in functions {
                    match cmd.try_send(&mut functions_tx, function.clone()) {
                        Ok(()) => {
                            trace!("Successfully sent function: {}", function.name);
                        },
                        Err(e) => {
                            error!("Failed to send function: {:?}", e);
                        },
                    }
                }
            },
            Err(e) => {
                error!("Failed to extract functions: {:?}", e);
            }
        }

        // Main loop - wait for any additional signals
        while cmd.is_running(&mut ||loop_feedback_rx.is_closed_and_empty() && functions_tx.mark_closed()) {
            let clean = await_for_all!(cmd.wait_periodic(Duration::from_secs(10)));

            match cmd.try_take(&mut loop_feedback_rx) {
                Some(signal) => {
                    trace!("Received loop signal: {:?}", signal);
                    cmd.request_graph_stop();
                },
                None => {
                    if clean {
                        trace!("No signals to process");
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[async_std::test]
    pub(crate) async fn test_function_scraping() {
        // Create a test source file with some function content
        let source_content = "fn test_function() {\n    println!(\"Hello\");\n}\n";
        std::fs::write("test_source.rs", source_content).unwrap();

        // Create the test-1.txt file with function metadata
        let test_content = r#"{"test_function:test_ns", "test_source.rs", 1, 3}"#;
        std::fs::write("test-1.txt", test_content).unwrap();

        // Test the extraction
        let results = extract_function_details("test-1.txt").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_function");
        assert_eq!(results[0].namespace, "test_ns");
        assert_eq!(results[0].filepath, "test_source.rs");
        assert_eq!(results[0].content, source_content.trim());

        // Clean up
        std::fs::remove_file("test-1.txt").unwrap();
        std::fs::remove_file("test_source.rs").unwrap();
    }

    #[async_std::test]
    pub(crate) async fn test_simple_process() {
        let mut graph = GraphBuilder::for_testing().build(());
        let (test_loop_feedback_tx, loop_feedback_rx) = graph.channel_builder().with_capacity(4).build();
        let (functions_tx, test_functions_rx) = graph.channel_builder().with_capacity(10).build();
        let state = new_state();

        // Create test file
        let test_content = r#"{"test_func:namespace", "test/path.rs", 1, 10}"#;
        std::fs::write("hashmap_function.txt", test_content).unwrap();

        graph.actor_builder()
            .with_name("UnitTest")
            .build_spawn(move |context| 
                internal_behavior(context, loop_feedback_rx.clone(), functions_tx.clone(), state.clone())
            );

        graph.start();
        
        // Wait a bit for processing
        async_std::task::sleep(Duration::from_millis(100)).await;
        
        // Check results
        let results = test_functions_rx.testing_take().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_func:namespace");

        graph.request_stop();
        graph.block_until_stopped(Duration::from_secs(15));

        // Clean up
        std::fs::remove_file("hashmap_function.txt").unwrap();
    }
}