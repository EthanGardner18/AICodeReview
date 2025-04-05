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

fn process_review_and_update_map(reviewed_function: &mut ReviewedFunction) -> Option<LoopSignal> {
    println!("Starting process_review_and_update_map");
    println!("Review message: {}", reviewed_function.review_message);
    
    // Extract the parts using double backtick separation and remove curly braces
    let review_msg = reviewed_function.review_message
        .trim_matches('{')
        .trim_matches('}')
        .trim();
        
    let parts: Vec<&str> = review_msg.split("``").collect();
    
    // Check if we have all required parts
    if parts.len() < 6 {
        println!("Not enough parts in review message. Expected 6, got {}", parts.len());
        println!("Parts: {:?}", parts);
        return None;
    }
    
    // Extract components using array indexing
    let current_function = parts[0].trim();    // processFile
    let severity = parts[1].trim();            // 1
    let review_text = parts[2].trim();         // This function handles...
    let continue_flag = parts[3].trim();       // 1
    let next_function = parts[4].trim();       // validateInput
    let next_filepath = parts[5].trim();       // src/utils.rs
    
    println!("Parsed components:");
    println!("Current function: {}", current_function);
    println!("Severity: {}", severity);
    println!("Review: {}", review_text);
    println!("Continue flag: {}", continue_flag);
    println!("Next function: {}", next_function);
    println!("Next filepath: {}", next_filepath);
    
    // Clean up the continue flag - ensure we get just the number
    let should_continue = continue_flag == "1";
    
    if should_continue {
        println!("Available functions in map: {:?}", reviewed_function.function_map.keys());
        
        // Create the composite key using the filepath and function name
        let composite_key = format!("{}:{}", next_filepath, next_function);
        println!("Looking for composite key: {}", composite_key);
        
        // Try exact match with composite key
        if reviewed_function.function_map.contains_key(&composite_key) {
            let filepath = reviewed_function.function_map.get(&composite_key).unwrap();
            println!("Found exact matching composite key: {}", composite_key);
            
            // Clone the HashMap and remove the found function
            let mut updated_map = reviewed_function.function_map.clone();
            updated_map.remove(&composite_key);
            
            let signal = LoopSignal {
                key: next_function.to_string(),
                filepath: filepath.clone(),
                remaining_functions: updated_map,
            };
            println!("Created LoopSignal: {:?}", signal);
            return Some(signal);
        }
        
        // If no exact match, try partial match
        for (key, filepath) in reviewed_function.function_map.iter() {
            if key.ends_with(&format!(":{}", next_function)) {
                println!("Found matching key: {}", key);
                let mut updated_map = reviewed_function.function_map.clone();
                updated_map.remove(key);
                
                let signal = LoopSignal {
                    key: next_function.to_string(),
                    filepath: filepath.clone(),
                    remaining_functions: updated_map,
                };
                println!("Created LoopSignal: {:?}", signal);
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
    println!("Archive is fired up. ");

    let mut state_guard = steady_state(&state, || ArchiveInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

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
                println!("RECIEVED FROM REVIEWER");
                // Process the review to find the next function
                if let Some(loop_signal) = process_review_and_update_map(&mut reviewed) {
                    // Send the next function information immediately
                    println!("archive - scraper \n{:#?}", &loop_signal);
                    match cmd.try_send(&mut loop_feedback_tx, loop_signal) {
                        Ok(()) => {
                            println!("Successfully sent next function signal. FROM ARCHIVE ACTOR TO FUNCTION SCRAPER");
                            println!("SENT loop_signal TO SCRAPER")
                        },
                        Err(e) => {
                            error!("Failed to send loop signal: {:?}", e);
                        }
                    }
                } else {
                    trace!("No next function to process");
                    if cmd.wait_shutdown().await
                    {
                        print!("Code is done");
                    }
                    print!("outside if");
                    // cmd.request_graph_stop();

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
                        print!("{:#?}", check_flag);
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

  
        //   //TODO:  here is an example reading from reviewed_rx
        //   match cmd.try_take(&mut reviewed_rx) {
        //       Some(rec) => {
        //           trace!("got rec: {:?}", rec);
        //       }
        //       None => {
        //           if clean {
        //              //this could be an error if we expected a value
        //           }
        //       }
        //   }
  
  
        // //TODO:  here is an example writing to loop_feedback_tx
        // match cmd.try_send(&mut loop_feedback_tx, LoopSignal::default() ) {
        //     Ok(()) => {
        //     },
        //     Err(msg) => { //in the above await we should have confirmed space is available
        //         trace!("error sending: {:?}", msg)
        //     },
        // }
  
  
        // //TODO:  here is an example writing to archived_tx
        // match cmd.try_send(&mut archived_tx, ArchivedFunction::default() ) {
        //     Ok(()) => {
        //     },
        //     Err(msg) => { //in the above await we should have confirmed space is available
        //         trace!("error sending: {:?}", msg)
        //     },
        // }
  

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
       let results_loop_feedback_vec = test_loop_feedback_rx.testing_take().await;
        
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_archived_rx.testing_avail_units().await, 1); // check expected count
       let results_archived_vec = test_archived_rx.testing_take().await;
        }
}