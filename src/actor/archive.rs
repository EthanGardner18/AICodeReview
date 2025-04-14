#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::function_reviewer::ReviewedFunction;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ArchivedFunction {
    pub name: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub review_message: String,
}

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub struct LoopSignal {
    pub key: String,
    pub filepath: String,
    pub remaining_functions: HashMap<String, String>,
}


//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ArchiveInternalState {
}

/*
    Function: write_review_to_file

    Description:
    Writes the provided review content to a file named "stored_functions.txt".
    The content is appended to the file, ensuring previous entries are retained.

    Parameters:
    - review_content: &str — A reference to the review content that needs to be stored.

    Returns:
    - Result<(), Box<dyn Error>> — Returns Ok(()) on success.
      On failure, returns an error if the file operation fails (e.g., permission denied, I/O error).

    Errors:
    - Returns an error if:
      - The file cannot be created or opened.
      - Writing to the file fails due to system restrictions.

    Side Effects:
    - Appends data to "stored_functions.txt" in the current working directory.
    - Creates the file if it does not exist.
    - Adds a newline after each review entry for readability.

    Notes:
    - Ensures past review entries are preserved by using append mode.
    - Can be used for logging or maintaining a record of code reviews.
*/
pub async fn write_review_to_file(review_content: &str) -> Result<(), Box<dyn Error>> {
    let file_path = "stored_functions.txt";
    
    // Open file in append mode, create if doesn't exist
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    // Write the content and add a newline
    writeln!(file, "{}", review_content)?;
    
    Ok(())
}

/*
    Function: process_review_and_update_map

    Description:
    Processes a reviewed function's feedback and updates the remaining functions map.
    Determines the next function to review based on structured AI responses.

    Parameters:
    - reviewed_function: &mut ReviewedFunction — A mutable reference to the reviewed function.

    Returns:
    - Option<LoopSignal> — Returns Some(LoopSignal) if another function is available for review.
      If no further review is necessary, returns an empty LoopSignal.

    Errors:
    - Returns None if:
      - The review message does not contain the expected components.
      - The function to be reviewed next cannot be found in the remaining functions map.

    Side Effects:
    - Parses AI-generated review messages using predefined delimiters.
    - Logs debugging information for better traceability.
    - Updates the map of remaining functions by removing reviewed entries.

    Notes:
    - The AI-generated review follows a structured format with six expected components.
    - Matches functions using a composite key combining the file path and function name.
    - If no exact match is found, a partial match is attempted.
    - Ensures iterative review continues efficiently by tracking remaining functions.
*/
fn process_review_and_update_map(reviewed_function: &mut ReviewedFunction) -> Option<LoopSignal> {
    trace!("Review message: {}", reviewed_function.review_message);
    
    // Extract the parts using double backtick separation and remove curly braces
    let review_msg = reviewed_function.review_message
        .trim_matches('{')
        .trim_matches('}')
        .trim();
        
    let parts: Vec<&str> = review_msg.split("``").collect();
    
    // Check if we have all required parts
    if parts.len() < 6 {
        trace!("Not enough parts in review message. Expected 6, got {}", parts.len());
        trace!("Parts: {:?}", parts);
        return None;
    }
    
    // Extract components using array indexing
    // current_function, serverity, review_text not used in this function
    let _current_function = parts[0].trim();    // processFile
    let _severity = parts[1].trim();            // 1
    let _review_text = parts[2].trim();         // This function handles...
    let continue_flag = parts[3].trim();       // 1
    let next_function = parts[4].trim();       // validateInput
    let next_filepath = parts[5].trim();       // src/utils.rs
    
    // Clean up the continue flag - ensure we get just the number
    let should_continue = continue_flag == "1";
    
    if should_continue {
        trace!("Available functions in map: {:?}", reviewed_function.function_map.keys());
        
        // Create the composite key using the filepath and function name
        let composite_key = format!("{}:{}", next_filepath, next_function);
        trace!("Looking for composite key: {}", composite_key);
        
        // Try exact match with composite key
        if reviewed_function.function_map.contains_key(&composite_key) {
            let filepath = reviewed_function.function_map.get(&composite_key).unwrap();
            trace!("Found exact matching composite key: {}", composite_key);
            
            // Clone the HashMap and remove the found function
            let mut updated_map = reviewed_function.function_map.clone();
            updated_map.remove(&composite_key);
            
            let signal = LoopSignal {
                key: next_function.to_string(),
                filepath: filepath.clone(),
                remaining_functions: updated_map,
            };
            trace!("Created LoopSignal: {:?}", signal);
            return Some(signal);
        }
        
        // If no exact match, try partial match
        for (key, filepath) in reviewed_function.function_map.iter() {
            if key.ends_with(&format!(":{}", next_function)) {
                trace!("Found matching key: {}", key);
                let mut updated_map = reviewed_function.function_map.clone();
                updated_map.remove(key);
                
                let signal = LoopSignal {
                    key: next_function.to_string(),
                    filepath: filepath.clone(),
                    remaining_functions: updated_map,
                };
                trace!("Created LoopSignal: {:?}", signal);
                return Some(signal);
            }
        }
        
        error!("Next function '{}' not found in remaining functions map", next_function);
        println!("Next function '{}' not found in remaining functions map", next_function);
    }
    
    // Return empty signal if we're done or if next function wasn't found
    let signal = LoopSignal {
        key: String::from(""),
        filepath: String::from(""),
        remaining_functions: HashMap::new(),
    };
    println!("Returning LoopSignal with no remaining functions");
    Some(signal)
}

