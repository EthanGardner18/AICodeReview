
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct FileMetadata {
  /// The initial directories (or other info) you may want to pass.
    /// List of files found that match the desired extensions.
    pub found_files: Vec<PathBuf>,
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ReaddirectoryInternalState {
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,file_list_tx: SteadyTx<FileMetadata>, state: SteadyState<ReaddirectoryInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [],[file_list_tx]);
  internal_behavior(cmd, file_list_tx, state).await
}

fn scan_directory_for_files(path: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found_files = Vec::new();
    if path.is_dir() {
        // Read the directory entries
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    // Recursively scan the subdirectory
                    found_files.extend(scan_directory_for_files(&entry_path, extensions));
                } else if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    // Check if the file extension matches one of the desired ones
                    if extensions.contains(&ext) {
                        found_files.push(entry_path);
                    }
                }
            }
        } else {
            println!("Failed to read directory '{}'", path.display());
        }
    }
    found_files
}



async fn internal_behavior<C: SteadyCommander>(mut cmd: C,file_list_tx: SteadyTx<FileMetadata>, state: SteadyState<ReaddirectoryInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || ReaddirectoryInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut file_list_tx = file_list_tx.lock().await;
   
    let input = "/home/rpoudel/SeniorDesign/test";

    println!("The inputted directory is {}", input);

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
           if input.eq_ignore_ascii_case("exit") {
            println!("Exiting program.");
        } else {
            // Define the coding file extensions to filter for.
            let coding_extensions = [
                "py", "cpp", "h", "hpp", "cc", "cxx", "rs", "c", "js", "jsx", "ts", "tsx", "java",
                "go", "html", "htm", "css", "sh", "php", "rb", "kt", "kts", "swift", "pl", "pm",
                "r", "md",
            ];

            // Convert the input into a PathBuf and store it (if you want to keep track of the initial directory).
            let input_dir = PathBuf::from(input);


            // Call our helper function to scan the directory for files matching the extensions.
            let found_files = scan_directory_for_files(&input_dir, &coding_extensions);
            
            // Iterate over found files and send them through the channel
            let file_metadata = FileMetadata { found_files };
            
            match cmd.try_send(&mut file_list_tx, file_metadata) {
                    Ok(()) => {
                        info!("File metadata sent successfully");
                    }
                    Err(msg) => {
                        trace!("Error sending file metadata: {:?}", msg);
                    }
                }
        }
   
   
   
   
 
    }
    Ok(())
}


#[cfg(test)]
pub async fn run(context: SteadyContext
        ,file_list_tx: SteadyTx<FileMetadata>, state: SteadyState<ReaddirectoryInternalState>
    ) -> Result<(),Box<dyn Error>> {
    let mut cmd =  into_monitor!(context, [],[file_list_tx]);
    if let Some(responder) = cmd.sidechannel_responder() {
         let mut file_list_tx = file_list_tx.lock().await;
         while cmd.is_running(&mut ||file_list_tx.mark_closed()) {
                 // in main use graph.sidechannel_director node_call(msg,"ReadDirectory")
                 let _did_echo = responder.echo_responder(&mut cmd,&mut file_list_tx).await;
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
       let (file_list_tx,test_file_list_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, file_list_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_file_list_rx.testing_avail_units().await, 1); // check expected count
       let results_file_list_vec = test_file_list_rx.testing_take().await;
        }
}
