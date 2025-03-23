#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::ArchivedFunction;

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;


//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionstorerInternalState {
}



// async fn store_function(function: &ArchivedFunction) -> Result<(), Box<dyn Error>> {
//     let output_path = "stored_functions.txt";
    
//     // Open file in append mode, create if doesn't exist
//     let mut file = OpenOptions::new()
//         .create(true)
//         .append(true)
//         .open(output_path)?;

//     //Format: {"function_name:namespace", "filepath", start_line, end_line}
//     let entry = format!(
//         "{{\"{}:{}\", \"{}\", {}, {}}}\n",
//         function.name,
//         function.namespace,
//         function.filepath,
//         function.start_line,
//         function.end_line
//     );

//     file.write_all(entry.as_bytes())?;
    
//     Ok(())
// }

pub fn generate_markdown(archived_fn: &ArchivedFunction) -> String {
    // Parse the review message
    let review_msg = archived_fn.review_message.trim_matches('{').trim_matches('}');
    
    // Find the first part (function name) and severity
    let mut parts = review_msg.splitn(3, ", ");
    let function_name = parts.next().unwrap_or("Unknown");
    let severity = parts.next().unwrap_or("Unknown");
    
    // Get the rest of the message up to the last ", 1," or ", 0,"
    let remaining = parts.next().unwrap_or("");
    let review_text = if let Some(pos) = remaining.rfind(", 1,") {
        &remaining[..pos]
    } else if let Some(pos) = remaining.rfind(", 0,") {
        &remaining[..pos]
    } else {
        remaining
    };

    // Determine color based on severity
    let severity_color = match severity.trim() {
        "1" => "<span style=\"color:green;\">Low Severity</span>",
        "2" => "<span style=\"color:yellow;\">Medium Severity</span>",
        "3" => "<span style=\"color:red;\">High Severity</span>",
        _ => "<span style=\"color:gray;\">Unknown Severity</span>",
    };

    // Extract function name from composite key if present
    let display_name = if archived_fn.name.contains(':') {
        archived_fn.name.split(':').nth(1).unwrap_or(&archived_fn.name)
    } else {
        &archived_fn.name
    };

    // Return the formatted markdown string
    format!(
        "## Function: `{}`\n\n\
        | **Aspect**        | **Details** |\n\
        |-------------------|------------|\n\
        | **Severity**      | {} |\n\
        | **Description**   | {} |\n\
        | **File Location** | {} (Lines {}-{}) |\n\
        | **Namespace**     | {} |\n",
        display_name,
        severity_color,
        review_text.trim(),
        archived_fn.filepath,
        archived_fn.start_line,
        archived_fn.end_line,
        archived_fn.namespace
    )
}


async fn store_function(archived_fn: &ArchivedFunction) -> io::Result<()> {
    let markdown = generate_markdown(archived_fn);

    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)  // Create the file if it doesn't exist
        .append(true)  // Open the file in append mode
        .open("reviewed_information.md")?;

    file.write_all(markdown.as_bytes())?;
    file.write_all(b"\n")?;  // Ensure a newline is added after each entry.

    Ok(())
}



#[cfg(not(test))]
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

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionstorerInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut archived_rx = archived_rx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||archived_rx.is_closed_and_empty()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut archived_rx,1)    );

  
          //TODO:  here is an example reading from archived_rx
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
pub async fn run(context: SteadyContext
        ,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
    ) -> Result<(),Box<dyn Error>> {
    let mut cmd =  into_monitor!(context, [archived_rx],[]);
    if let Some(responder) = cmd.sidechannel_responder() {
         let mut archived_rx = archived_rx.lock().await;
         while cmd.is_running(&mut ||
             archived_rx.is_closed_and_empty()) {
                // in main use graph.sidechannel_director node_call(msg,"FunctionStorer")
                let _did_check = responder.equals_responder(&mut cmd,&mut archived_rx).await;
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