pub async fn run(context: SteadyContext
        ,reviewed_rx: SteadyRx<ReviewedFunction>
        ,loop_feedback_tx: SteadyTx<LoopSignal>
        ,archived_tx: SteadyTx<ArchivedFunction>, state: SteadyState<ArchiveInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [reviewed_rx],[loop_feedback_tx,archived_tx]);
  internal_behavior(cmd,reviewed_rx, loop_feedback_tx, archived_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,reviewed_rx: SteadyRx<ReviewedFunction>,loop_feedback_tx: SteadyTx<LoopSignal>,archived_tx: SteadyTx<ArchivedFunction>, state: SteadyState<ArchiveInternalState>
 ) -> Result<(),Box<dyn Error>> {
    trace!("Archive is fired up. ");

    let mut state_guard = steady_state(&state, || ArchiveInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut reviewed_rx = reviewed_rx.lock().await;
   let mut loop_feedback_tx = loop_feedback_tx.lock().await;
   let mut archived_tx = archived_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||reviewed_rx.is_closed_and_empty() && loop_feedback_tx.mark_closed() && archived_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut reviewed_rx,1)    );


         match cmd.try_take(&mut reviewed_rx) {
            Some(mut reviewed) => {
                trace!("RECIEVED FROM REVIEWER");
                // Process the review to find the next function
                if let Some(loop_signal) = process_review_and_update_map(&mut reviewed) {
                    // Send the next function information immediately
                    match cmd.try_send(&mut loop_feedback_tx, loop_signal) {
                        Ok(()) => {
                            trace!("Successfully sent next function signal. FROM ARCHIVE ACTOR TO FUNCTION SCRAPER");
                            trace!("SENT loop_signal TO SCRAPER")
                        },
                        Err(e) => {
                            error!("Failed to send loop signal: {:?}", e);
                        }
                    }
                } else {
                    trace!("No next function to process");
                    if cmd.wait_shutdown().await
                    {
                        trace!("Code is done");
                    }
                }

                // Write the current review to file
                if let Err(e) = write_review_to_file(&reviewed.review_message).await {
                    error!("Failed to write review to file: {:?}", e);
                }

                // Archive the current function
                let archived = ArchivedFunction {
                    name: reviewed.name,
                    filepath: reviewed.filepath,
                    start_line: reviewed.start_line,
                    end_line: reviewed.end_line,
                    review_message: reviewed.review_message,
                };

                let check_flag = archived.clone();

                match cmd.try_send(&mut archived_tx, archived) {
                    Ok(()) => {
                        trace!("Successfully archived function");
                        trace!("{:#?}", check_flag);
                    },
                    Err(e) => {
                        error!("Failed to send archived function: {:?}", e);
                    }
                }
            }
            None => {
                if clean {
                    trace!("No more reviews to process");
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
       let (test_reviewed_tx,reviewed_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (loop_feedback_tx,test_loop_feedback_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (archived_tx,test_archived_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, reviewed_rx.clone(), loop_feedback_tx.clone(), archived_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_reviewed_tx.testing_send_all(vec![ReviewedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_loop_feedback_rx.testing_avail_units().await, 1); // check expected count
       let _results_loop_feedback_vec = test_loop_feedback_rx.testing_take().await;
        
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_archived_rx.testing_avail_units().await, 1); // check expected count
       let _results_archived_vec = test_archived_rx.testing_take().await;
        }
}