#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::LoopSignal;
use regex::Regex;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct CodeFunction {
    pub name: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionscraperInternalState {
}

fn extract_function_details(file_path: &str) -> Result<Vec<CodeFunction>, Box<dyn Error>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let re = Regex::new(r#"\{"([^:]+:[^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\}"#)?;
    
    let mut function_details = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if let Some(captures) = re.captures(&line) {
            let function = CodeFunction {
                name: captures[1].to_string(),
                filepath: captures[2].to_string(),
                start_line: captures[3].parse()?,
                end_line: captures[4].parse()?,
            };
            function_details.push(function);
        }
    }
    
    Ok(function_details)
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

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut loop_feedback_rx = loop_feedback_rx.lock().await;
   let mut functions_tx = functions_tx.lock().await;

       

        // Main loop - wait for any additional signals
        while cmd.is_running(&mut ||loop_feedback_rx.is_closed_and_empty() && functions_tx.mark_closed()) {
            let clean = await_for_all!(cmd.wait_periodic(Duration::from_secs(10)));


                // Initial scrape of functions
            match extract_function_details("hashmap_function.txt") {
                Ok(functions) => {
                    trace!("Found {} functions to process", functions.len());
                    
                    // Send each function through the channel
                    for function in functions {
                        match cmd.try_send(&mut functions_tx, function.clone()) {
                            Ok(()) => {
                                println!("Successfully sent function: {}", function.name);
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

            match cmd.try_take(&mut loop_feedback_rx) {
                Some(signal) => {
                    if !signal.state {
                        break
                    }
                    trace!("Received loop signal: {:?}", signal);
                    // Could implement re-scanning of the file here if needed
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
        // Create a temporary test file
        let test_content = r#"{"test_func:namespace", "test/path.rs", 1, 10}"#;
        std::fs::write("test_functions.txt", test_content).unwrap();

        // Test the extraction
        let results = extract_function_details("test_functions.txt").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_func:namespace");
        assert_eq!(results[0].filepath, "test/path.rs");
        assert_eq!(results[0].start_line, 1);
        assert_eq!(results[0].end_line, 10);

        // Clean up
        std::fs::remove_file("test_functions.txt").unwrap();
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