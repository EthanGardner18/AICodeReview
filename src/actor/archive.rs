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
    pub namespace: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub review_message: String,
}

#[derive(Debug)]
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
    let file_path = "reviewed_information.md";
    
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
    
    // Extract the parts using a more robust approach
    let review_msg = reviewed_function.review_message.trim_matches('{').trim_matches('}');
    
    // Find the indices of the last three commas
    let mut comma_indices: Vec<_> = review_msg.match_indices(',').map(|(i, _)| i).collect();
    if comma_indices.len() < 3 {
        println!("Not enough commas found in review message");
        return None;
    }
    
    // Get the last three comma positions
    let last_three_commas = &comma_indices[comma_indices.len()-3..];
    
    // Extract the relevant parts
    let continue_flag = review_msg[last_three_commas[0]+1..last_three_commas[1]].trim();
    let next_function = review_msg[last_three_commas[1]+1..last_three_commas[2]].trim();
    
    println!("Continue flag: {}", continue_flag);
    println!("Next function: {}", next_function);
    
    let should_continue = continue_flag == "1";
    if should_continue {
        println!("Available functions in map: {:?}", reviewed_function.function_map.keys());
        
        // Find the full key (Class:function) that ends with the next_function_name
        for (key, filepath) in reviewed_function.function_map.iter() {
            println!("Checking key: {}", key);
            // Split the key to get the class and function parts
            let key_parts: Vec<&str> = key.split(':').collect();
            if key_parts.len() == 2 && key_parts[1] == next_function {
                println!("Found matching key: {}", key);
                // Clone the HashMap and remove the found function
                let mut updated_map = reviewed_function.function_map.clone();
                updated_map.remove(key);
                
                trace!("Found next function: {} at {}", key, filepath);
                println!("Found next function: {} at {}", key, filepath);
                
                // Create the LoopSignal with the necessary information
                let signal = LoopSignal {
                    key: key.to_string(),
                    filepath: filepath.clone(),
                    remaining_functions: updated_map,
                };
                println!("Created LoopSignal: {:#?}", signal);
                return Some(signal);
            }
        }
        
        error!("Next function '{}' not found in remaining functions map", next_function);
        println!("Next function '{}' not found in remaining functions map", next_function);
    } else {
        trace!("Review process complete (flag = 0)");
        println!("Review process complete (flag = 0)");
    }
    
    println!("Returning None from process_review_and_update_map");
    None
}

// pub async fn process_reviewed_function(mut reviewed_function: ReviewedFunction) -> Result<(), Box<dyn Error>> {
//     // Process the current review (store it, log it, etc.)
    
//     // Get the next function to review
//     match process_review_and_update_map(&mut reviewed_function) {
//         Some(LoopSignal { key, filepath, remaining_functions }) => {
//             trace!("Next function to review: {} at {}", key, filepath);
//             // Your code to send the LoopSignal to the next actor...
//         }
//         None => {
//             error!("Failed to process review message");
//         }
//     }
    
//     Ok(())
// }




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

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    reviewed_rx: SteadyRx<ReviewedFunction>,
    loop_feedback_tx: SteadyTx<LoopSignal>,
    archived_tx: SteadyTx<ArchivedFunction>, 
    state: SteadyState<ArchiveInternalState>
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || ArchiveInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {
        let mut reviewed_rx = reviewed_rx.lock().await;
        let mut loop_feedback_tx = loop_feedback_tx.lock().await;
        let mut archived_tx = archived_tx.lock().await;

        while cmd.is_running(&mut ||reviewed_rx.is_closed_and_empty() && loop_feedback_tx.mark_closed() && archived_tx.mark_closed()) {
            let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut reviewed_rx,1));

            match cmd.try_take(&mut reviewed_rx) {
                Some(mut reviewed) => {
                    println!("RECIEVED FROM REVIEWER");
                    // Process the review to find the next function
                    if let Some(loop_signal) = process_review_and_update_map(&mut reviewed) {
                        // Send the next function information immediately
                        println!("archive - scraper \n{:#?}", &loop_signal);
                        match cmd.try_send(&mut loop_feedback_tx, loop_signal) {
                            Ok(()) => {
                                trace!("Successfully sent next function signal");
                                println!("SENT loop_signal TO SCRAPER")
                            },
                            Err(e) => {
                                error!("Failed to send loop signal: {:?}", e);
                            }
                        }
                    } else {
                        trace!("No next function to process");
                        cmd.request_graph_stop();
                    }

                    // Write the current review to file
                    if let Err(e) = write_review_to_file(&reviewed.review_message).await {
                        error!("Failed to write review to file: {:?}", e);
                    }

                    // Archive the current function
                    let archived = ArchivedFunction {
                        name: reviewed.name,
                        namespace: reviewed.namespace,
                        filepath: reviewed.filepath,
                        start_line: reviewed.start_line,
                        end_line: reviewed.end_line,
                        review_message: reviewed.review_message,
                    };

                    match cmd.try_send(&mut archived_tx, archived) {
                        Ok(()) => {
                            trace!("Successfully archived function");
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
       let results_loop_feedback_vec = test_loop_feedback_rx.testing_take().await;
        
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_archived_rx.testing_avail_units().await, 1); // check expected count
       let results_archived_vec = test_archived_rx.testing_take().await;
        }
}