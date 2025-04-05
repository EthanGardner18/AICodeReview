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
use std::path::{Path, PathBuf};
// use std::time::SystemTime;
use chrono::{DateTime, Local};
use std::fs;


//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionstorerInternalState {
}


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


async fn store_function(archived_fn: &ArchivedFunction) -> io::Result<()> {
    let markdown = generate_markdown(archived_fn);

    // Hard-coded base directory path for Linux systems
    let base_dir = "/home/glassfrog";  // Change "user" to the actual username
    let review_base_dir = PathBuf::from(base_dir).join("review_output");
    
    // Create the base review output directory
    fs::create_dir_all(&review_base_dir)?;

    // Get the original file path and convert it to a PathBuf
    let original_path = PathBuf::from(&archived_fn.filepath);

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
    if let Some(mut state) = state_guard.as_mut() {
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


// #[cfg(test)]
// pub async fn run(context: SteadyContext
//         ,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
//     ) -> Result<(),Box<dyn Error>> {
//     let mut cmd =  into_monitor!(context, [archived_rx],[]);
//     if let Some(responder) = cmd.sidechannel_responder() {
//          let mut archived_rx = archived_rx.lock().await;
//          while cmd.is_running(&mut ||
//              archived_rx.is_closed_and_empty()) {
//                 // in main use graph.sidechannel_director node_call(msg,"FunctionStorer")
//                 let _did_check = responder.equals_responder(&mut cmd,&mut archived_rx).await;
//          }
//     }
//     Ok(())
// }

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